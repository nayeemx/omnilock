use std::path::Path;

// ACL enforcement is handled by the Windows service via named pipe.
// These functions only validate that the path exists.

pub fn lock_file(path: &str) -> Result<(), String> {
    if !Path::new(path).exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    Ok(())
}

pub fn unlock_file(path: &str) -> Result<(), String> {
    if !Path::new(path).exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    Ok(())
}

pub fn lock_folder(path: &str) -> Result<(), String> {
    let dir_path = std::path::Path::new(path);
    if !dir_path.exists() || !dir_path.is_dir() {
        return Err(format!("Folder does not exist: {}", path));
    }
    Ok(())
}

pub fn unlock_folder(path: &str) -> Result<(), String> {
    let dir_path = std::path::Path::new(path);
    if !dir_path.exists() || !dir_path.is_dir() {
        return Err(format!("Folder does not exist: {}", path));
    }
    Ok(())
}
