use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

pub trait DeviceContextObject
{
    fn to_pointer(&self) -> *mut winapi::ctypes::c_void;
}

impl DeviceContextObject for HFONT
{
    fn to_pointer(&self) -> *mut winapi::ctypes::c_void
    {
        *self as *mut winapi::ctypes::c_void
    }
}

impl DeviceContextObject for HBRUSH
{
    fn to_pointer(&self) -> *mut winapi::ctypes::c_void
    {
        *self as *mut winapi::ctypes::c_void
    }
}

impl DeviceContextObject for HGDIOBJ
{
    fn to_pointer(&self) -> *mut winapi::ctypes::c_void
    {
        *self as *mut winapi::ctypes::c_void
    }
}

impl DeviceContextObject for HPEN
{
    fn to_pointer(&self) -> *mut winapi::ctypes::c_void
    {
        *self as *mut winapi::ctypes::c_void
    }
}

pub fn button_is_checked(window_handle: HWND) -> bool
{
    unsafe
    {
        SendMessageW(window_handle, BM_GETCHECK, 0, 0) == BST_CHECKED as isize
    }
}

pub fn dispatch_message(message: &MSG)
{
    unsafe
    {
        DispatchMessageW(message);
    }
}

pub fn draw_rectangle(device_context: HDC, left: i32, top: i32, right: i32, bottom: i32)
{
    unsafe
    {
        Rectangle(device_context, left, top, right, bottom);
    }
}

pub fn draw_string(device_context: HDC, x: i32, y: i32, string: Vec<u16>)
{
    unsafe
    {
        TextOutW(device_context, x, y, string.as_ptr(), string.len() as i32);
    }
}

pub fn enable_window(window_handle: HWND, enable: BOOL)
{
    unsafe
    {
        EnableWindow(window_handle, enable);
    }
}

pub fn get_dc(window_handle: HWND) -> HDC
{
    unsafe
    {
        GetDC(window_handle)
    }
}

pub fn get_message(message: &mut MSG, window_handle: HWND) -> BOOL
{
    unsafe
    {
        GetMessageW(message, window_handle, 0, 0)
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

pub fn restore_dc(device_context: HDC)
{
    unsafe
    {
        RestoreDC(device_context, -1);
    }
}

pub fn save_dc(device_context: HDC)
{
    unsafe
    {
        SaveDC(device_context);
    }
}

pub fn select_object<T: DeviceContextObject>(device_context: HDC, object: T) -> HGDIOBJ
{
    unsafe
    {
        SelectObject(device_context, object.to_pointer())
    }
}

pub fn set_text_color(device_context: HDC, color: COLORREF)
{
    unsafe
    {
        SetTextColor(device_context, color);
    }
}

pub fn translate_message(message: &MSG)
{
    unsafe
    {
        TranslateMessage(message);
    }
}