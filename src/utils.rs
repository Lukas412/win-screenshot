use std::ffi::OsString;
use std::os::windows::ffi::OsStrExt;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowW, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible,
};

#[derive(Debug)]
pub struct HwndName {
    pub hwnd: isize,
    pub window_name: String,
}

#[derive(Debug)]
pub enum FWError {
    NotFoundOrFault,
}

pub fn find_window(window_name: &str) -> Result<isize, FWError> {
    unsafe {
        let w = FindWindowW(
            PCWSTR::null(),
            PCWSTR(
                OsString::from(window_name)
                    .encode_wide()
                    .chain(Some(0))
                    .collect::<Vec<_>>()
                    .as_ptr(),
            ),
        );
        match w {
            HWND(0) => Err(FWError::NotFoundOrFault),
            HWND(p) => Ok(p),
        }
    }
}

unsafe extern "system" fn wl_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let vec = lparam.0 as *mut Vec<HwndName>;

    if IsWindowVisible(hwnd) == false {
        return BOOL::from(true);
    }

    let gwtl = GetWindowTextLengthW(hwnd);
    if gwtl == 0 {
        return BOOL::from(true);
    }

    let mut name_buf: Vec<u16> = vec![0; (gwtl + 1) as usize];

    let gwt = GetWindowTextW(hwnd, &mut name_buf);
    if gwt == 0 {
        return BOOL::from(true);
    }

    let name_buf = match name_buf.split_last() {
        Some((_, last)) => last,
        None => return BOOL::from(true),
    };

    let name = String::from_utf16_lossy(name_buf);

    (*vec).push(HwndName {
        hwnd: hwnd.0,
        window_name: name,
    });

    BOOL::from(true)
}

#[derive(Debug)]
pub enum WLError {
    EnumWindowsError,
}

pub fn window_list() -> Result<Vec<HwndName>, WLError> {
    let mut hwnd_name = Vec::new();
    unsafe {
        let ew = EnumWindows(
            Some(wl_callback),
            LPARAM(&mut hwnd_name as *mut Vec<HwndName> as isize),
        );
        if ew == false {
            return Err(WLError::EnumWindowsError);
        }
    }
    Ok(hwnd_name)
}
