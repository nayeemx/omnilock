use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Security::*;
use windows_sys::Win32::Security::Authorization::*;
use windows_sys::Win32::Storage::FileSystem::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::Shell::*;

fn to_wide(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn is_access_denied(ret: u32) -> bool {
    ret == ERROR_ACCESS_DENIED
}

fn current_user_sid_buf() -> Result<Vec<u8>, String> {
    unsafe {
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

fn make_safe_dacl() -> Result<(*mut ACL, Vec<u8>), String> {
    unsafe {
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
}

pub fn safe_recover_acl(path: &str) -> Result<(), String> {
    if !Path::new(path).exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    unsafe {
        let path_wide = to_wide(path);

        enable_privilege("SeTakeOwnershipPrivilege")?;

        let (new_dacl, buf) = make_safe_dacl()?;
        let token_user = &*(buf.as_ptr() as *const TOKEN_USER);

        // Set both owner and DACL in one call
        let ret = SetNamedSecurityInfoW(
            path_wide.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            token_user.User.Sid,
            std::ptr::null_mut(),
            new_dacl,
            std::ptr::null_mut(),
        );

        LocalFree(new_dacl as *mut _);

        if ret != 0 {
            return Err(format!("SetNamedSecurityInfo failed: err={}", ret));
        }
        Ok(())
    }
}

fn apply_safe_lock(path: &str) -> Result<(), String> {
    if !Path::new(path).exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    unsafe {
        let path_wide = to_wide(path);
        enable_privilege("SeTakeOwnershipPrivilege")?;

        // Set owner to SYSTEM so user loses ownership-based privileges
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

        if ret != 0 {
            return Err(format!("SetNamedSecurityInfo(OWNER=SYSTEM) failed: err={}", ret));
        }

        // Replace DACL: only Administrators + SYSTEM, current user removed
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

fn remove_safe_lock(path: &str) -> Result<(), String> {
    if !Path::new(path).exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    unsafe {
        let path_wide = to_wide(path);
        enable_privilege("SeTakeOwnershipPrivilege")?;

        let (new_dacl, buf) = make_safe_dacl()?;
        let token_user = &*(buf.as_ptr() as *const TOKEN_USER);

        // Step 1: restore DACL first (needs WRITE_DAC via Administrators group)
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

        // Step 2: restore owner to current user (needs SeTakeOwnershipPrivilege)
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
        Ok(())
    }
}

fn backup_path_for(path: &str) -> Result<PathBuf, String> {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let safe_name = path.replace(|c: char| !c.is_alphanumeric() && c != '.' && c != '_', "_");
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup_dir = PathBuf::from(appdata)
        .join("InnologyBD")
        .join("OmniLock")
        .join("backups")
        .join(&safe_name);
    fs::create_dir_all(&backup_dir).map_err(|e| format!("Cannot create backup dir: {}", e))?;
    let name = Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let ext = Path::new(path)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let stem = if name.ends_with(&ext) && ext.len() > 1 {
        &name[..name.len() - ext.len()]
    } else {
        &name
    };
    Ok(backup_dir.join(format!("{}_{}{}", stem, now, ext)))
}

fn create_backup_before_lock(path: &str) -> Result<(), String> {
    let src = Path::new(path);
    if !src.exists() {
        return Ok(());
    }
    let dst = backup_path_for(path)?;
    if src.is_dir() {
        let manifest_path = dst.with_extension("json");
        let mut entries = Vec::new();
        if let Ok(dir_entries) = fs::read_dir(src) {
            for entry in dir_entries.flatten() {
                entries.push(entry.path().to_string_lossy().to_string());
            }
        }
        let meta = serde_json::json!({
            "path": path,
            "backup_time": SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            "contents": entries,
        });
        fs::write(&manifest_path, serde_json::to_string_pretty(&meta).unwrap())
            .map_err(|e| format!("Cannot write backup manifest: {}", e))?;
    } else {
        fs::copy(src, &dst).map_err(|e| format!("Cannot create backup: {}", e))?;
        let mut perms = fs::metadata(&dst).map_err(|e| e.to_string())?.permissions();
        perms.set_readonly(true);
        fs::set_permissions(&dst, perms).ok();
    }
    Ok(())
}

pub fn list_backups(path: &str) -> Result<Vec<(String, u64)>, String> {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let safe_name = path.replace(|c: char| !c.is_alphanumeric() && c != '.' && c != '_', "_");
    let backup_dir = PathBuf::from(appdata)
        .join("InnologyBD")
        .join("OmniLock")
        .join("backups")
        .join(&safe_name);
    if !backup_dir.exists() {
        return Ok(Vec::new());
    }
    let mut backups = Vec::new();
    if let Ok(entries) = fs::read_dir(&backup_dir) {
        for entry in entries.flatten() {
            let meta = entry.metadata().ok();
            let _size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            if let Some(modified) = meta.and_then(|m| m.modified().ok()) {
                let secs = modified
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                backups.push((entry.path().to_string_lossy().to_string(), secs));
            } else {
                backups.push((entry.path().to_string_lossy().to_string(), 0));
            }
        }
    }
    backups.sort_by(|a, b| b.1.cmp(&a.1));
    Ok(backups)
}

pub fn restore_backup(backup_path: &str, target_path: &str) -> Result<(), String> {
    let src = Path::new(backup_path);
    let dst = Path::new(target_path);
    if !src.exists() {
        return Err("Backup file does not exist".to_string());
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Cannot create target parent: {}", e))?;
    }
    fs::copy(src, dst).map_err(|e| format!("Cannot restore backup: {}", e))?;
    Ok(())
}

pub fn lock_file(path: &str) -> Result<(), String> {
    create_backup_before_lock(path)?;
    apply_safe_lock(path)
}

pub fn unlock_file(path: &str) -> Result<(), String> {
    remove_safe_lock(path)
}

fn apply_lock_icon(path: &str) -> Result<(), String> {
    let dir = Path::new(path);
    if !dir.is_dir() {
        return Err("Not a directory".to_string());
    }

    let ini_path = dir.join("desktop.ini");
    let ini_content = "[.ShellClassInfo]\r\nIconResource=%SystemRoot%\\system32\\imageres.dll,204\r\n";
    fs::write(&ini_path, ini_content).map_err(|e| e.to_string())?;

    unsafe {
        let ini_wide = to_wide(&ini_path.to_string_lossy());
        SetFileAttributesW(ini_wide.as_ptr(), FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM);
        SHChangeNotify(SHCNE_UPDATEITEM as i32, SHCNF_PATHW as u32, ini_wide.as_ptr() as _, std::ptr::null_mut());
    }

    Ok(())
}

fn remove_lock_icon(path: &str) -> Result<(), String> {
    let dir = Path::new(path);
    let ini_path = dir.join("desktop.ini");
    if ini_path.exists() {
        unsafe {
            let ini_wide = to_wide(&ini_path.to_string_lossy());
            SetFileAttributesW(ini_wide.as_ptr(), FILE_ATTRIBUTE_NORMAL);
        }
        fs::remove_file(&ini_path).map_err(|e| e.to_string())?;

        unsafe {
            let folder_wide = to_wide(path);
            SHChangeNotify(SHCNE_UPDATEITEM as i32, SHCNF_PATHW as u32, folder_wide.as_ptr() as _, std::ptr::null_mut());
        }
    }
    Ok(())
}

pub fn lock_folder(path: &str) -> Result<(), String> {
    let dir_path = Path::new(path);
    if !dir_path.exists() || !dir_path.is_dir() {
        return Err(format!("Folder does not exist: {}", path));
    }
    create_backup_before_lock(path)?;
    apply_safe_lock(path)?;
    apply_lock_icon(path)?;
    Ok(())
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
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut sd,
        );

        if is_access_denied(ret) {
            return Ok(true);
        }
        if ret != 0 {
            return Err(format!("GetNamedSecurityInfo failed: {}", ret));
        }

        // Check owner: SYSTEM = locked
        let mut owner_sid: PSID = std::ptr::null_mut();
        let mut owner_defaulted: i32 = 0;
        GetSecurityDescriptorOwner(sd, &mut owner_sid, &mut owner_defaulted);

        if !owner_sid.is_null() {
            let system_sid_str = to_wide("S-1-5-18");
            let mut system_sid: PSID = std::ptr::null_mut();
            if ConvertStringSidToSidW(system_sid_str.as_ptr(), &mut system_sid) != 0 {
                let is_system = EqualSid(owner_sid, system_sid) != 0;
                LocalFree(system_sid as _);
                LocalFree(sd);
                return Ok(is_system);
            }
        }

        LocalFree(sd);
        Ok(false)
    }
}

pub fn unlock_folder(path: &str) -> Result<(), String> {
    let dir_path = Path::new(path);
    if !dir_path.exists() || !dir_path.is_dir() {
        return Err(format!("Folder does not exist: {}", path));
    }
    remove_safe_lock(path)?;
    remove_lock_icon(path)?;
    // Recursively reset ACLs on child files that inherited the restricted DACL
    unlock_children_recursive(dir_path)?;
    Ok(())
}

fn unlock_children_recursive(dir: &Path) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|e| format!("Cannot read directory: {}", e))?;
    for entry in entries.flatten() {
        let child = entry.path();
        if child.is_dir() {
            unlock_children_recursive(&child)?;
        } else if child.is_file() {
            // Only reset if the file's ACL is restricted (owner check is fast)
            if let Ok(true) = verify_lock(&child.to_string_lossy()) {
                remove_safe_lock(&child.to_string_lossy()).ok();
            }
        }
    }
    Ok(())
}
