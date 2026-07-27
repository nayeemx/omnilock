use std::time::Duration;
pub use omnilock_shared::*;

fn pipe_request(req: &SvcRequest) -> Option<SvcResponse> {
    use std::os::windows::ffi::OsStrExt;
    let name_wide: Vec<u16> = std::ffi::OsStr::new(PIPE_NAME)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        use windows_sys::Win32::Storage::FileSystem::*;
        use windows_sys::Win32::Foundation::*;

        let pipe = CreateFileW(
            name_wide.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );
        if pipe == INVALID_HANDLE_VALUE {
            return None;
        }

        let json = serde_json::to_vec(req).unwrap_or_default();
        let mut bw: u32 = 0;
        WriteFile(pipe, json.as_ptr(), json.len() as u32, &mut bw, std::ptr::null_mut());
        FlushFileBuffers(pipe);

        // Read with retry timeout (up to 5 seconds)
        let mut buf = vec![0u8; 8192];
        let mut br: u32;
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(5);
        loop {
            br = 0;
            ReadFile(pipe, buf.as_mut_ptr(), buf.len() as u32, &mut br, std::ptr::null_mut());
            if br > 0 {
                break;
            }
            if start.elapsed() >= timeout {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        CloseHandle(pipe);

        if br > 0 {
            buf.truncate(br as usize);
            serde_json::from_slice(&buf).ok()
        } else {
            None
        }
    }
}

pub fn is_service_running() -> bool {
    match pipe_request(&SvcRequest::Ping) {
        Some(SvcResponse::Pong) => true,
        _ => false,
    }
}

pub fn notify_lock_file(path: &str) {
    let display_name = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    let _ = pipe_request(&SvcRequest::LockFile {
        path: path.to_string(),
        display_name,
    });
}

pub fn notify_lock_folder(path: &str) {
    let display_name = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());
    let _ = pipe_request(&SvcRequest::LockFolder {
        path: path.to_string(),
        display_name,
    });
}

pub fn notify_lock_drive(drive_letter: &str) {
    let _ = pipe_request(&SvcRequest::LockDrive {
        drive_letter: drive_letter.to_string(),
        display_name: format!("{}:\\", drive_letter),
    });
}

pub fn notify_unlock_item(path: &str, password: &str) {
    let _ = pipe_request(&SvcRequest::UnlockItem {
        path: path.to_string(),
        password: password.to_string(),
    });
}

pub fn sync_vault_to_service(vault_data: &[u8]) {
    let _ = pipe_request(&SvcRequest::SyncVault {
        vault_data: vault_data.to_vec(),
    });
}
