use std::ptr::null_mut;
use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
    SetupDiGetDeviceInstanceIdW, SetupDiGetDeviceInterfaceDetailW, DIGCF_DEVICEINTERFACE,
    DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA, SP_DEVINFO_DATA,
};
use windows_sys::Win32::Devices::Usb::{
    GUID_DEVINTERFACE_USB_DEVICE, WinUsb_AbortPipe, WinUsb_Free, WinUsb_Initialize,
    WinUsb_QueryPipe, WinUsb_ReadPipe, WinUsb_WritePipe, WINUSB_INTERFACE_HANDLE,
    WINUSB_PIPE_INFORMATION,
};
use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::IO::{GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, FILE_READ_DATA, FILE_SHARE_READ, FILE_SHARE_WRITE,
    FILE_WRITE_DATA, OPEN_EXISTING,
};

use crate::error::{Error, Result};

pub const VID: u16 = 0x138a;
pub const PID: u16 = 0x00ab;

const EP_OUT: u8 = 0x01;
const EP_IN: u8 = 0x81; // command responses
const EP_82: u8 = 0x82; // image / calibration data
const EP_INT: u8 = 0x83; // interrupt events

const CMD_TIMEOUT_MS: u32 = 15000;
const INT_TIMEOUT_MS: u32 = 100;
const READ_82_TIMEOUT_MS: u32 = 10000;

const ERROR_IO_PENDING: i32 = 997;

fn last_error(what: &str) -> Error {
    Error::Usb(format!(
        "{} (win32 error {})",
        what,
        std::io::Error::last_os_error()
    ))
}

/// Find the device interface path for our sensor (WinUSB-bound).
fn find_device_path() -> Result<String> {
    let guid = GUID_DEVINTERFACE_USB_DEVICE;
    let devs = unsafe {
        SetupDiGetClassDevsW(
            &guid,
            null_mut(),
            null_mut(),
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
    };
    if devs == 0 {
        return Err(last_error("SetupDiGetClassDevsW"));
    }

    let mut found = None;
    let mut idx: u32 = 0;
    loop {
        let mut iface = SP_DEVICE_INTERFACE_DATA {
            cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            ..Default::default()
        };
        let ok =
            unsafe { SetupDiEnumDeviceInterfaces(devs, null_mut(), &guid, idx, &mut iface) };
        if ok == 0 {
            break;
        }
        idx += 1;

        // size query
        let mut need: u32 = 0;
        let _ = unsafe {
            SetupDiGetDeviceInterfaceDetailW(devs, &iface, null_mut(), 0, &mut need, null_mut())
        };
        let mut buf = vec![0u8; need as usize];
        let mut dev_info = SP_DEVINFO_DATA {
            cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };
        let ok = unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                devs,
                &iface,
                buf.as_mut_ptr() as _,
                need,
                null_mut(),
                &mut dev_info,
            )
        };
        if ok == 0 {
            continue;
        }
        // device path is a wide string at the start of the buffer
        let path: String = unsafe {
            let p = buf.as_ptr() as *const u16;
            let mut end = 0usize;
            while p.add(end).read() != 0 {
                end += 1;
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(p, end))
        };

        // match by instance id: USB\VID_138A&PID_00AB\...
        let mut inst = [0u16; 256];
        let len = unsafe {
            SetupDiGetDeviceInstanceIdW(
                devs,
                &dev_info,
                inst.as_mut_ptr(),
                inst.len() as u32,
                null_mut(),
            )
        };
        if len != 0 {
            let mut end = 0usize;
            while end < inst.len() && inst[end] != 0 {
                end += 1;
            }
            let id = String::from_utf16_lossy(&inst[..end]);
            let want = format!("VID_{:04X}&PID_{:04X}", VID, PID);
            if id.to_ascii_uppercase().contains(&want) {
                found = Some(path);
                break;
            }
        }
    }
    unsafe { SetupDiDestroyDeviceInfoList(devs) };

    found.ok_or_else(|| {
        Error::Other(format!(
            "Fingerprint sensor USB\\VID_{:04X}&PID_{:04X} not found. \
             Is it bound to WinUSB? Run scripts\\rebind.ps1 as Administrator first.",
            VID, PID
        ))
    })
}

struct OverlappedOp {
    ol: OVERLAPPED,
    event: windows_sys::Win32::Foundation::HANDLE,
}

impl OverlappedOp {
    fn new() -> Result<OverlappedOp> {
        let event = unsafe { CreateEventW(null_mut(), 1, 0, null_mut()) };
        if event.is_null() {
            return Err(last_error("CreateEventW"));
        }
        let mut ol: OVERLAPPED = unsafe { std::mem::zeroed() };
        ol.hEvent = event;
        Ok(OverlappedOp { ol, event })
    }

    /// Wait for completion, returns bytes transferred.
    fn wait(&mut self, file: windows_sys::Win32::Foundation::HANDLE, timeout_ms: u32) -> Result<u32> {
        let wait = unsafe { WaitForSingleObject(self.event, timeout_ms) };
        if wait != 0 {
            return Err(Error::Usb(format!("WinUSB transfer timed out (wait={})", wait)));
        }
        let mut got: u32 = 0;
        let ok = unsafe { GetOverlappedResult(file as _, &mut self.ol, &mut got, 0) };
        if ok == 0 {
            return Err(last_error("GetOverlappedResult"));
        }
        Ok(got)
    }
}

