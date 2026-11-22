use std::collections::HashSet;
use std::ffi::c_void;
use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleBitmap, CreateCompatibleDC,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDIBits, HBITMAP, HDC, HGDIOBJ, SelectObject,
};
use windows_sys::Win32::Storage::Xps::PrintWindow;
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible, PW_RENDERFULLCONTENT,
};

#[path = "../../../src/platform/png.rs"]
mod png;

struct WindowSearch {
    process_id: u32,
    title: &'static str,
    matches: Vec<HWND>,
}

unsafe extern "system" fn enumerate_window(hwnd: HWND, parameter: LPARAM) -> BOOL {
    let search = unsafe { &mut *(parameter as *mut WindowSearch) };
    if unsafe { IsWindowVisible(hwnd) } == 0 {
        return 1;
    }

    let mut process_id = 0;
    unsafe { GetWindowThreadProcessId(hwnd, &mut process_id) };
    if process_id != search.process_id {
        return 1;
    }

    let mut title = [0u16; 256];
    let length = unsafe { GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32) };
    if length > 0 && String::from_utf16_lossy(&title[..length as usize]) == search.title {
        search.matches.push(hwnd);
    }
    1
}

fn find_unique_app_window(title: &'static str) -> Result<HWND, String> {
    let mut search = WindowSearch {
        process_id: unsafe { GetCurrentProcessId() },
        title,
        matches: Vec::new(),
    };
    let parameter = &mut search as *mut WindowSearch as LPARAM;
    if unsafe { EnumWindows(Some(enumerate_window), parameter) } == 0 {
        return Err("EnumWindows failed while locating the GPUI top-level window".into());
    }
    match search.matches.as_slice() {
        [hwnd] => Ok(*hwnd),
        [] => Err(format!(
            "no visible top-level window titled {title:?} belongs to this process"
        )),
        _ => Err(format!(
            "expected one visible GPUI top-level window titled {title:?}, found {}",
            search.matches.len()
        )),
    }
}

struct CaptureDc {
    screen: HDC,
    memory: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    selected: bool,
}

impl CaptureDc {
    unsafe fn new(width: i32, height: i32) -> Result<Self, String> {
        let screen = unsafe { windows_sys::Win32::Graphics::Gdi::GetDC(null_mut()) };
        if screen.is_null() {
            return Err("GetDC failed".into());
        }
        let memory = unsafe { CreateCompatibleDC(screen) };
        if memory.is_null() {
            unsafe { windows_sys::Win32::Graphics::Gdi::ReleaseDC(null_mut(), screen) };
            return Err("CreateCompatibleDC failed".into());
        }
        let bitmap = unsafe { CreateCompatibleBitmap(screen, width, height) };
        if bitmap.is_null() {
            unsafe {
                DeleteDC(memory);
                windows_sys::Win32::Graphics::Gdi::ReleaseDC(null_mut(), screen);
            }
            return Err("CreateCompatibleBitmap failed".into());
        }
        let previous = unsafe { SelectObject(memory, bitmap as HGDIOBJ) };
        if previous.is_null() || previous as isize == -1 {
            unsafe {
                DeleteObject(bitmap as HGDIOBJ);
                DeleteDC(memory);
                windows_sys::Win32::Graphics::Gdi::ReleaseDC(null_mut(), screen);
            }
            return Err("SelectObject failed for the capture bitmap".into());
        }
        Ok(Self {
            screen,
            memory,
            bitmap,
            previous,
            selected: true,
        })
    }

    unsafe fn deselect_bitmap(&mut self) {
        if self.selected {
            unsafe { SelectObject(self.memory, self.previous) };
            self.selected = false;
        }
    }

