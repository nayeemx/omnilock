use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Security::*;
use windows_sys::Win32::Security::Authorization::*;
use windows_sys::Win32::System::Threading::*;

fn to_wide(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

unsafe fn enable_privilege(name: &str) -> Result<(), String> {
    let mut h_token = std::ptr::null_mut();
    if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut h_token) == 0 {
        return Err("OpenProcessToken failed".to_string());
    }
    let name_w = to_wide(name);
    let mut luid: LUID = std::mem::zeroed();
    if LookupPrivilegeValueW(std::ptr::null_mut(), name_w.as_ptr(), &mut luid) == 0 {
        CloseHandle(h_token);
        return Err(format!("LookupPrivilegeValue({}) failed", name));
    }
    let mut tp = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES {
            Luid: luid,
            Attributes: SE_PRIVILEGE_ENABLED,
        }],
    };
    let ret = AdjustTokenPrivileges(h_token, 0, &mut tp, std::mem::size_of::<TOKEN_PRIVILEGES>() as u32, std::ptr::null_mut(), std::ptr::null_mut());
    let gle = GetLastError();
    if ret == 0 || gle != 0 {
        CloseHandle(h_token);
        return Err(format!("AdjustTokenPrivileges({}) failed: err={}", name, gle));
    }
    CloseHandle(h_token);
    Ok(())
}

unsafe fn current_user_sid_buf() -> Result<Vec<u8>, String> {
    let mut h_token = std::ptr::null_mut();
    if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut h_token) == 0 {
        return Err("OpenProcessToken failed".to_string());
    }
    let mut len: u32 = 0;
    GetTokenInformation(h_token, TokenUser, std::ptr::null_mut(), 0, &mut len);
    let mut buf = vec![0u8; len as usize];
    if GetTokenInformation(h_token, TokenUser, buf.as_mut_ptr() as _, len, &mut len) == 0 {
        CloseHandle(h_token);
        return Err("GetTokenInformation failed".to_string());
    }
    CloseHandle(h_token);
    Ok(buf)
}

fn is_access_denied(ret: u32) -> bool {
    ret == ERROR_ACCESS_DENIED
}

unsafe fn make_safe_dacl() -> Result<(*mut ACL, Vec<u8>), String> {
    let buf = current_user_sid_buf()?;
    let token_user = &*(buf.as_ptr() as *const TOKEN_USER);

    let mut admin_sid: PSID = std::ptr::null_mut();
    let admin_sid_str = to_wide("S-1-5-32-544");
    ConvertStringSidToSidW(admin_sid_str.as_ptr(), &mut admin_sid);

    let mut system_sid: PSID = std::ptr::null_mut();
    let system_sid_str = to_wide("S-1-5-18");
    ConvertStringSidToSidW(system_sid_str.as_ptr(), &mut system_sid);

    let mut entries: Vec<EXPLICIT_ACCESS_W> = Vec::new();

    let mut ea_user: EXPLICIT_ACCESS_W = std::mem::zeroed();
    ea_user.grfAccessPermissions = GENERIC_ALL;
    ea_user.grfAccessMode = GRANT_ACCESS;
    ea_user.grfInheritance = SUB_CONTAINERS_AND_OBJECTS_INHERIT;
    ea_user.Trustee.TrusteeForm = TRUSTEE_IS_SID;
    ea_user.Trustee.TrusteeType = TRUSTEE_IS_USER;
    ea_user.Trustee.ptstrName = token_user.User.Sid as *mut u16;
    entries.push(ea_user);

    if !admin_sid.is_null() {
        let mut ea_admin: EXPLICIT_ACCESS_W = std::mem::zeroed();
        ea_admin.grfAccessPermissions = GENERIC_ALL;
        ea_admin.grfAccessMode = GRANT_ACCESS;
        ea_admin.grfInheritance = SUB_CONTAINERS_AND_OBJECTS_INHERIT;
        ea_admin.Trustee.TrusteeForm = TRUSTEE_IS_SID;
        ea_admin.Trustee.TrusteeType = TRUSTEE_IS_GROUP;
        ea_admin.Trustee.ptstrName = admin_sid as *mut u16;
        entries.push(ea_admin);
    }

    if !system_sid.is_null() {
        let mut ea_system: EXPLICIT_ACCESS_W = std::mem::zeroed();
        ea_system.grfAccessPermissions = GENERIC_ALL;
        ea_system.grfAccessMode = GRANT_ACCESS;
        ea_system.grfInheritance = SUB_CONTAINERS_AND_OBJECTS_INHERIT;
        ea_system.Trustee.TrusteeForm = TRUSTEE_IS_SID;
        ea_system.Trustee.TrusteeType = TRUSTEE_IS_WELL_KNOWN_GROUP;
        ea_system.Trustee.ptstrName = system_sid as *mut u16;
        entries.push(ea_system);
    }

    let mut new_dacl: *mut ACL = std::ptr::null_mut();
    let ret = SetEntriesInAclW(entries.len() as u32, entries.as_mut_ptr(), std::ptr::null_mut(), &mut new_dacl);

    if !admin_sid.is_null() { LocalFree(admin_sid as _); }
    if !system_sid.is_null() { LocalFree(system_sid as _); }

    if ret != 0 {
        return Err(format!("SetEntriesInAclW failed: {}", ret));
    }

    Ok((new_dacl, buf))
}

