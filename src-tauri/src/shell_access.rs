use std::ptr;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_LOCAL_SERVER, COINIT_MULTITHREADED, IDispatch};
use windows::Win32::UI::Shell::{IWebBrowserApp, IShellWindows, ShellWindows};
use windows::Win32::System::Variant::{VARIANT, VARENUM};
use windows::core::Interface;

fn make_vt_i4(value: i32) -> VARIANT {
    unsafe {
        let mut v = VARIANT::default();
        let inner = &mut *v.Anonymous.Anonymous;
        ptr::write(&mut inner.vt, VARENUM(3u16));
        ptr::write(&mut inner.Anonymous.lVal, value);
        v
    }
}

pub fn get_explorer_paths() -> Vec<String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    let shell: IShellWindows = match unsafe { CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER) } {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let count: i32 = match unsafe { shell.Count() } {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut paths = Vec::with_capacity(count as usize);
    for i in 0..count {
        let index = make_vt_i4(i as i32);
        let disp: IDispatch = match unsafe { shell.Item(&index) } {
            Ok(d) => d,
            Err(_) => continue,
        };

        let browser: IWebBrowserApp = match disp.cast() {
            Ok(b) => b,
            Err(_) => continue,
        };

        let url = match unsafe { browser.LocationURL() } {
            Ok(u) => u.to_string(),
            Err(_) => continue,
        };

        if let Some(path) = url_to_path(&url) {
            paths.push(path);
        }
    }

    paths
}

fn url_to_path(url: &str) -> Option<String> {
    if !url.starts_with("file:///") {
        return None;
    }
    let raw = url.strip_prefix("file:///")?;
    let decoded = urlencoding::decode(raw).ok()?;
    Some(decoded.replace('/', "\\"))
}
