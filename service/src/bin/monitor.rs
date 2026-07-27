#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::cell::RefCell;
use std::sync::OnceLock;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::Storage::FileSystem::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::*;

use omnilock_svc::ipc::{SvcRequest, SvcResponse, PIPE_NAME};

const WM_TRAY_ICON: u32 = 8001;
const IDM_UNLOCK_BASE: usize = 9000;
const IDM_UNLOCK_ALL: usize = 9990;
const IDM_EXIT: usize = 9999;
const IDC_PASSWORD_EDIT: isize = 1001;
const IDC_UNLOCK_BTN: isize = 1002;
const IDC_CANCEL_BTN: isize = 1003;

struct SyncHandle(HWND);
unsafe impl Sync for SyncHandle {}
unsafe impl Send for SyncHandle {}

static HWND_MAIN: OnceLock<SyncHandle> = OnceLock::new();
thread_local! {
    static HWND_DIALOG: RefCell<HWND> = const { RefCell::new(std::ptr::null_mut()) };
    static TRAY_ITEMS: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
    static PENDING_UNLOCK_PATH: RefCell<String> = const { RefCell::new(String::new()) };
}

fn pipe_request(req: &SvcRequest) -> Option<SvcResponse> {
    let name_wide: Vec<u16> = PIPE_NAME.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let pipe = CreateFileW(
            name_wide.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        );
        if pipe == INVALID_HANDLE_VALUE { return None; }
        let json = serde_json::to_vec(req).unwrap_or_default();
        let mut bw: u32 = 0;
        WriteFile(pipe, json.as_ptr(), json.len() as u32, &mut bw, std::ptr::null_mut());
        FlushFileBuffers(pipe);
        let mut buf = vec![0u8; 8192];
        let mut br: u32;
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(5);
        loop {
            br = 0;
            ReadFile(pipe, buf.as_mut_ptr(), buf.len() as u32, &mut br, std::ptr::null_mut());
            if br > 0 { break; }
            if start.elapsed() >= timeout { break; }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        CloseHandle(pipe);
        if br > 0 { buf.truncate(br as usize); serde_json::from_slice(&buf).ok() } else { None }
    }
}

fn refresh_items() {
    if let Some(SvcResponse::LockedItems(items)) = pipe_request(&SvcRequest::GetLockedItems) {
        TRAY_ITEMS.with(|t| {
            *t.borrow_mut() = items.into_iter().map(|i| (i.path, i.display_name)).collect();
        });
    }
}

unsafe fn w(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }

unsafe fn get_text_from(hwnd: HWND, id: isize) -> String {
    let edit = GetDlgItem(hwnd, id as i32);
    let len = GetWindowTextLengthW(edit);
    if len == 0 { return String::new(); }
    let mut buf = vec![0u16; (len + 1) as usize];
    GetWindowTextW(edit, buf.as_mut_ptr(), len + 1);
    String::from_utf16_lossy(&buf[..len as usize])
}

unsafe fn show_unlock_dialog(item_path: &str, item_name: &str) {
    PENDING_UNLOCK_PATH.with(|p| { *p.borrow_mut() = item_path.to_string(); });

    let class_name = w("OmniLockUnlockDlg");
    let mut wc: WNDCLASSW = std::mem::zeroed();
    wc.lpfnWndProc = Some(dialog_proc);
    wc.hInstance = GetModuleHandleW(std::ptr::null());
    wc.lpszClassName = class_name.as_ptr();
    wc.hbrBackground = (COLOR_WINDOW + 1) as HBRUSH;
    RegisterClassW(&wc);

    let parent = HWND_MAIN.get().unwrap().0;
    let title = format!("Unlock: {}", item_name);
    HWND_DIALOG.with(|d| {
        *d.borrow_mut() = CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            class_name.as_ptr(),
            w(&title).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            0, 0, 400, 160,
            parent, std::ptr::null_mut(), wc.hInstance, std::ptr::null(),
        );
    });

    let dialog = HWND_DIALOG.with(|d| *d.borrow());

    let prompt = format!("Enter password to unlock:\n{}", item_name);
    CreateWindowExW(0, w("STATIC").as_ptr(), w(&prompt).as_ptr(), WS_CHILD | WS_VISIBLE, 10, 10, 370, 50, dialog, std::ptr::null_mut(), wc.hInstance, std::ptr::null());

    CreateWindowExW(WS_EX_CLIENTEDGE, w("EDIT").as_ptr(), std::ptr::null(), WS_CHILD | WS_VISIBLE | WS_TABSTOP | 0x20, 10, 65, 370, 24, dialog, IDC_PASSWORD_EDIT as *mut _, wc.hInstance, std::ptr::null());

    CreateWindowExW(0, w("BUTTON").as_ptr(), w("Unlock").as_ptr(), WS_CHILD | WS_VISIBLE | WS_TABSTOP | 0x01, 220, 100, 80, 28, dialog, IDC_UNLOCK_BTN as *mut _, wc.hInstance, std::ptr::null());
    CreateWindowExW(0, w("BUTTON").as_ptr(), w("Cancel").as_ptr(), WS_CHILD | WS_VISIBLE | WS_TABSTOP, 310, 100, 70, 28, dialog, IDC_CANCEL_BTN as *mut _, wc.hInstance, std::ptr::null());

    let h_edit = GetDlgItem(dialog, IDC_PASSWORD_EDIT as i32);
    SendMessageW(h_edit, WM_SETFOCUS, 0, 0);
    ShowWindow(dialog, SW_SHOW);
    UpdateWindow(dialog);

    let mut msg: MSG = std::mem::zeroed();
    while GetMessageW(&mut msg, dialog, 0, 0) > 0 {
        if IsDialogMessageW(dialog, &mut msg) == 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    HWND_DIALOG.with(|d| { *d.borrow_mut() = std::ptr::null_mut(); });
}

