use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use windows_service::service::*;
use windows_service::service_control_handler::{self, ServiceControlHandlerResult, ServiceStatusHandle};
use windows_service::service_dispatcher;

use omnilock_svc::acl;
use omnilock_svc::ipc::{SvcRequest, SvcResponse, PIPE_NAME};
use omnilock_svc::state::{self, LockedItemsState};
use omnilock_svc::vault;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Security::*;
use windows_sys::Win32::Storage::FileSystem::*;
use windows_sys::Win32::System::Pipes::*;

static STOP_FLAG: AtomicBool = AtomicBool::new(false);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|a| a.as_str()) == Some("--standalone") {
        if let Err(e) = run_standalone() {
            eprintln!("Standalone error: {}", e);
        }
        return;
    }
    service_dispatcher::start("OmniLockService", ffi_service_main).unwrap();
}

fn run_standalone() -> Result<(), Box<dyn std::error::Error>> {
    let locked = Arc::new(Mutex::new(state::load_state()));
    eprintln!("[Svc-Standalone] Loaded {} locked items", locked.lock().unwrap().locked_items.len());
    state::sync_vault_to_programdata();

    let locked_pipe = locked.clone();
    std::thread::spawn(move || pipe_server_thread(locked_pipe));

    let locked_acl = locked.clone();
    std::thread::spawn(move || acl_enforcement_thread(locked_acl));

    eprintln!("[Svc-Standalone] Running. Press Ctrl+C to stop.");
    while !STOP_FLAG.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_secs(1));
    }
    eprintln!("[Svc-Standalone] Stopping...");
    let s = locked.lock().unwrap();
    let _ = state::save_state(&s);
    Ok(())
}

extern "system" fn ffi_service_main(_argc: u32, _argv: *mut *mut u16) {
    if let Err(e) = run_service() {
        eprintln!("Service error: {}", e);
    }
}

fn run_service() -> Result<(), Box<dyn std::error::Error>> {
    let status_handle = service_control_handler::register(
        OsString::from("OmniLockService"),
        handle_control,
    )?;

    set_status(&status_handle, ServiceState::StartPending, 1)?;

    let locked = Arc::new(Mutex::new(state::load_state()));
    eprintln!("[Svc] Loaded {} locked items", locked.lock().unwrap().locked_items.len());
    state::sync_vault_to_programdata();

    let locked_pipe = locked.clone();
    std::thread::spawn(move || pipe_server_thread(locked_pipe));

    let locked_acl = locked;
    std::thread::spawn(move || acl_enforcement_thread(locked_acl));

    set_status(&status_handle, ServiceState::Running, 0)?;

    while !STOP_FLAG.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_secs(1));
    }

    set_status(&status_handle, ServiceState::Stopped, 0)?;
    Ok(())
}

fn acl_enforcement_thread(locked: Arc<Mutex<LockedItemsState>>) {
    loop {
        std::thread::sleep(Duration::from_secs(15));
        if STOP_FLAG.load(Ordering::Relaxed) { break; }
        let s = locked.lock().unwrap();
        for item in &s.locked_items {
            if item.item_type == "drive" {
                let letter = item.path.chars().next().unwrap_or('?').to_string();
                let _ = acl::lock_drive(&letter);
            } else if std::path::Path::new(&item.path).exists() {
                let _ = acl::apply_lock(&item.path);
            }
        }
    }
}

fn handle_control(event: ServiceControl) -> ServiceControlHandlerResult {
    match event {
        ServiceControl::Stop => {
            STOP_FLAG.store(true, Ordering::Relaxed);
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    }
}

fn set_status(
    handle: &ServiceStatusHandle,
    state: ServiceState,
    checkpoint: u32,
) -> Result<(), windows_service::Error> {
    handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::ServiceSpecific(0),
        checkpoint,
        wait_hint: Duration::from_secs(10),
        process_id: None,
    })
}

fn log(msg: &str) {
    use std::io::Write;
    let _ = std::fs::OpenOptions::new()
        .create(true).append(true)
        .open(r"C:\ProgramData\InnologyBD\OmniLock\svc.log")
        .and_then(|mut f| writeln!(f, "[{}] {}", chrono_str(), msg));
}

