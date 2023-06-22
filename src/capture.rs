use std::mem::size_of;
use windows::Win32::Foundation::{ERROR_INVALID_PARAMETER, E_FAIL, HWND};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, StretchBlt, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    SRCCOPY,
};
use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS, PW_CLIENTONLY};
use windows::Win32::UI::HiDpi::{SetProcessDpiAwareness, PROCESS_PER_MONITOR_DPI_AWARE};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, PW_RENDERFULLCONTENT, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

use crate::wrappers::{CreatedHdc, Hbitmap, Hdc, Rect};

#[derive(Debug)]
pub enum WSError {
    GetDCIsNull,
    GetClientRectIsZero,
    CreateCompatibleDCIsNull,
    CreateCompatibleBitmapIsNull,
    SelectObjectError,
    PrintWindowIsZero,
    GetDIBitsError,
    GetSystemMetricsIsZero,
    StretchBltIsZero,
    BitBltError,
}

#[derive(Clone, Copy)]
pub enum Area {
    Full,
    ClientOnly,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Using {
    BitBlt,
    PrintWindow,
}

#[derive(Debug)]
pub struct RgbBuf {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

pub fn capture_window(hwnd: isize) -> Result<RgbBuf, windows::core::Error> {
    capture_window_ex(hwnd, Using::PrintWindow, Area::Full, None, None)
}

pub fn capture_window_into_buffer(
    hwnd: isize,
    buffer: &mut Vec<u8>,
) -> Result<WindowSize, windows::core::Error> {
    capture_window_into_buffer_ex(hwnd, buffer, Using::PrintWindow, Area::Full, None, None)
}

pub fn capture_window_into_bgr_buffer(
    hwnd: isize,
    buffer: &mut Vec<u8>,
) -> Result<WindowSize, windows::core::Error> {
    buffer.clear();
    let hwnd = HWND(hwnd);

    unsafe {
        let _ = SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE);

        let hdc_screen = Hdc::get_dc(hwnd)?;

        // BitBlt support only ClientOnly
        let rect = Rect::get_window_rect(hwnd)?;

        let hdc = CreatedHdc::create_compatible_dc(hdc_screen.hdc)?;
        let hbmp = Hbitmap::create_compatible_bitmap(hdc_screen.hdc, rect.width, rect.height)?;

        if SelectObject(hdc.hdc, hbmp.hbitmap).is_invalid() {
            return Err(windows::core::Error::from_win32());
        }

        let flags = PRINT_WINDOW_FLAGS(PW_RENDERFULLCONTENT);

        if PrintWindow(hwnd, hdc.hdc, flags) == false {
            return Err(windows::core::Error::from_win32());
        }

        let bmih = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biPlanes: 1,
            biBitCount: 32,
            biWidth: rect.width,
            biHeight: -rect.height,
            biCompression: BI_RGB.0 as u32,
            ..Default::default()
        };
        let mut bmi = BITMAPINFO {
            bmiHeader: bmih,
            ..Default::default()
        };
        buffer.reserve((4 * rect.width * rect.height) as usize);
        let gdb = GetDIBits(
            hdc.hdc,
            hbmp.hbitmap,
            0,
            rect.height as u32,
            Some(buffer.as_mut_ptr() as *mut core::ffi::c_void),
            &mut bmi,
            DIB_RGB_COLORS,
        );
        if gdb == 0 || gdb == ERROR_INVALID_PARAMETER.0 as i32 {
            return Err(windows::core::Error::new(E_FAIL, "GetDIBits error".into()));
        }
        Ok(WindowSize {
            width: rect.width as u32,
            height: rect.height as u32,
        })
    }
}
pub fn capture_window_ex(
    hwnd: isize,
    using: Using,
    area: Area,
    crop_xy: Option<[i32; 2]>,
    crop_wh: Option<[i32; 2]>,
) -> Result<RgbBuf, windows::core::Error> {
    let mut buffer = vec![];
    let WindowSize { width, height } =
        capture_window_into_buffer_ex(hwnd, &mut buffer, using, area, crop_xy, crop_wh)?;
    Ok(RgbBuf {
        pixels: buffer,
        width,
        height,
    })
}

