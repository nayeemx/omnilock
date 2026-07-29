use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Security::*;
use windows_sys::Win32::Security::Authorization::*;

fn to_wide(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

pub fn apply_lock(path: &str) -> Result<(), String> {
    if !std::path::Path::new(path).exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    unsafe {
        let mut everyone_sid: PSID = std::ptr::null_mut();
        let everyone_sid_str = to_wide("S-1-1-0");
        if ConvertStringSidToSidW(everyone_sid_str.as_ptr(), &mut everyone_sid) == 0 {
            return Err(format!("ConvertStringSidToSid failed: {}", std::io::Error::last_os_error()));
        }

        let mut ea: EXPLICIT_ACCESS_W = std::mem::zeroed();
        ea.grfAccessPermissions = GENERIC_ALL;
        ea.grfAccessMode = DENY_ACCESS;
        ea.grfInheritance = SUB_CONTAINERS_AND_OBJECTS_INHERIT;
        ea.Trustee.TrusteeForm = TRUSTEE_IS_SID;
        ea.Trustee.TrusteeType = TRUSTEE_IS_WELL_KNOWN_GROUP;
        ea.Trustee.ptstrName = everyone_sid as *mut u16;

        let mut new_dacl: *mut ACL = std::ptr::null_mut();
        let ret = SetEntriesInAclW(1, &mut ea, std::ptr::null_mut(), &mut new_dacl);
        if ret != 0 {
            LocalFree(everyone_sid as *mut _);
            return Err(format!("SetEntriesInAcl failed: {}", ret));
        }

        let path_wide = to_wide(path);
        let ret = SetNamedSecurityInfoW(
            path_wide.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            new_dacl,
            std::ptr::null_mut(),
        );

        LocalFree(new_dacl as *mut _);
        LocalFree(everyone_sid as *mut _);

        if ret != 0 {
            return Err(format!("SetNamedSecurityInfo failed: err={}", ret));
        }
        Ok(())
    }
}

pub fn remove_lock(path: &str) -> Result<(), String> {
    if !std::path::Path::new(path).exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    unsafe {
        let path_wide = to_wide(path);

        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        let ret = GetNamedSecurityInfoW(
            path_wide.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut sd,
        );
        if ret != 0 {
            return Err(format!("GetNamedSecurityInfo failed: {}", ret));
        }

        let mut dacl_present: i32 = 0;
        let mut dacl: *mut ACL = std::ptr::null_mut();
        let mut dacl_defaulted: i32 = 0;
        GetSecurityDescriptorDacl(sd, &mut dacl_present, &mut dacl, &mut dacl_defaulted);

        if dacl_present == 0 || dacl.is_null() {
            LocalFree(sd);
            return Ok(());
        }

        let mut everyone_sid: PSID = std::ptr::null_mut();
        let everyone_sid_str = to_wide("S-1-1-0");
        if ConvertStringSidToSidW(everyone_sid_str.as_ptr(), &mut everyone_sid) == 0 {
            LocalFree(sd);
            return Err(format!("ConvertStringSidToSid failed: {}", std::io::Error::last_os_error()));
        }

        let mut count: u32 = 0;
        let mut entries: *mut EXPLICIT_ACCESS_W = std::ptr::null_mut();
        let ret = GetExplicitEntriesFromAclW(dacl, &mut count, &mut entries);
        if ret != 0 {
            LocalFree(everyone_sid as *mut _);
            LocalFree(sd);
            return Err(format!("GetExplicitEntriesFromAclW failed: {}", ret));
        }

        let mut filtered: Vec<EXPLICIT_ACCESS_W> = Vec::new();
        for i in 0..count {
            let entry = &*entries.add(i as usize);
            let is_deny_everyone = entry.grfAccessMode == DENY_ACCESS
                && EqualSid(entry.Trustee.ptstrName as PSID, everyone_sid) != 0;
            if !is_deny_everyone {
                filtered.push(*entry);
            }
        }

        LocalFree(entries as *mut _);
        LocalFree(everyone_sid as *mut _);

        let mut new_dacl: *mut ACL = std::ptr::null_mut();
        let ret = SetEntriesInAclW(
            filtered.len() as u32,
            if filtered.is_empty() { std::ptr::null_mut() } else { filtered.as_mut_ptr() },
            std::ptr::null_mut(),
            &mut new_dacl,
        );
        if ret != 0 {
            LocalFree(sd);
            return Err(format!("SetEntriesInAcl failed: {}", ret));
        }

        let ret = SetNamedSecurityInfoW(
            path_wide.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            new_dacl,
            std::ptr::null_mut(),
        );

        LocalFree(new_dacl as *mut _);
        LocalFree(sd);

        if ret != 0 {
            return Err(format!("SetNamedSecurityInfo failed: err={}", ret));
        }
        Ok(())
    }
}

pub fn verify_lock(path: &str) -> bool {
    unsafe {
        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        let path_wide = to_wide(path);

        let ret = GetNamedSecurityInfoW(
            path_wide.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut sd,
        );
        if ret != 0 {
            return false;
        }

        let mut dacl_present: i32 = 0;
        let mut dacl: *mut ACL = std::ptr::null_mut();
        let mut dacl_defaulted: i32 = 0;
        GetSecurityDescriptorDacl(sd, &mut dacl_present, &mut dacl, &mut dacl_defaulted);

        let mut has_deny = false;
        if dacl_present != 0 && !dacl.is_null() {
            let mut count: u32 = 0;
            let mut ea_ptr: *mut EXPLICIT_ACCESS_W = std::ptr::null_mut();
            let ret = GetExplicitEntriesFromAclW(dacl, &mut count, &mut ea_ptr);
            if ret == 0 && !ea_ptr.is_null() {
                for i in 0..count {
                    let entry = &*ea_ptr.add(i as usize);
                    if entry.grfAccessMode == DENY_ACCESS {
                        has_deny = true;
                        break;
                    }
                }
                LocalFree(ea_ptr as *mut _);
            }
        }

        LocalFree(sd);
        has_deny
    }
}

pub fn lock_drive(drive_letter: &str) -> Result<(), String> {
    let root = format!("{}:\\", drive_letter);
    apply_lock(&root)
}

pub fn unlock_drive(drive_letter: &str) -> Result<(), String> {
    let root = format!("{}:\\", drive_letter);
    remove_lock(&root)
}