fn chrono_str() -> String {
    format!("{:?}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
}

fn build_pipe_security() -> (Box<SECURITY_DESCRIPTOR>, SECURITY_ATTRIBUTES) {
    unsafe {
        let mut sd: Box<SECURITY_DESCRIPTOR> = Box::new(std::mem::zeroed());
        let sd_ptr = &mut *sd as *mut SECURITY_DESCRIPTOR as *mut _;
        InitializeSecurityDescriptor(sd_ptr, 1);
        SetSecurityDescriptorDacl(sd_ptr, 1, std::ptr::null_mut(), 0);

        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: sd_ptr,
            bInheritHandle: 0,
        };
        (sd, sa)
    }
}

fn pipe_server_thread(locked: Arc<Mutex<LockedItemsState>>) {
    let name_wide: Vec<u16> = PIPE_NAME.encode_utf16().chain(std::iter::once(0)).collect();
    let (_sd, mut sa) = build_pipe_security();

    loop {
        if STOP_FLAG.load(Ordering::Relaxed) { break; }

        unsafe {
            let pipe = CreateNamedPipeW(
                name_wide.as_ptr(),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_WAIT,
                1, 8192, 8192, 0,
                &mut sa as *mut _,
            );
            if pipe == INVALID_HANDLE_VALUE {
                log(&format!("CreateNamedPipe failed: {}", std::io::Error::last_os_error()));
                std::thread::sleep(Duration::from_secs(2));
                continue;
            }
            log(&format!("Pipe created, handle={:?}", pipe));

            let connected = ConnectNamedPipe(pipe, std::ptr::null_mut());
            let err = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            if connected == 0 && err != 535 {
                log(&format!("ConnectNamedPipe failed: err={}", err));
                DisconnectNamedPipe(pipe);
                CloseHandle(pipe);
                continue;
            }

            log("Client connected, reading...");

            let mut buf = vec![0u8; 8192];
            let mut bytes_read: u32 = 0;
            let ok = ReadFile(pipe, buf.as_mut_ptr(), buf.len() as u32, &mut bytes_read, std::ptr::null_mut());

            if ok == 0 || bytes_read == 0 {
                log(&format!("ReadFile failed: ok={} err={}", ok, std::io::Error::last_os_error()));
                DisconnectNamedPipe(pipe);
                CloseHandle(pipe);
                continue;
            }

            log(&format!("Read {} bytes: {:?}", bytes_read, &buf[..bytes_read as usize]));
            buf.truncate(bytes_read as usize);

            let req: SvcRequest = match serde_json::from_slice(&buf) {
                Ok(r) => r,
                Err(e) => {
                    let resp = SvcResponse::Error { message: format!("Bad JSON: {}", e) };
                    write_response(pipe, &resp);
                    DisconnectNamedPipe(pipe);
                    CloseHandle(pipe);
                    continue;
                }
            };

            let resp = process_request(req, &locked);
            write_response(pipe, &resp);
            let mut drain = [0u8; 64];
            let mut drain_n: u32 = 0;
            ReadFile(pipe, drain.as_mut_ptr(), drain.len() as u32, &mut drain_n, std::ptr::null_mut());
            DisconnectNamedPipe(pipe);
            CloseHandle(pipe);
        }
    }
}

unsafe fn write_response(pipe: windows_sys::Win32::Foundation::HANDLE, resp: &SvcResponse) {
    let json = serde_json::to_vec(resp).unwrap_or_default();
    let mut bw: u32 = 0;
    log(&format!("Writing {} bytes: {:?}", json.len(), &json));
    let ok = WriteFile(pipe, json.as_ptr(), json.len() as u32, &mut bw, std::ptr::null_mut());
    log(&format!("WriteFile ok={} bw={}", ok, bw));
}