pub fn capture_window_into_buffer_ex(
    hwnd: isize,
    buffer: &mut Vec<u8>,
    using: Using,
    area: Area,
    crop_xy: Option<[i32; 2]>,
    crop_wh: Option<[i32; 2]>,
) -> Result<WindowSize, windows::core::Error> {
    let result = capture_window_into_bgr_buffer_ex(hwnd, buffer, using, area, crop_xy, crop_wh)?;
    buffer.chunks_exact_mut(4).for_each(|c| c.swap(0, 2));
    Ok(result)
}

pub fn capture_window_into_bgr_buffer_ex(
    hwnd: isize,
    buffer: &mut Vec<u8>,
    using: Using,
    area: Area,
    crop_xy: Option<[i32; 2]>,
    crop_wh: Option<[i32; 2]>,
) -> Result<WindowSize, windows::core::Error> {
    buffer.clear();
    let hwnd = HWND(hwnd);

    unsafe {
        #[allow(unused_must_use)]
        {
            SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE);
        }

        let hdc_screen = Hdc::get_dc(hwnd)?;

        // BitBlt support only ClientOnly
        let rect = match (using, area) {
            (Using::PrintWindow, Area::Full) => Rect::get_window_rect(hwnd),
            (Using::BitBlt, _) | (Using::PrintWindow, Area::ClientOnly) => {
                Rect::get_client_rect(hwnd)
            }
        }?;

        let [cx, cy] = crop_xy.unwrap_or([0, 0]);
        let [cw, ch] = crop_wh.unwrap_or([rect.width - cx, rect.height - cy]);
        let crop = crop_xy.is_some() || crop_wh.is_some();

        let hdc = CreatedHdc::create_compatible_dc(hdc_screen.hdc)?;
        let hbmp = match (crop, using) {
            (true, Using::BitBlt) => Hbitmap::create_compatible_bitmap(hdc_screen.hdc, cw, ch),
            (false, Using::BitBlt) | (_, Using::PrintWindow) => {
                Hbitmap::create_compatible_bitmap(hdc_screen.hdc, rect.width, rect.height)
            }
        }?;

        if SelectObject(hdc.hdc, hbmp.hbitmap).is_invalid() {
            return Err(windows::core::Error::from_win32());
        }

        let flags = PRINT_WINDOW_FLAGS(match area {
            Area::Full => PW_RENDERFULLCONTENT,
            Area::ClientOnly => PW_CLIENTONLY.0 | PW_RENDERFULLCONTENT,
        });

        match using {
            Using::BitBlt => {
                if BitBlt(hdc.hdc, 0, 0, cw, ch, hdc_screen.hdc, cx, cy, SRCCOPY) == false {
                    return Err(windows::core::Error::from_win32());
                }
            }
            Using::PrintWindow => {
                if PrintWindow(hwnd, hdc.hdc, flags) == false {
                    return Err(windows::core::Error::from_win32());
                }
            }
        }

        let (width, height, hdc, hbmp) = match (crop, using) {
            (true, Using::PrintWindow) => {
                let hdc2 = CreatedHdc::create_compatible_dc(hdc.hdc)?;
                let hbmp2 = Hbitmap::create_compatible_bitmap(hdc.hdc, cw, ch)?;
                let so = SelectObject(hdc2.hdc, hbmp2.hbitmap);
                if so.is_invalid() {
                    return Err(windows::core::Error::from_win32());
                }
                if BitBlt(hdc2.hdc, 0, 0, cw, ch, hdc.hdc, cx, cy, SRCCOPY) == false {
                    return Err(windows::core::Error::from_win32());
                }
                if SelectObject(hdc2.hdc, so).is_invalid() {
                    return Err(windows::core::Error::from_win32());
                }
                (cw, ch, hdc2, hbmp2)
            }
            (true, Using::BitBlt) => (cw, ch, hdc, hbmp),
            (false, _) => (rect.width, rect.height, hdc, hbmp),
        };

        let bmih = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biPlanes: 1,
            biBitCount: 32,
            biWidth: width,
            biHeight: -height,
            biCompression: BI_RGB.0 as u32,
            ..Default::default()
        };
        let mut bmi = BITMAPINFO {
            bmiHeader: bmih,
            ..Default::default()
        };
        buffer.reserve((4 * width * height) as usize);
        let gdb = GetDIBits(
            hdc.hdc,
            hbmp.hbitmap,
            0,
            height as u32,
            Some(buffer.as_mut_ptr() as *mut core::ffi::c_void),
            &mut bmi,
            DIB_RGB_COLORS,
        );
        if gdb == 0 || gdb == ERROR_INVALID_PARAMETER.0 as i32 {
            return Err(windows::core::Error::new(E_FAIL, "GetDIBits error".into()));
        }
        Ok(WindowSize {
            width: width as u32,
            height: height as u32,
        })
    }
}

