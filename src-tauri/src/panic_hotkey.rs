use std::sync::atomic::{AtomicBool, Ordering};

static PANIC_ACTIVE: AtomicBool = AtomicBool::new(false);

type RegisterHotKeyFn = unsafe extern "system" fn(i32, i32, u32, u32) -> i32;
type GetMessageWFn = unsafe extern "system" fn(*mut MSG, i32, u32, u32) -> i32;
type TranslateMessageFn = unsafe extern "system" fn(*const MSG) -> i32;
type DispatchMessageWFn = unsafe extern "system" fn(*const MSG) -> i32;

#[repr(C)]
struct MSG {
    hwnd: isize,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    pt_x: i32,
    pt_y: i32,
}

const MOD_ALT: u32 = 0x0001;
const MOD_WIN: u32 = 0x0800;
const WM_HOTKEY: u32 = 0x0312;

pub fn register_panic_hotkey() -> Result<(), String> {
    unsafe {
        extern "system" {
            fn GetModuleHandleA(name: *const u8) -> isize;
            fn GetProcAddress(module: isize, name: *const u8) -> usize;
        }

        let user32 = GetModuleHandleA(b"user32.dll\0".as_ptr());
        if user32 == 0 {
            return Err("Cannot load user32.dll".to_string());
        }

        let addr = GetProcAddress(user32, b"RegisterHotKey\0".as_ptr());
        if addr == 0 {
            return Err("RegisterHotKey not found".to_string());
        }

        let register_hotkey: RegisterHotKeyFn = std::mem::transmute(addr);
        let result = register_hotkey(0, 1, MOD_WIN | MOD_ALT, 0x4C);
        if result == 0 {
            let err = windows_sys::Win32::Foundation::GetLastError();
            if err != 0 {
                return Err(format!("Failed to register hotkey (error {})", err));
            }
        }
    }
    Ok(())
}

pub fn start_hotkey_listener() {
    std::thread::spawn(|| unsafe {
        extern "system" {
            fn GetModuleHandleA(name: *const u8) -> isize;
            fn GetProcAddress(module: isize, name: *const u8) -> usize;
        }

        let user32 = GetModuleHandleA(b"user32.dll\0".as_ptr());
        if user32 == 0 { return; }

        let get_msg_addr = GetProcAddress(user32, b"GetMessageW\0".as_ptr());
        let translate_addr = GetProcAddress(user32, b"TranslateMessage\0".as_ptr());
        let dispatch_addr = GetProcAddress(user32, b"DispatchMessageW\0".as_ptr());

        if get_msg_addr == 0 || translate_addr == 0 || dispatch_addr == 0 { return; }

        let get_msg: GetMessageWFn = std::mem::transmute(get_msg_addr);
        let translate: TranslateMessageFn = std::mem::transmute(translate_addr);
        let dispatch: DispatchMessageWFn = std::mem::transmute(dispatch_addr);

        let mut msg: MSG = std::mem::zeroed();
        loop {
            let result = get_msg(&mut msg, 0, 0, 0);
            if result == 0 || result == -1 { break; }
            if msg.message == WM_HOTKEY {
                PANIC_ACTIVE.store(true, Ordering::SeqCst);
            }
            translate(&msg);
            dispatch(&msg);
        }
    });
}

pub fn is_panic_active() -> bool {
    PANIC_ACTIVE.load(Ordering::SeqCst)
}

pub fn deactivate_panic() {
    PANIC_ACTIVE.store(false, Ordering::SeqCst);
}