unsafe extern "system" fn dialog_proc(hwnd: HWND, msg: u32, wparam: WPARAM, _lparam: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as isize;
            if id == IDC_UNLOCK_BTN {
                let password = get_text_from(hwnd, IDC_PASSWORD_EDIT);
                if password.is_empty() {
                    MessageBoxW(hwnd, w("Please enter a password").as_ptr(), w("OmniLock").as_ptr(), 0x00000030);
                    return 0;
                }
                let path = PENDING_UNLOCK_PATH.with(|p| p.borrow().clone());
                let req = SvcRequest::UnlockItem { path, password };
                let resp = pipe_request(&req);
                let (msg_text, icon) = match resp {
                    Some(SvcResponse::Ok { message }) => (message, 0x00000040),
                    Some(SvcResponse::Error { message }) => (message, 0x00000010),
                    _ => ("Service not reachable".to_string(), 0x00000010),
                };
                MessageBoxW(hwnd, w(&msg_text).as_ptr(), w("OmniLock").as_ptr(), 0x00000000 | icon);
                refresh_items();
                DestroyWindow(hwnd);
            } else if id == IDC_CANCEL_BTN || id == 2 {
                DestroyWindow(hwnd);
            }
            0
        }
        WM_CLOSE => { DestroyWindow(hwnd); 0 }
        WM_DESTROY => { PostQuitMessage(0); 0 }
        _ => DefWindowProcW(hwnd, msg, wparam, _lparam),
    }
}

unsafe fn build_tray_menu() -> HMENU {
    let menu = CreatePopupMenu();
    let items = TRAY_ITEMS.with(|t| t.borrow().clone());
    let count = items.len();
    if count == 0 {
        AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, w("No locked items").as_ptr());
    } else {
        let header = format!("{} locked item(s):", count);
        AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, w(&header).as_ptr());
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        for (i, (_, name)) in items.iter().enumerate() {
            let label = format!("Unlock: {}", name);
            AppendMenuW(menu, MF_STRING, IDM_UNLOCK_BASE + i, w(&label).as_ptr());
        }
        if count > 1 {
            AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
            AppendMenuW(menu, MF_STRING, IDM_UNLOCK_ALL, w("Unlock All").as_ptr());
        }
    }
    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
    AppendMenuW(menu, MF_STRING, IDM_EXIT, w("Exit Monitor").as_ptr());
    menu
}

unsafe fn add_tray_icon() {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = HWND_MAIN.get().unwrap().0;
    nid.uID = 1;
    nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY_ICON;
    let tip = w("OmniLock Monitor");
    let len = tip.len().min(128);
    nid.szTip[..len].copy_from_slice(&tip[..len]);
    nid.hIcon = LoadIconW(std::ptr::null_mut(), IDI_SHIELD as *const u16);
    if nid.hIcon.is_null() { nid.hIcon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION as *const u16); }
    Shell_NotifyIconW(NIM_ADD, &nid);
}

unsafe fn remove_tray_icon() {
    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = HWND_MAIN.get().unwrap().0;
    nid.uID = 1;
    Shell_NotifyIconW(NIM_DELETE, &nid);
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TRAY_ICON => {
            if lparam as u32 == WM_RBUTTONUP {
                refresh_items();
                let menu = build_tray_menu();
                let mut pt = POINT { x: 0, y: 0 };
                GetCursorPos(&mut pt);
                SetForegroundWindow(hwnd);
                let cmd = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_NONOTIFY, pt.x, pt.y, 0, hwnd, std::ptr::null()) as usize;
                let items = TRAY_ITEMS.with(|t| t.borrow().clone());
                let count = items.len();
                if cmd >= IDM_UNLOCK_BASE && cmd < IDM_UNLOCK_BASE + count {
                    let idx = cmd - IDM_UNLOCK_BASE;
                    let (path, name) = items[idx].clone();
                    show_unlock_dialog(&path, &name);
                } else if cmd == IDM_UNLOCK_ALL {
                    show_unlock_dialog("*", "ALL items");
                } else if cmd == IDM_EXIT {
                    remove_tray_icon();
                    PostQuitMessage(0);
                }
                DestroyMenu(menu);
            }
            0
        }
        WM_DESTROY => { remove_tray_icon(); PostQuitMessage(0); 0 }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn main() {
    unsafe {
        let class_name = w("OmniLockMonitorWnd");
        let mut wc: WNDCLASSW = std::mem::zeroed();
        wc.lpfnWndProc = Some(wnd_proc);
        wc.hInstance = GetModuleHandleW(std::ptr::null());
        wc.lpszClassName = class_name.as_ptr();
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(0, class_name.as_ptr(), std::ptr::null(), 0, 0, 0, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(), wc.hInstance, std::ptr::null());
        if hwnd.is_null() { return; }
        HWND_MAIN.set(SyncHandle(hwnd)).ok();

        add_tray_icon();
        refresh_items();

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
