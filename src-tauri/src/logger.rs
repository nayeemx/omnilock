use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use std::time::SystemTime;

static LOG_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

fn log_path() -> std::path::PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(appdata)
        .join("InnologyBD")
        .join("OmniLock")
        .join("omnilock.log")
}

pub fn init() {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let mut guard = LOG_FILE.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(file);
        drop(guard);
    }
}

pub fn log(component: &str, message: &str) {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_else(|_| "0".to_string());

    let entry = format!("[{}] {}: {}\n", now, component, message);

    let mut guard = LOG_FILE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(ref mut file) = *guard {
        let _ = file.write_all(entry.as_bytes());
        let _ = file.flush();
    }
}

pub fn read_log(max_bytes: usize) -> String {
    let path = log_path();
    if !path.exists() {
        return String::new();
    }
    match std::fs::read(&path) {
        Ok(bytes) => {
            let start = if bytes.len() > max_bytes {
                bytes.len() - max_bytes
            } else {
                0
            };
            String::from_utf8_lossy(&bytes[start..]).to_string()
        }
        Err(_) => String::new(),
    }
}

pub fn clear_log() {
    let path = log_path();
    let _ = std::fs::remove_file(&path);
    let mut guard = LOG_FILE.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
    if let Ok(file) = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&path)
    {
        *guard = Some(file);
    }
}