fn process_request(request: SvcRequest, locked: &Arc<Mutex<LockedItemsState>>) -> SvcResponse {
    match request {
        SvcRequest::Ping => SvcResponse::Pong,

        SvcRequest::GetStatus => {
            let s = locked.lock().unwrap();
            SvcResponse::Status { running: true, locked_count: s.locked_items.len() }
        }

        SvcRequest::GetLockedItems => {
            let s = locked.lock().unwrap();
            SvcResponse::LockedItems(s.locked_items.clone())
        }

        SvcRequest::LockFile { path, display_name } => {
            let mut s = locked.lock().unwrap();
            if !s.locked_items.iter().any(|i| i.path == path) {
                s.locked_items.push(state::LockedItem {
                    item_type: "file".to_string(), path: path.clone(), display_name: display_name.clone(),
                });
                let _ = state::save_state(&s);
            }
            drop(s);
            match acl::apply_lock(&path) {
                Ok(()) => SvcResponse::Ok { message: format!("Locked: {}", path) },
                Err(e) => {
                    log(&format!("ACL warning for {}: {}", path, e));
                    SvcResponse::Ok { message: format!("Locked (ACL pending): {}", path) }
                }
            }
        }

        SvcRequest::LockFolder { path, display_name } => {
            let mut s = locked.lock().unwrap();
            if !s.locked_items.iter().any(|i| i.path == path) {
                s.locked_items.push(state::LockedItem {
                    item_type: "folder".to_string(), path: path.clone(), display_name: display_name.clone(),
                });
                let _ = state::save_state(&s);
            }
            drop(s);
            match acl::apply_lock(&path) {
                Ok(()) => SvcResponse::Ok { message: format!("Locked: {}", path) },
                Err(e) => {
                    log(&format!("ACL warning for {}: {}", path, e));
                    SvcResponse::Ok { message: format!("Locked (ACL pending): {}", path) }
                }
            }
        }

        SvcRequest::LockDrive { drive_letter, display_name } => {
            let mut s = locked.lock().unwrap();
            let path = format!("{}:\\", drive_letter);
            if !s.locked_items.iter().any(|i| i.path == path) {
                s.locked_items.push(state::LockedItem {
                    item_type: "drive".to_string(), path: path.clone(), display_name,
                });
                let _ = state::save_state(&s);
            }
            drop(s);
            match acl::lock_drive(&drive_letter) {
                Ok(()) => SvcResponse::Ok { message: format!("Drive {} locked", drive_letter) },
                Err(e) => {
                    log(&format!("ACL warning for drive {}: {}", drive_letter, e));
                    SvcResponse::Ok { message: format!("Drive {} locked (ACL pending)", drive_letter) }
                }
            }
        }

        SvcRequest::LockApp { name, path, display_name } => {
            let mut s = locked.lock().unwrap();
            if !s.locked_items.iter().any(|i| i.path == path) {
                s.locked_items.push(state::LockedItem {
                    item_type: "app".to_string(), path: path.clone(),
                    display_name: if display_name.is_empty() { name } else { display_name },
                });
                let _ = state::save_state(&s);
            }
            drop(s);
            match acl::apply_lock(&path) {
                Ok(()) => SvcResponse::Ok { message: "App locked".to_string() },
                Err(e) => {
                    log(&format!("ACL warning for app {}: {}", path, e));
                    SvcResponse::Ok { message: "App locked (ACL pending)".to_string() }
                }
            }
        }

        SvcRequest::UnlockItem { path, password } => {
            if !verify_password(&password) {
                return SvcResponse::Error { message: "Incorrect password".to_string() };
            }
            if path.ends_with(":\\") || (path.len() == 2 && path.ends_with(':')) {
                let letter = path.chars().next().unwrap_or('?').to_string();
                let _ = acl::unlock_drive(&letter);
            } else {
                let _ = acl::remove_lock(&path);
            }
            let mut s = locked.lock().unwrap();
            s.locked_items.retain(|i| i.path != path);
            let _ = state::save_state(&s);
            SvcResponse::Ok { message: format!("Unlocked: {}", path) }
        }

        SvcRequest::SyncVault { vault_data } => {
            let vault_path = state::vault_path_programdata();
            if let Err(e) = std::fs::write(&vault_path, &vault_data) {
                return SvcResponse::Error { message: format!("Save failed: {}", e) };
            }
            SvcResponse::Ok { message: "Vault synced".to_string() }
        }
        SvcRequest::Shutdown => {
            STOP_FLAG.store(true, Ordering::Relaxed);
            SvcResponse::Ok { message: "Shutting down".to_string() }
        }
    }
}

fn verify_password(password: &str) -> bool {
    // Verify by trying to decrypt the vault with the provided password
    vault::verify_vault_password(password)
}