    unsafe fn pixels(&self, width: i32, height: i32) -> Result<Vec<u8>, String> {
        let mut info: BITMAPINFO = unsafe { std::mem::zeroed() };
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let lines = unsafe {
            GetDIBits(
                self.memory,
                self.bitmap,
                0,
                height as u32,
                pixels.as_mut_ptr().cast::<c_void>(),
                &mut info,
                DIB_RGB_COLORS,
            )
        };
        if lines != height {
            return Err(format!("GetDIBits returned {lines} of {height} rows"));
        }
        Ok(pixels)
    }
}

impl Drop for CaptureDc {
    fn drop(&mut self) {
        unsafe {
            self.deselect_bitmap();
            DeleteObject(self.bitmap as HGDIOBJ);
            DeleteDC(self.memory);
            windows_sys::Win32::Graphics::Gdi::ReleaseDC(null_mut(), self.screen);
        }
    }
}

pub fn capture_current_process_window_png(path: &str, title: &'static str) -> Result<(), String> {
    let hwnd = find_unique_app_window(title)?;
    let (width, height, pixels) = capture_window_pixels(hwnd)?;
    png::write_bgra(path, width, height, &pixels).map_err(|error| {
        format!(
            "could not write PNG to {}: {error}",
            std::path::Path::new(path).display()
        )
    })
}

fn capture_window_pixels(hwnd: HWND) -> Result<(u32, u32, Vec<u8>), String> {
    if unsafe { IsIconic(hwnd) } != 0 {
        return Err("the GPUI window is minimized".into());
    }
    let mut rect: RECT = unsafe { std::mem::zeroed() };
    if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
        return Err("GetWindowRect failed for the GPUI window".into());
    }
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width <= 0 || height <= 0 {
        return Err("the GPUI window has invalid dimensions".into());
    }
    let pixel_count = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| "the GPUI window dimensions overflow".to_owned())?;
    if pixel_count > 32_000_000 {
        return Err(format!(
            "refusing an unusually large capture ({width} x {height})"
        ));
    }

    let mut dc = unsafe { CaptureDc::new(width, height)? };
    if unsafe { PrintWindow(hwnd, dc.memory, PW_RENDERFULLCONTENT) } == 0 {
        return Err("the compositor could not capture the GPUI window".into());
    }
    unsafe { dc.deselect_bitmap() };
    // Never fall back to scraping the composed desktop: a covering window
    // could otherwise be included in the calculator's screenshot.
    let pixels = unsafe { dc.pixels(width, height)? };
    validate_pixels(width as u32, height as u32, &pixels)?;
    Ok((width as u32, height as u32, pixels))
}

fn validate_pixels(width: u32, height: u32, pixels: &[u8]) -> Result<(), String> {
    let Some(pixel_count) = (width as usize).checked_mul(height as usize) else {
        return Err("pixel dimensions overflow".into());
    };
    if pixel_count < 4 || pixels.len() != pixel_count.saturating_mul(4) {
        return Err("capture buffer has invalid dimensions".into());
    }

    let step = (pixel_count / 16_384).max(1);
    let mut distinct = HashSet::new();
    for index in (0..pixel_count).step_by(step) {
        let pixel = &pixels[index * 4..index * 4 + 3];
        distinct.insert([pixel[0], pixel[1], pixel[2]]);
        if distinct.len() >= 4 {
            return Ok(());
        }
    }
    Err("captured pixels are blank or nearly uniform".into())
}

#[cfg(test)]
mod tests {
    use super::validate_pixels;

    #[test]
    fn pixel_validation_rejects_uniform_frames_and_accepts_visible_detail() {
        assert!(validate_pixels(4, 4, &[0; 4 * 4 * 4]).is_err());
        let mut pixels = vec![0; 4 * 4 * 4];
        for (index, pixel) in pixels.chunks_exact_mut(4).enumerate() {
            pixel[0] = (index * 16) as u8;
            pixel[1] = (index * 8) as u8;
            pixel[2] = (index * 4) as u8;
            pixel[3] = 255;
        }
        assert!(validate_pixels(4, 4, &pixels).is_ok());
    }
}
