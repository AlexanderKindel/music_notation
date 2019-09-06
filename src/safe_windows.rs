use winapi::shared::windef::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

pub fn button_is_checked(window_handle: HWND) -> bool
{
    unsafe
    {
        SendMessageW(window_handle, BM_GETCHECK, 0, 0) == BST_CHECKED as isize
    }
}

pub fn get_dc(window_handle: HWND) -> HDC
{
    unsafe
    {
        GetDC(window_handle)
    }
}

pub fn get_pixel(device_context: HDC, x: i32, y: i32) -> COLORREF
{
    unsafe
    {
        GetPixel(device_context, x, y)
    }
}

pub fn release_dc(window_handle: HWND, device_context: HDC) -> i32
{
    unsafe
    {
        ReleaseDC(window_handle, device_context)
    }
}