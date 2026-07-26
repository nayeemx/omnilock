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
            return Err(format!("Failed to register hotkey (error {})", err));
        }
    }
    Ok(())
}

pub fn do_lock_workstation() {
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

pub fn do_mute_audio() {
    unsafe {
        extern "system" {
            fn CoInitializeEx(pvReserved: *mut core::ffi::c_void, dwCoInit: u32) -> i32;
            fn CoCreateInstance(
                rclsid: *const core::ffi::c_void,
                pUnkOuter: *mut core::ffi::c_void,
                dwClsContext: u32,
                riid: *const core::ffi::c_void,
                ppv: *mut *mut core::ffi::c_void,
            ) -> i32;
            fn CoUninitialize();
        }

        const COINIT_APARTMENTTHREADED: u32 = 0x2;
        const CLSCTX_ALL: u32 = 0x17;

        let hr = CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED);
        if hr < 0 && hr != -2147417850 {
            return;
        }

        let clsid_mmdevenum: [u8; 16] = [
            0x95, 0x03, 0xDE, 0xBC, 0x2F, 0xE5, 0x7C, 0x46,
            0x8E, 0x3D, 0xC4, 0x57, 0x92, 0x91, 0x69, 0x2E,
        ];
        let iid_immdevenum: [u8; 16] = [
            0xD2, 0x64, 0x56, 0xA9, 0x14, 0x96, 0x35, 0x4F,
            0xA7, 0x46, 0xDE, 0x8D, 0xB6, 0x36, 0x17, 0xE6,
        ];

        let mut p_enumerator: *mut core::ffi::c_void = std::ptr::null_mut();
        let hr = CoCreateInstance(
            clsid_mmdevenum.as_ptr() as *const _,
            std::ptr::null_mut(),
            CLSCTX_ALL,
            iid_immdevenum.as_ptr() as *const _,
            &mut p_enumerator,
        );
        if hr < 0 || p_enumerator.is_null() {
            CoUninitialize();
            return;
        }

        let vtable = *(p_enumerator as *const *const *const core::ffi::c_void);

        type GetDefaultAudioEndpointFn = unsafe extern "system" fn(*mut core::ffi::c_void, i32, u32, *mut *mut core::ffi::c_void) -> i32;
        let get_default: GetDefaultAudioEndpointFn = std::mem::transmute(*vtable.add(4));

        let mut p_device: *mut core::ffi::c_void = std::ptr::null_mut();
        let hr = get_default(p_enumerator, 0, 0, &mut p_device);
        if hr < 0 || p_device.is_null() {
            type ReleaseFn = unsafe extern "system" fn(*mut core::ffi::c_void) -> u32;
            let release: ReleaseFn = std::mem::transmute(*vtable.add(2));
            release(p_enumerator);
            CoUninitialize();
            return;
        }

        let iid_audioepvol: [u8; 16] = [
            0x82, 0x2C, 0xC5, 0x5C, 0x1E, 0x84, 0x46, 0x45,
            0x97, 0x22, 0x0C, 0xF7, 0x40, 0x78, 0x22, 0x9A,
        ];

        let device_vtable = *(p_device as *const *const *const core::ffi::c_void);
        type ActivateFn = unsafe extern "system" fn(
            *mut core::ffi::c_void,
            *const core::ffi::c_void,
            u32,
            *mut core::ffi::c_void,
            *mut *mut core::ffi::c_void,
        ) -> i32;
        let activate: ActivateFn = std::mem::transmute(*device_vtable.add(3));

        let mut p_volume: *mut core::ffi::c_void = std::ptr::null_mut();
        let hr = activate(
            p_device,
            iid_audioepvol.as_ptr() as *const _,
            0x1,
            std::ptr::null_mut(),
            &mut p_volume,
        );

        if hr >= 0 && !p_volume.is_null() {
            let vol_vtable = *(p_volume as *const *const *const core::ffi::c_void);
            type SetMuteFn = unsafe extern "system" fn(*mut core::ffi::c_void, i32, *const GUID) -> i32;
            let set_mute: SetMuteFn = std::mem::transmute(*vol_vtable.add(10));
            set_mute(p_volume, 1, std::ptr::null());

            type VolReleaseFn = unsafe extern "system" fn(*mut core::ffi::c_void) -> u32;
            let vol_release: VolReleaseFn = std::mem::transmute(*vol_vtable.add(2));
            vol_release(p_volume);
        }

        type DeviceReleaseFn = unsafe extern "system" fn(*mut core::ffi::c_void) -> u32;
        let device_release: DeviceReleaseFn = std::mem::transmute(*device_vtable.add(2));
        device_release(p_device);

        type EnumReleaseFn = unsafe extern "system" fn(*mut core::ffi::c_void) -> u32;
        let enum_release: EnumReleaseFn = std::mem::transmute(*vtable.add(2));
        enum_release(p_enumerator);

        CoUninitialize();
    }
}

#[repr(C)]
struct GUID {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

pub fn panic_lock() {
    do_mute_audio();
    do_lock_workstation();
}

pub fn start_hotkey_listener() {
    std::thread::spawn(|| {
        if let Err(e) = register_panic_hotkey() {
            eprintln!("Failed to register panic hotkey: {}", e);
            return;
        }

        unsafe {
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
                    panic_lock();
                }
                translate(&msg);
                dispatch(&msg);
            }
        }
    });
}
