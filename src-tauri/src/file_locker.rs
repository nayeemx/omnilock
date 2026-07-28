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
            let mut count: u32 = 0;
            let mut ea_ptr: *mut EXPLICIT_ACCESS_W = std::ptr::null_mut();
            let ret = GetExplicitEntriesFromAclW(dacl, &mut count, &mut ea_ptr);
            
            if ret == 0 && !ea_ptr.is_null() && count > 0 {
                let mut new_eas: Vec<EXPLICIT_ACCESS_W> = Vec::new();
                
                for i in 0..count {
                    let entry = &*ea_ptr.add(i as usize);
                    let is_deny = entry.grfAccessMode == DENY_ACCESS;
                    let is_everyone = if !entry.Trustee.ptstrName.is_null() && entry.Trustee.TrusteeForm == TRUSTEE_IS_SID {
                        let sid_str = to_wide("S-1-1-0");
                        let mut test_sid: PSID = std::ptr::null_mut();
                        if ConvertStringSidToSidW(sid_str.as_ptr(), &mut test_sid) != 0 {
                            let result = EqualSid(entry.Trustee.ptstrName as PSID, test_sid);
                            LocalFree(test_sid as *mut _);
                            result != 0
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    
                    if !(is_deny && is_everyone) {
                        new_eas.push(*entry);
                    }
                }
                
                LocalFree(ea_ptr as *mut _);
                
                if new_eas.is_empty() {
                    let ret = SetNamedSecurityInfoW(
                        path_wide.as_ptr(),
                        SE_FILE_OBJECT,
                        DACL_SECURITY_INFORMATION,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    );
                    LocalFree(sd);
                    LocalFree(everyone_sid as *mut _);
                    if ret != 0 {
                        return Err(format!("SetNamedSecurityInfo (clear DACL) failed: {}", ret));
                    }
                    return Ok(());
                }
                
                let mut new_dacl: *mut ACL = std::ptr::null_mut();
                let ret = SetEntriesInAclW(new_eas.len() as u32, new_eas.as_mut_ptr(), std::ptr::null_mut(), &mut new_dacl);
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
            } else {
                LocalFree(ea_ptr as *mut _);
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

pub fn verify_lock(path: &str) -> Result<bool, String> {
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
        Ok(has_deny)
    }
}

pub fn unlock_folder(path: &str) -> Result<(), String> {
    let dir_path = std::path::Path::new(path);
    if !dir_path.exists() || !dir_path.is_dir() {
        return Err(format!("Folder does not exist: {}", path));
    }
    remove_deny_acl(path)
}