pub fn apply_lock(path: &str) -> Result<(), String> {
    if !std::path::Path::new(path).exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    unsafe {
        let path_wide = to_wide(path);
        let _ = enable_privilege("SeTakeOwnershipPrivilege");

        // Set owner to SYSTEM
        let mut system_sid: PSID = std::ptr::null_mut();
        let system_sid_str = to_wide("S-1-5-18");
        if ConvertStringSidToSidW(system_sid_str.as_ptr(), &mut system_sid) == 0 {
            return Err(format!("ConvertStringSidToSid(SYSTEM) failed"));
        }

        let ret = SetNamedSecurityInfoW(
            path_wide.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            system_sid,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        LocalFree(system_sid as _);

        if ret != 0 && !is_access_denied(ret) {
            return Err(format!("SetNamedSecurityInfo(OWNER=SYSTEM) failed: err={}", ret));
        }

        // Replace DACL: only Administrators + SYSTEM
        let mut admin_sid: PSID = std::ptr::null_mut();
        let admin_sid_str = to_wide("S-1-5-32-544");
        ConvertStringSidToSidW(admin_sid_str.as_ptr(), &mut admin_sid);

        let mut sys_sid2: PSID = std::ptr::null_mut();
        let sys_sid2_str = to_wide("S-1-5-18");
        ConvertStringSidToSidW(sys_sid2_str.as_ptr(), &mut sys_sid2);

        let mut entries: Vec<EXPLICIT_ACCESS_W> = Vec::new();

        if !admin_sid.is_null() {
            let mut ea: EXPLICIT_ACCESS_W = std::mem::zeroed();
            ea.grfAccessPermissions = GENERIC_ALL;
            ea.grfAccessMode = GRANT_ACCESS;
            ea.grfInheritance = SUB_CONTAINERS_AND_OBJECTS_INHERIT;
            ea.Trustee.TrusteeForm = TRUSTEE_IS_SID;
            ea.Trustee.TrusteeType = TRUSTEE_IS_GROUP;
            ea.Trustee.ptstrName = admin_sid as *mut u16;
            entries.push(ea);
        }

        if !sys_sid2.is_null() {
            let mut ea: EXPLICIT_ACCESS_W = std::mem::zeroed();
            ea.grfAccessPermissions = GENERIC_ALL;
            ea.grfAccessMode = GRANT_ACCESS;
            ea.grfInheritance = SUB_CONTAINERS_AND_OBJECTS_INHERIT;
            ea.Trustee.TrusteeForm = TRUSTEE_IS_SID;
            ea.Trustee.TrusteeType = TRUSTEE_IS_WELL_KNOWN_GROUP;
            ea.Trustee.ptstrName = sys_sid2 as *mut u16;
            entries.push(ea);
        }

        let mut new_dacl: *mut ACL = std::ptr::null_mut();
        let ret = SetEntriesInAclW(entries.len() as u32, entries.as_mut_ptr(), std::ptr::null_mut(), &mut new_dacl);

        if !admin_sid.is_null() { LocalFree(admin_sid as _); }
        if !sys_sid2.is_null() { LocalFree(sys_sid2 as _); }
        if ret != 0 {
            return Err(format!("SetEntriesInAclW failed: {}", ret));
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

        if ret != 0 {
            return Err(format!("SetNamedSecurityInfo(DACL) failed: err={}", ret));
        }
        Ok(())
    }
}

pub fn remove_lock(path: &str) -> Result<(), String> {
    if !std::path::Path::new(path).exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    let p = std::path::Path::new(path);

    unsafe {
        let path_wide = to_wide(path);
        let _ = enable_privilege("SeTakeOwnershipPrivilege");

        let (new_dacl, buf) = make_safe_dacl()?;
        let token_user = &*(buf.as_ptr() as *const TOKEN_USER);

        // Step 1: restore DACL first
        let ret1 = SetNamedSecurityInfoW(
            path_wide.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            new_dacl,
            std::ptr::null_mut(),
        );

        if ret1 != 0 {
            LocalFree(new_dacl as *mut _);
            return Err(format!("SetNamedSecurityInfo(DACL) failed: err={}", ret1));
        }

        // Step 2: restore owner to current user
        let ret2 = SetNamedSecurityInfoW(
            path_wide.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            token_user.User.Sid,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );

        LocalFree(new_dacl as *mut _);

        if ret2 != 0 {
            return Err(format!("SetNamedSecurityInfo(OWNER) failed: err={}", ret2));
        }
    }

    // Recursively reset ACLs on child files that inherited the restricted DACL
    if p.is_dir() {
        remove_children_recursive(p).ok();
    }

    Ok(())
}

fn remove_children_recursive(dir: &std::path::Path) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }
    let entries = std::fs::read_dir(dir).map_err(|e| format!("Cannot read directory: {}", e))?;
    for entry in entries.flatten() {
        let child = entry.path();
        if child.is_dir() {
            remove_children_recursive(&child)?;
        } else if child.is_file() {
            if verify_lock(&child.to_string_lossy()) {
                // Reuse top-level logic for each child file
                let child_str = child.to_string_lossy();
                let _ = remove_lock(&child_str);
            }
        }
    }
    Ok(())
}

pub fn verify_lock(path: &str) -> bool {
    unsafe {
        let path_wide = to_wide(path);

        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        let ret = GetNamedSecurityInfoW(
            path_wide.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut sd,
        );

        if is_access_denied(ret) {
            return true;
        }
        if ret != 0 {
            return false;
        }

        let mut owner_sid: PSID = std::ptr::null_mut();
        let mut owner_defaulted: i32 = 0;
        GetSecurityDescriptorOwner(sd, &mut owner_sid, &mut owner_defaulted);

        let mut is_locked = false;
        if !owner_sid.is_null() {
            let system_sid_str = to_wide("S-1-5-18");
            let mut system_sid: PSID = std::ptr::null_mut();
            if ConvertStringSidToSidW(system_sid_str.as_ptr(), &mut system_sid) != 0 {
                is_locked = EqualSid(owner_sid, system_sid) != 0;
                LocalFree(system_sid as _);
            }
        }

        LocalFree(sd);
        is_locked
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
