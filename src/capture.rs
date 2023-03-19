use std::mem::size_of;
use std::time::Instant;
use windows::Win32::Foundation::{ERROR_INVALID_PARAMETER, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, StretchBlt, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC,
    SRCCOPY,
};
use windows::Win32::Storage::Xps::{PrintWindow, PRINT_WINDOW_FLAGS, PW_CLIENTONLY};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetSystemMetrics, GetWindowRect, PW_RENDERFULLCONTENT, SM_CXVIRTUALSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};
use windows::core::HRESULT;

use crate::wrappers::{Hdc, Rect, CreatedHdc, Hbitmap};

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

pub enum Area {
    Full,
    ClientOnly,
}

#[derive(Debug)]
pub struct RgbBuf {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub fn capture_window(hwnd: isize) -> Result<RgbBuf, windows::core::Error> {
    capture_window_ex(hwnd, Area::Full, None)
}

pub fn capture_window_ex(
    hwnd: isize,
    area: Area,
    crop: Option<[i32; 4]>,
) -> Result<RgbBuf, windows::core::Error> {
    let hwnd = HWND(hwnd);

    unsafe {
        let hdc_screen = Hdc::get_dc(hwnd)?;

        let rect = match area {
            Area::Full => Rect::get_window_rect(hwnd),
            Area::ClientOnly => Rect::get_client_rect(hwnd),
        }?;

        let hdc = CreatedHdc::create_compatible_dc(hdc_screen.hdc)?;
        let hbmp = Hbitmap::create_compatible_bitmap(hdc_screen.hdc, rect.width, rect.height)?;
        if SelectObject(hdc.hdc, hbmp.hbitmap).is_invalid() {
            return Err(windows::core::Error::from_win32());
        }

        let flags = match area {
            Area::Full => PRINT_WINDOW_FLAGS(PW_RENDERFULLCONTENT),
            Area::ClientOnly => PRINT_WINDOW_FLAGS(PW_CLIENTONLY.0 | PW_RENDERFULLCONTENT),
        };
        if PrintWindow(hwnd, hdc.hdc, flags) == false {
            return Err(windows::core::Error::from_win32());
        }
        
        let (w, h, hdc, hbmp) = match crop {
            Some(crop) => {
                let [x, y, w, h] = crop;
                let hdc2 = CreatedHdc::create_compatible_dc(hdc.hdc)?;
                let hbmp2 = Hbitmap::create_compatible_bitmap(hdc.hdc, w, h)?;
                let so = SelectObject(hdc2.hdc, hbmp2.hbitmap);
                if so.is_invalid() {
                    return Err(windows::core::Error::from_win32());
                }
                if BitBlt(hdc2.hdc, 0, 0, w, h, hdc.hdc, x, y, SRCCOPY) == false {
                    return Err(windows::core::Error::from_win32());
                }
                if SelectObject(hdc2.hdc, so).is_invalid() {
                    return Err(windows::core::Error::from_win32());
                }
                (w, h, hdc2, hbmp2)
            }
            None => (rect.width, rect.height, hdc, hbmp),
        };
        let bmih = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biPlanes: 1,
            biBitCount: 24,
            biWidth: w,
            biHeight: -h,
            biCompression: BI_RGB,
            ..Default::default()
        };
        let mut bmi = BITMAPINFO {
            bmiHeader: bmih,
            ..Default::default()
        };
        let mut buf: Vec<u8> = vec![0; (3 * w * h) as usize];
        let gdb = GetDIBits(
            hdc.hdc,
            hbmp.hbitmap,
            0,
            h as u32,
            Some(buf.as_mut_ptr() as *mut core::ffi::c_void),
            &mut bmi,
            DIB_RGB_COLORS,
        );
        if gdb == 0 || gdb == ERROR_INVALID_PARAMETER.0 as i32 {
            return Err(windows::core::Error::new(HRESULT(555), "GetDIBits error".into()));
        }
        buf.chunks_exact_mut(3).for_each(|c| c.swap(0, 2));
        Ok(RgbBuf {
            pixels: buf,
            width: w as u32,
            height: h as u32,
        })
    }
}

pub fn capture_display() -> Result<RgbBuf, WSError> {
    unsafe {
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
            biBitCount: 24,
            biWidth: width,
            biHeight: -height,
            biCompression: BI_RGB,
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

        buf.chunks_exact_mut(3).for_each(|c| c.swap(0, 2));

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