impl Drop for OverlappedOp {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.event) };
    }
}

pub struct Device {
    file: windows_sys::Win32::Foundation::HANDLE,
    winusb: WINUSB_INTERFACE_HANDLE,
    max_in_packet: u32,
}

impl Device {
    pub fn open() -> Result<Device> {
        let path = find_device_path()?;
        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let file = unsafe {
            CreateFileW(
                wide.as_ptr(),
                GENERIC_READ | GENERIC_WRITE | FILE_READ_DATA | FILE_WRITE_DATA,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                null_mut(),
            )
        };
        if file == INVALID_HANDLE_VALUE {
            return Err(last_error("CreateFileW on sensor device path"));
        }
        let mut winusb: WINUSB_INTERFACE_HANDLE = null_mut();
        let ok = unsafe { WinUsb_Initialize(file as _, &mut winusb) };
        if ok == 0 {
            unsafe { CloseHandle(file) };
            return Err(last_error("WinUsb_Initialize"));
        }

        // Query pipe info for EP 0x81 to learn the max packet size
        // (needed to detect short-packet termination of responses).
        let mut max_in_packet: u32 = 64;
        for i in 0..8u8 {
            let mut pipe_info: WINUSB_PIPE_INFORMATION = unsafe { std::mem::zeroed() };
            let ok = unsafe { WinUsb_QueryPipe(winusb, 0, i, &mut pipe_info) };
            if ok != 0 && pipe_info.PipeId == EP_IN {
                max_in_packet = pipe_info.MaximumPacketSize as u32;
                break;
            }
        }

        Ok(Device {
            file,
            winusb,
            max_in_packet,
        })
    }

    fn read_pipe(
        &self,
        pipe: u8,
        buf: &mut [u8],
        timeout_ms: u32,
    ) -> Result<u32> {
        let mut op = OverlappedOp::new()?;
        let mut transferred: u32 = 0;
        let ok = unsafe {
            WinUsb_ReadPipe(
                self.winusb,
                pipe,
                buf.as_mut_ptr(),
                buf.len() as u32,
                &mut transferred,
                &mut op.ol,
            )
        };
        if ok == 0 {
            let err = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            if err != ERROR_IO_PENDING {
                return Err(Error::Usb(format!(
                    "WinUsb_ReadPipe(0x{:02x}) failed (win32 error {})",
                    pipe, err
                )));
            }
        }
        match op.wait(self.file, timeout_ms) {
            Ok(got) => Ok(got),
            Err(e) => {
                // Abort the endpoint so a timed-out read does not block the next one.
                let _ = unsafe { WinUsb_AbortPipe(self.winusb, pipe) };
                Err(e)
            }
        }
    }

    /// Write a command to EP 0x01 and read the response from EP 0x81.
    /// Reading mirrors libusb semantics: stop at a short packet / ZLP.
    pub fn cmd(&self, out: &[u8]) -> Result<Vec<u8>> {
        let mut op = OverlappedOp::new()?;
        let mut written: u32 = 0;
        let mut tmp = out.to_vec();
        let ok = unsafe {
            WinUsb_WritePipe(
                self.winusb,
                EP_OUT,
                tmp.as_mut_ptr(),
                tmp.len() as u32,
                &mut written,
                &mut op.ol,
            )
        };
        if ok == 0 {
            let err = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
            if err != ERROR_IO_PENDING {
                return Err(Error::Usb(format!(
                    "WinUsb_WritePipe failed (win32 error {})",
                    err
                )));
            }
        }
        let _ = op.wait(self.file, CMD_TIMEOUT_MS)?;

        let mut resp = Vec::new();
        loop {
            let mut chunk = vec![0u8; 64 * 1024];
            let got = self.read_pipe(EP_IN, &mut chunk, CMD_TIMEOUT_MS)?;
            resp.extend_from_slice(&chunk[..got as usize]);
            if got == 0 || got < chunk.len() as u32 && got % self.max_in_packet != 0 {
                break;
            }
            if resp.len() >= 100 * 1024 {
                break;
            }
        }
        Ok(resp)
    }

    /// Read raw data (images / calibration) from EP 0x82.
    pub fn read_82(&self) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        loop {
            let mut chunk = vec![0u8; 64 * 1024];
            let got = self.read_pipe(EP_82, &mut chunk, READ_82_TIMEOUT_MS)?;
            data.extend_from_slice(&chunk[..got as usize]);
            if got == 0 || got < chunk.len() as u32 && got % self.max_in_packet != 0 {
                break;
            }
            if data.len() >= 1024 * 1024 {
                break;
            }
        }
        Ok(data)
    }

    /// Wait for one interrupt event packet from EP 0x83 (100 ms poll).
    pub fn wait_int(&self) -> Result<Vec<u8>> {
        loop {
            let mut chunk = vec![0u8; 1024];
            match self.read_pipe(EP_INT, &mut chunk, INT_TIMEOUT_MS) {
                Ok(got) => {
                    chunk.truncate(got as usize);
                    return Ok(chunk);
                }
                Err(Error::Usb(s)) if s.contains("timed out") => continue,
                Err(e) => return Err(e),
            }
        }
    }

    pub fn close(&mut self) {
        unsafe {
            let _ = WinUsb_Free(self.winusb);
            let _ = CloseHandle(self.file);
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            let _ = WinUsb_Free(self.winusb);
            let _ = CloseHandle(self.file);
        }
    }
}