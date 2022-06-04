use image::imageops::flip_vertical;
use image::{ImageBuffer, Rgba};
use std::mem::size_of;
use std::ptr::null_mut;
use winapi::shared::windef::*;
use winapi::shared::winerror::ERROR_INVALID_PARAMETER;
use winapi::um::wingdi::SRCCOPY;
use winapi::um::wingdi::*;
use winapi::um::winuser::GetSystemMetrics;
use winapi::um::winuser::*;

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
}

pub type Image = ImageBuffer<Rgba<u8>, Vec<u8>>;

pub fn capture_window(hwnd: usize) -> Result<Image, WSError> {
    const PW_RENDERFULLCONTENT: u32 = 2;

    unsafe {
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let rc: LPRECT = &mut rect;

        let hdc_screen = GetDC(hwnd as HWND);
        if hdc_screen.is_null() {
            return Err(WSError::GetDCIsNull);
        }

        let get_cr = GetWindowRect(hwnd as HWND, rc);
        if get_cr == 0 {
            ReleaseDC(null_mut(), hdc_screen);
            return Err(WSError::GetClientRectIsZero);
        }

        let width = (*rc).right - (*rc).left;
        let height = (*rc).bottom - (*rc).top;

        let hdc = CreateCompatibleDC(hdc_screen);
        if hdc.is_null() {
            ReleaseDC(null_mut(), hdc_screen);
            return Err(WSError::CreateCompatibleDCIsNull);
        }

        let hbmp = CreateCompatibleBitmap(hdc_screen, width, height);
        if hbmp.is_null() {
            DeleteDC(hdc);
            ReleaseDC(null_mut(), hdc_screen);
            return Err(WSError::CreateCompatibleBitmapIsNull);
        }

        let so = SelectObject(hdc, hbmp as HGDIOBJ);
        if so == HGDI_ERROR || so.is_null() {
            DeleteDC(hdc);
            DeleteObject(hbmp as HGDIOBJ);
            ReleaseDC(null_mut(), hdc_screen);
            return Err(WSError::SelectObjectError);
        }

        let bmih = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biPlanes: 1,
            biBitCount: 32,
            biWidth: width,
            biHeight: height,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };

        let mut bmi = BITMAPINFO {
            bmiHeader: bmih,
            ..Default::default()
        };

        let mut buf: Vec<u8> = vec![0; 4 * width as usize * height as usize];

        let pw = PrintWindow(hwnd as HWND, hdc, PW_RENDERFULLCONTENT);
        if pw == 0 {
            DeleteDC(hdc);
            DeleteObject(hbmp as HGDIOBJ);
            ReleaseDC(null_mut(), hdc_screen);
            return Err(WSError::PrintWindowIsZero);
        }

        let gdb = GetDIBits(
            hdc,
            hbmp,
            0,
            height as u32,
            buf.as_mut_ptr() as *mut winapi::ctypes::c_void,
            &mut bmi,
            DIB_RGB_COLORS,
        );
        if gdb == 0 || gdb == ERROR_INVALID_PARAMETER as i32 {
            DeleteDC(hdc);
            DeleteObject(hbmp as HGDIOBJ);
            ReleaseDC(null_mut(), hdc_screen);
            return Err(WSError::GetDIBitsError);
        }

        buf.chunks_exact_mut(4).for_each(|c| c.swap(0, 2));

        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_raw(width as u32, height as u32, buf).unwrap();

        DeleteDC(hdc);
        DeleteObject(hbmp as HGDIOBJ);
        ReleaseDC(null_mut(), hdc_screen);

        Ok(flip_vertical(&img))
    }
}

pub fn capture_display() -> Result<Image, WSError> {
    unsafe {
        let hdc_screen = GetDC(null_mut());
        if hdc_screen.is_null() {
            return Err(WSError::GetDCIsNull);
        }

        let hdc = CreateCompatibleDC(hdc_screen);
        if hdc.is_null() {
            ReleaseDC(null_mut(), hdc_screen);
            return Err(WSError::CreateCompatibleDCIsNull);
        }

        let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        let hbmp = CreateCompatibleBitmap(hdc_screen, width, height);
        if hbmp.is_null() {
            DeleteDC(hdc);
            ReleaseDC(null_mut(), hdc_screen);
            return Err(WSError::CreateCompatibleBitmapIsNull);
        }

        let so = SelectObject(hdc, hbmp as HGDIOBJ);
        if so == HGDI_ERROR || so.is_null() {
            DeleteDC(hdc);
            DeleteObject(hbmp as HGDIOBJ);
            ReleaseDC(null_mut(), hdc_screen);
            return Err(WSError::SelectObjectError);
        }

        let sb = StretchBlt(
            hdc, 0, 0, width, height, hdc_screen, x, y, width, height, SRCCOPY,
        );
        if sb == 0 {
            DeleteDC(hdc);
            DeleteObject(hbmp as HGDIOBJ);
            ReleaseDC(null_mut(), hdc_screen);
            return Err(WSError::StretchBltIsZero);
        }

        let bmih = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biPlanes: 1,
            biBitCount: 32,
            biWidth: width,
            biHeight: height,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };

        let mut bmi = BITMAPINFO {
            bmiHeader: bmih,
            ..Default::default()
        };

        let mut buf: Vec<u8> = vec![0; 4 * width as usize * height as usize];

        let gdb = GetDIBits(
            hdc,
            hbmp,
            0,
            height as u32,
            buf.as_mut_ptr() as *mut winapi::ctypes::c_void,
            &mut bmi,
            DIB_RGB_COLORS,
        );
        if gdb == 0 || gdb == ERROR_INVALID_PARAMETER as i32 {
            DeleteDC(hdc);
            DeleteObject(hbmp as HGDIOBJ);
            ReleaseDC(null_mut(), hdc_screen);
            return Err(WSError::GetDIBitsError);
        }

        buf.chunks_exact_mut(4).for_each(|c| c.swap(0, 2));

        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_raw(width as u32, height as u32, buf).unwrap();

        DeleteDC(hdc);
        DeleteDC(hdc_screen);
        ReleaseDC(null_mut(), hdc_screen);

        Ok(flip_vertical(&img))
    }
}
