use std::path::Path;

pub fn lock_file(path: &str) -> Result<(), String> {
    if !Path::new(path).exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    // ACL lockdown via icacls
    let result = std::process::Command::new("icacls")
        .args([path, "/deny", "Everyone:(OI)(CI)F", "/T", "/Q"])
        .output()
        .map_err(|e| e.to_string())?;

    if !result.status.success() {
        return Err(format!("Failed to lock: {}", String::from_utf8_lossy(&result.stderr)));
    }
    Ok(())
}

pub fn unlock_file(path: &str) -> Result<(), String> {
    if !Path::new(path).exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    let result = std::process::Command::new("icacls")
        .args([path, "/grant", "Everyone:(OI)(CI)F", "/T", "/Q"])
        .output()
        .map_err(|e| e.to_string())?;

    if !result.status.success() {
        return Err(format!("Failed to unlock: {}", String::from_utf8_lossy(&result.stderr)));
    }
    Ok(())
}

pub fn lock_folder(path: &str) -> Result<(), String> {
    let dir_path = std::path::Path::new(path);
    if !dir_path.exists() || !dir_path.is_dir() {
        return Err(format!("Folder does not exist: {}", path));
    }
    lock_file(path)?;

    for entry in std::fs::read_dir(dir_path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let entry_path = entry.path();
        if entry_path.is_file() {
            lock_file(entry_path.to_string_lossy().as_ref())?;
        }
    }
    Ok(())
}

pub fn unlock_folder(path: &str) -> Result<(), String> {
    let dir_path = std::path::Path::new(path);
    if !dir_path.exists() || !dir_path.is_dir() {
        return Err(format!("Folder does not exist: {}", path));
    }
    unlock_file(path)?;

    for entry in std::fs::read_dir(dir_path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let entry_path = entry.path();
        if entry_path.is_file() {
            unlock_file(entry_path.to_string_lossy().as_ref())?;
        }
    }
    Ok(())
}
