use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Once;

static AUTO_LOCK_ENABLED: AtomicBool = AtomicBool::new(false);
static AUTO_LOCK_MINUTES: AtomicU32 = AtomicU32::new(5);
static RUNNING: AtomicBool = AtomicBool::new(false);
static ONCE: Once = Once::new();

#[repr(C)]
struct LASTINPUTINFO {
    cb_size: u32,
    dw_time: u32,
}

pub fn set_auto_lock_minutes(minutes: u32) {
    AUTO_LOCK_MINUTES.store(minutes, Ordering::SeqCst);
    AUTO_LOCK_ENABLED.store(minutes > 0, Ordering::SeqCst);
}

pub fn start_auto_lock_monitor() {
    ONCE.call_once(|| {
        std::thread::spawn(|| {
            RUNNING.store(true, Ordering::SeqCst);
            loop {
                if !RUNNING.load(Ordering::SeqCst) {
                    break;
                }

                if AUTO_LOCK_ENABLED.load(Ordering::SeqCst) {
                    let minutes = AUTO_LOCK_MINUTES.load(Ordering::SeqCst);
                    if minutes == 0 {
                        // Immediate mode - skip sleep
                        std::thread::sleep(std::time::Duration::from_secs(5));
                        continue;
                    }

                    let idle_secs = get_idle_time_secs();
                    let threshold = minutes as u64 * 60;

                    if idle_secs >= threshold {
                        do_lock_workstation();
                        // Reset after locking
                        AUTO_LOCK_ENABLED.store(false, Ordering::SeqCst);
                        std::thread::sleep(std::time::Duration::from_secs(60));
                        AUTO_LOCK_ENABLED.store(minutes > 0, Ordering::SeqCst);
                    }
                }

                std::thread::sleep(std::time::Duration::from_secs(5));
            }
        });
    });
}

fn get_idle_time_secs() -> u64 {
    unsafe {
        extern "system" {
            fn GetModuleHandleA(name: *const u8) -> isize;
            fn GetProcAddress(module: isize, name: *const u8) -> usize;
            fn GetTickCount() -> u32;
        }

        let user32 = GetModuleHandleA(b"user32.dll\0".as_ptr());
        if user32 == 0 { return 0; }

        let addr = GetProcAddress(user32, b"GetLastInputInfo\0".as_ptr());
        if addr == 0 { return 0; }

        type GetLastInputInfoFn = unsafe extern "system" fn(*mut LASTINPUTINFO) -> i32;
        let get_last_input: GetLastInputInfoFn = std::mem::transmute(addr);

        let mut info = LASTINPUTINFO {
            cb_size: std::mem::size_of::<LASTINPUTINFO>() as u32,
            dw_time: 0,
        };

        if get_last_input(&mut info) == 0 {
            return 0;
        }

        let tick = GetTickCount();
        let elapsed = tick.wrapping_sub(info.dw_time);
        (elapsed / 1000) as u64
    }
}

fn do_lock_workstation() {
    unsafe {
        extern "system" {
            fn GetModuleHandleA(name: *const u8) -> isize;
            fn GetProcAddress(module: isize, name: *const u8) -> usize;
        }

        let user32 = GetModuleHandleA(b"user32.dll\0".as_ptr());
        if user32 == 0 { return; }

        let addr = GetProcAddress(user32, b"LockWorkStation\0".as_ptr());
        if addr == 0 { return; }

        type LockWorkStationFn = unsafe extern "system" fn() -> i32;
        let lock_fn: LockWorkStationFn = std::mem::transmute(addr);
        lock_fn();
    }
}

pub fn stop_auto_lock_monitor() {
    RUNNING.store(false, Ordering::SeqCst);
}
