use std::path::Path;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Security::*;
use windows_sys::Win32::Security::Authorization::*;

fn to_wide(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn apply_deny_acl(path: &str) -> Result<(), String> {
    if !Path::new(path).exists() {
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

fn remove_deny_acl(path: &str) -> Result<(), String> {
    if !Path::new(path).exists() {
        return Err(format!("Path does not exist: {}", path));
    }

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
            return Err(format!("GetNamedSecurityInfo failed: {}", ret));
        }

        let mut dacl_present: i32 = 0;
        let mut dacl: *mut ACL = std::ptr::null_mut();
        let mut dacl_defaulted: i32 = 0;
        GetSecurityDescriptorDacl(sd, &mut dacl_present, &mut dacl, &mut dacl_defaulted);

        let mut everyone_sid: PSID = std::ptr::null_mut();
        let everyone_sid_str = to_wide("S-1-1-0");
        if ConvertStringSidToSidW(everyone_sid_str.as_ptr(), &mut everyone_sid) == 0 {
            LocalFree(sd);
            return Err(format!("ConvertStringSidToSid failed: {}", std::io::Error::last_os_error()));
        }

        if dacl_present != 0 && !dacl.is_null() {
            let mut remove_ea: EXPLICIT_ACCESS_W = std::mem::zeroed();
            remove_ea.grfAccessPermissions = GENERIC_ALL;
            remove_ea.grfAccessMode = REVOKE_ACCESS;
            remove_ea.grfInheritance = SUB_CONTAINERS_AND_OBJECTS_INHERIT;
            remove_ea.Trustee.TrusteeForm = TRUSTEE_IS_SID;
            remove_ea.Trustee.TrusteeType = TRUSTEE_IS_WELL_KNOWN_GROUP;
            remove_ea.Trustee.ptstrName = everyone_sid as *mut u16;

            let mut new_dacl: *mut ACL = std::ptr::null_mut();
            let ret = SetEntriesInAclW(1, &mut remove_ea, dacl, &mut new_dacl);
            if ret == 0 {
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
                if ret != 0 {
                    LocalFree(sd);
                    LocalFree(everyone_sid as *mut _);
                    return Err(format!("SetNamedSecurityInfo (remove) failed: {}", ret));
                }
            } else {
                LocalFree(sd);
                LocalFree(everyone_sid as *mut _);
                return Err(format!("SetEntriesInAcl (remove) failed: {}", ret));
            }
        }

        LocalFree(sd);
        LocalFree(everyone_sid as *mut _);
        Ok(())
    }
}

pub fn lock_file(path: &str) -> Result<(), String> {
    apply_deny_acl(path)
}

pub fn unlock_file(path: &str) -> Result<(), String> {
    remove_deny_acl(path)
}

pub fn lock_folder(path: &str) -> Result<(), String> {
    let dir_path = std::path::Path::new(path);
    if !dir_path.exists() || !dir_path.is_dir() {
        return Err(format!("Folder does not exist: {}", path));
    }
    apply_deny_acl(path)
}

pub fn unlock_folder(path: &str) -> Result<(), String> {
    let dir_path = std::path::Path::new(path);
    if !dir_path.exists() || !dir_path.is_dir() {
        return Err(format!("Folder does not exist: {}", path));
    }
    remove_deny_acl(path)
}