pub fn capture_display() -> Result<RgbBuf, WSError> {
    unsafe {
        // win 8.1 temporary DPI aware
        #[allow(unused_must_use)]
        {
            SetProcessDpiAwareness(PROCESS_PER_MONITOR_DPI_AWARE);
        }
        // for win 10
        //SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let hdc_screen = GetDC(HWND::default());
        if hdc_screen.is_invalid() {
            return Err(WSError::GetDCIsNull);
        }

        let hdc = CreateCompatibleDC(hdc_screen);
        if hdc.is_invalid() {
            ReleaseDC(HWND::default(), hdc_screen);
            return Err(WSError::CreateCompatibleDCIsNull);
        }

        let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        let hbmp = CreateCompatibleBitmap(hdc_screen, width, height);
        if hbmp.is_invalid() {
            DeleteDC(hdc);
            ReleaseDC(HWND::default(), hdc_screen);
            return Err(WSError::CreateCompatibleBitmapIsNull);
        }

        let so = SelectObject(hdc, hbmp);
        if so.is_invalid() {
            DeleteDC(hdc);
            DeleteObject(hbmp);
            ReleaseDC(HWND::default(), hdc_screen);
            return Err(WSError::SelectObjectError);
        }

        let sb = StretchBlt(
            hdc, 0, 0, width, height, hdc_screen, x, y, width, height, SRCCOPY,
        );
        if sb == false {
            DeleteDC(hdc);
            DeleteObject(hbmp);
            ReleaseDC(HWND::default(), hdc_screen);
            return Err(WSError::StretchBltIsZero);
        }

        let bmih = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biPlanes: 1,
            biBitCount: 32,
            biWidth: width,
            biHeight: -height,
            biCompression: BI_RGB.0 as u32,
            ..Default::default()
        };

        let mut bmi = BITMAPINFO {
            bmiHeader: bmih,
            ..Default::default()
        };

        let mut buf: Vec<u8> = vec![0; (4 * width * height) as usize];

        let gdb = GetDIBits(
            hdc,
            hbmp,
            0,
            height as u32,
            Some(buf.as_mut_ptr() as *mut core::ffi::c_void),
            &mut bmi,
            DIB_RGB_COLORS,
        );
        if gdb == 0 || gdb == ERROR_INVALID_PARAMETER.0 as i32 {
            DeleteDC(hdc);
            DeleteObject(hbmp);
            ReleaseDC(HWND::default(), hdc_screen);
            return Err(WSError::GetDIBitsError);
        }

        buf.chunks_exact_mut(4).for_each(|c| c.swap(0, 2));

        DeleteDC(hdc);
        DeleteObject(hbmp);
        ReleaseDC(HWND::default(), hdc_screen);

        Ok(RgbBuf {
            pixels: buf,
            width: width as u32,
            height: height as u32,
        })
    }
}
