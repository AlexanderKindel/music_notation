extern crate num_rational;
extern crate winapi;

use std::collections::HashMap;
use std::ptr::null_mut;
use winapi::um::errhandlingapi::GetLastError;
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::shared::windowsx::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

const WHOLE_NOTE_WIDTH: i32 = 120;
const DURATION_RATIO: f32 = 0.61803399;

static mut GRAY_PEN: Option<HPEN> = None;
static mut GRAY_BRUSH: Option<HBRUSH> = None;
static mut RED_PEN: Option<HPEN> = None;
static mut RED_BRUSH: Option<HBRUSH> = None;

struct RhythmicPosition
{
    bar_number: u32,
    whole_notes_from_start_of_bar: num_rational::Ratio<u8>
}

trait StaffObject
{
    fn font_codepoint(&self) -> u16;
    fn rhythmic_position(&self) -> &RhythmicPosition;
    fn distance_from_staff_start(&self) -> i32;
    fn width(&self) -> i32;
    fn draw(&self, parent_staff: &Staff, device_context: HDC);
}

struct Clef
{
    font_codepoint: u16,
    rhythmic_position: RhythmicPosition,
    distance_from_staff_start: i32,
    width: i32,
    staff_spaces_of_baseline_above_bottom_line: u8
}

impl StaffObject for Clef
{
    fn font_codepoint(&self) -> u16
    {
        self.font_codepoint
    }
    fn rhythmic_position(&self) -> &RhythmicPosition
    {
        &self.rhythmic_position
    }
    fn distance_from_staff_start(&self) -> i32
    {
        self.distance_from_staff_start
    }
    fn width(&self) -> i32
    {
        self.width
    }
    fn draw(&self, parent_staff: &Staff, device_context: HDC)
    {
        unsafe
        {
            let staff_space_count = parent_staff.line_count as i32 - 1;
            TextOutW(device_context, parent_staff.left_edge + self.distance_from_staff_start +
                parent_staff.height / staff_space_count, parent_staff.bottom_line_y -
                (parent_staff.height * self.staff_spaces_of_baseline_above_bottom_line as i32) /
                staff_space_count, vec![self.font_codepoint, 0].as_ptr(), 1);
        }
    }
}

struct Staff
{
    line_count: u8,
    line_thickness: u8,
    left_edge: i32,
    bottom_line_y: i32,
    height: i32,
    contents: Vec<Box<StaffObject>>
}

impl Staff
{
    fn draw(&self, window_memory: *mut MainWindowMemory, device_context: HDC)
    {	    
        unsafe
        {			
            let space_count = self.line_count as i32 - 1;
            let line_thickness = self.line_thickness as i32;
            let bottom_line_bottom = self.bottom_line_y + line_thickness / 2;
            let mut staff_height_times_line_number = 0;	        
            let right_edge =
            if self.contents.len() == 0
            {
                self.left_edge + WHOLE_NOTE_WIDTH
            }
            else
            {
                let last_object = &self.contents[self.contents.len() - 1];
                self.left_edge + last_object.distance_from_staff_start() + last_object.width()
            };
            let original_device_context = SaveDC(device_context);
            SelectObject(device_context, GetStockObject(BLACK_PEN as i32));
            SelectObject(device_context, GetStockObject(BLACK_BRUSH as i32));
            for _ in 0..self.line_count
            {		    
                let current_line_bottom =
                    bottom_line_bottom - staff_height_times_line_number / space_count;
                Rectangle(device_context, self.left_edge, current_line_bottom - line_thickness,
                    right_edge, current_line_bottom);
                staff_height_times_line_number += self.height;
            }
            SelectObject(device_context, (*window_memory).sized_music_fonts.get(
                &self.height).unwrap().font as *mut winapi::ctypes::c_void);
            SetBkMode(device_context, TRANSPARENT as i32);
            SetTextAlign(device_context, TA_BASELINE);
            for staff_object in &self.contents
            {
                staff_object.draw(self, device_context);
            }	
            RestoreDC(device_context, original_device_context);			
        }
    }
}

struct ObjectAddress
{
    staff_index: u16,
    staff_contents_index: u32
}

struct SystemSlice
{
    rhythmic_position: RhythmicPosition,
    objects_at_position: Vec<ObjectAddress>
}

enum EntryCursor
{
    ActiveCursor(ObjectAddress),
    GhostCursor(ObjectAddress),
    None
}

struct SizedMusicFont
{
    font: HFONT,
    number_of_staves_with_size: u8 
}

struct MainWindowMemory
{
    sized_music_fonts: HashMap<i32, SizedMusicFont>,
    staves: Vec<Staff>,
    system_slices: Vec<SystemSlice>,
    entry_cursor: EntryCursor,
    add_staff_button_handle: HWND,
    add_clef_button_handle: HWND
}

fn wide_char_string(value: &str) -> Vec<u16>
{    
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(value).encode_wide().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn main_window_proc(window_handle: HWND, u_msg: UINT, w_param: WPARAM,
    l_param: LPARAM) -> LRESULT
{
    match u_msg
    {
        WM_COMMAND =>
        {
            if HIWORD(w_param as u32) == BN_CLICKED
            {
                let window_memory =
                    GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
                if l_param == (*window_memory).add_staff_button_handle as isize
                {
                    let bottom_line_y =
                    if (*window_memory).staves.len() == 0
                    {
                        90
                    }
                    else
                    {
                        (*window_memory).staves[(*window_memory).staves.len() - 1].bottom_line_y +
                            80
                    };
                    (*window_memory).staves.push(Staff{line_count: 5, line_thickness: 1,
                        left_edge: 20, bottom_line_y: bottom_line_y, height: 40,
                        contents: Vec::new()});
                    let mut client_rect: RECT = std::mem::uninitialized();
                    GetClientRect(window_handle, &mut client_rect);
                    InvalidateRect(window_handle, &client_rect, FALSE);
                    match (*window_memory).sized_music_fonts.get_mut(&40)
                    {
                        Some(sized_font) =>
                        {
                            sized_font.number_of_staves_with_size +=1;
                        }
                        None =>
                        {
                            (*window_memory).sized_music_fonts.insert(40, SizedMusicFont{font:
                                CreateFontW(-40, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                wide_char_string("Bravura").as_ptr()),
                                number_of_staves_with_size: 1});
                        }
                    };
                }
                else if l_param == (*window_memory).add_clef_button_handle as isize
                {
                    match (*window_memory).entry_cursor
                    {
                        EntryCursor::ActiveCursor(ref address) =>
                        {
                            let staff = &mut (*window_memory).staves[address.staff_index as usize];
                            if staff.contents.len() == 0
                            {
                                let device_context = GetDC(window_handle);
                                SelectObject(device_context, (*window_memory).sized_music_fonts.get(
                                    &staff.height).unwrap().font as *mut winapi::ctypes::c_void);
                                let mut abc_array: [ABC; 1] = std::mem::uninitialized();
                                GetCharABCWidthsW(device_context, 0xe050, 0xe051,
                                    abc_array.as_mut_ptr());
                                staff.contents.push(Box::new(Clef{font_codepoint: 0xe050,
                                    rhythmic_position: RhythmicPosition{bar_number: 0,
                                    whole_notes_from_start_of_bar: num_rational::Ratio::new(0, 1)},
                                    distance_from_staff_start: 0, width: abc_array[0].abcB as i32 +
                                    (2 * staff.height) / (staff.line_count as i32 - 1),
                                    staff_spaces_of_baseline_above_bottom_line: 1}));
                                let mut client_rect: RECT = std::mem::uninitialized();
                                GetClientRect(window_handle, &mut client_rect);
                                InvalidateRect(window_handle, &client_rect, TRUE);
                            }
                        }
                        _ => ()
                    }
                }
                0
            }
            else
            {
                DefWindowProcW(window_handle, u_msg, w_param, l_param)
            }
        },
        WM_LBUTTONDOWN =>
        {
            let window_memory =
                GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
            match (*window_memory).entry_cursor
            {
                EntryCursor::GhostCursor(ref address) =>
                {
                    (*window_memory).entry_cursor =
                        EntryCursor::ActiveCursor(ObjectAddress{staff_index: address.staff_index,
                        staff_contents_index: address.staff_contents_index});
                    let ref staff = (*window_memory).staves[address.staff_index as usize];
                    InvalidateRect(window_handle, &RECT{left: staff.left_edge,
                        top: staff.bottom_line_y - staff.height, right: WHOLE_NOTE_WIDTH,
                        bottom: staff.bottom_line_y}, FALSE);
                },
                _ => ()
            }
            0
        },
        WM_MOUSEMOVE =>
        {
            let window_memory =
                GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
            match (*window_memory).entry_cursor
            {			    
                EntryCursor::None =>
                {
                    let cursor_x = GET_X_LPARAM(l_param);
                    let cursor_y = GET_Y_LPARAM(l_param);
                    for staff_index in 0..(*window_memory).staves.len()
                    {
                        let staff = &(*window_memory).staves[staff_index];
                        if staff.left_edge <= cursor_x && cursor_x <= staff.left_edge +
                            WHOLE_NOTE_WIDTH && staff.bottom_line_y - staff.height <= cursor_y &&
                            cursor_y <= staff.bottom_line_y
                        {
                            (*window_memory).entry_cursor = EntryCursor::GhostCursor(ObjectAddress{
                                staff_index:staff_index as u16, staff_contents_index: 0});
                            InvalidateRect(window_handle, &RECT{left: staff.left_edge,
                                top: staff.bottom_line_y - staff.height, right: WHOLE_NOTE_WIDTH,
                                bottom: staff.bottom_line_y}, FALSE);
                            return 0;
                        }
                    }
                },
                EntryCursor::GhostCursor(ref address) =>
                {
                    let cursor_x = GET_X_LPARAM(l_param);
                    let cursor_y = GET_Y_LPARAM(l_param);
                    for staff_index in 0..(*window_memory).staves.len()
                    {
                        let staff = &(*window_memory).staves[staff_index];
                        if staff.left_edge <= cursor_x && cursor_x <= staff.left_edge +
                            WHOLE_NOTE_WIDTH && staff.bottom_line_y - staff.height <= cursor_y &&
                            cursor_y <= staff.bottom_line_y
                        {
                            (*window_memory).entry_cursor = EntryCursor::GhostCursor(ObjectAddress{
                                staff_index:staff_index as u16, staff_contents_index: 0});
                            InvalidateRect(window_handle, &RECT{left: staff.left_edge,
                                top: staff.bottom_line_y - staff.height, right: WHOLE_NOTE_WIDTH,
                                bottom: staff.bottom_line_y}, FALSE);
                            return 0;
                        }
                    }
                    let old_staff = &(*window_memory).staves[address.staff_index as usize];					
                    InvalidateRect(window_handle, &RECT{left: old_staff.left_edge,
                        top: old_staff.bottom_line_y - old_staff.height, right: WHOLE_NOTE_WIDTH,
                        bottom: old_staff.bottom_line_y}, TRUE);
                    (*window_memory).entry_cursor = EntryCursor::None;
                },
                EntryCursor::ActiveCursor(_) => ()
            }
            0
        }
        WM_PAINT =>
        {
            let window_memory =
                GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
            let mut ps: PAINTSTRUCT = std::mem::uninitialized();
            let device_context = BeginPaint(window_handle, &mut ps);						
            for staff in &(*window_memory).staves
            {
                staff.draw(window_memory, device_context);
            }
            match (*window_memory).entry_cursor
            {
                EntryCursor::GhostCursor(ref address) =>
                {
                    let original_pen = SelectObject(device_context,
                        GRAY_PEN.unwrap() as *mut winapi::ctypes::c_void);
                    let original_brush = SelectObject(device_context,
                        GRAY_BRUSH.unwrap() as *mut winapi::ctypes::c_void);
                    let ref staff = (*window_memory).staves[address.staff_index as usize];
                    Rectangle(device_context, staff.left_edge, staff.bottom_line_y - staff.height,
                        staff.left_edge + 1, staff.bottom_line_y);
                    SelectObject(device_context, original_pen);
                    SelectObject(device_context, original_brush);
                },
                EntryCursor::ActiveCursor(ref address) =>
                {
                    let original_pen = SelectObject(device_context,
                        RED_PEN.unwrap() as *mut winapi::ctypes::c_void);
                    let original_brush = SelectObject(device_context,
                        RED_BRUSH.unwrap() as *mut winapi::ctypes::c_void);
                    let ref staff = (*window_memory).staves[address.staff_index as usize];
                    Rectangle(device_context, staff.left_edge, staff.bottom_line_y - staff.height,
                        staff.left_edge + 1, staff.bottom_line_y);
                    SelectObject(device_context, original_pen);
                    SelectObject(device_context, original_brush);
                },
                _ => ()
            }
            EndPaint(window_handle, &mut ps);
            DefWindowProcW(window_handle, u_msg, w_param, l_param)
        },
        _ => DefWindowProcW(window_handle, u_msg, w_param, l_param)
    }    
}

fn main()
{
    unsafe
    {
        let gray = RGB(127, 127, 127);
        GRAY_PEN = Some(CreatePen(PS_SOLID as i32, 1, gray));
        GRAY_BRUSH = Some(CreateSolidBrush(gray));
        let red = RGB(255, 0, 0);
        RED_PEN = Some(CreatePen(PS_SOLID as i32, 1, red));
        RED_BRUSH = Some(CreateSolidBrush(red));
        let h_instance = winapi::um::libloaderapi::GetModuleHandleW(null_mut());
        if h_instance == winapi::shared::ntdef::NULL as HINSTANCE
        {
            panic!("Failed to get module handle; error code {}", GetLastError());
        }
        let main_window_name = wide_char_string("main").as_ptr();
        let cursor = LoadCursorW(null_mut(), IDC_ARROW);
        if cursor == winapi::shared::ntdef::NULL as HICON
        {
            panic!("Failed to load cursor; error code {}", GetLastError());
        }
        if RegisterClassW(&WNDCLASSW{style: CS_HREDRAW | CS_OWNDC, lpfnWndProc:
            Some(main_window_proc as unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) ->
            LRESULT), cbClsExtra: 0, cbWndExtra: std::mem::size_of::<isize>() as i32, hInstance:
            h_instance, hIcon: null_mut(), hCursor: cursor,
            hbrBackground: (COLOR_WINDOW + 1) as HBRUSH, lpszMenuName: null_mut(),
            lpszClassName: main_window_name}) == 0
        {
            panic!("Failed to register main window class; error code {}", GetLastError());
        }
        let main_window_handle = CreateWindowExW(0, main_window_name,
            wide_char_string("Music Notation").as_ptr(), WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, null_mut(), null_mut(),
            h_instance, null_mut());
        if main_window_handle == winapi::shared::ntdef::NULL as HWND
        {
            panic!("Failed to create main window; error code {}", GetLastError());
        }
        let button_string = wide_char_string("BUTTON").as_ptr();
        let add_staff_button_handle = CreateWindowExW(0, button_string,
            wide_char_string("Add staff").as_ptr(), WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON |
            BS_VCENTER, 0, 0, 70, 20, main_window_handle, null_mut(), h_instance, null_mut());
        if add_staff_button_handle == winapi::shared::ntdef::NULL as HWND
        {
            panic!("Failed to create add staff button; error code {}", GetLastError());
        }
        let add_clef_button_handle = CreateWindowExW(0, button_string,
            wide_char_string("Add clef").as_ptr(), WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON |
            BS_VCENTER, 70, 0, 70, 20, main_window_handle, null_mut(), h_instance, null_mut());
        if add_clef_button_handle == winapi::shared::ntdef::NULL as HWND
        {
            panic!("Failed to create add clef button; error code {}", GetLastError());
        }
        let main_window_memory = MainWindowMemory{sized_music_fonts: HashMap::new(),
            staves: Vec::new(), system_slices: Vec::new(), entry_cursor: EntryCursor::None,
            add_staff_button_handle: add_staff_button_handle,
            add_clef_button_handle: add_clef_button_handle};		
        if SetWindowLongPtrW(main_window_handle, GWLP_USERDATA,
            &main_window_memory as *const _ as isize) == 0xe050
        {
            panic!("Failed to set extra window memory; error code {}", GetLastError());
        }
        ShowWindow(main_window_handle, SW_MAXIMIZE);
        let mut message: MSG = std::mem::uninitialized();
        while GetMessageW(&mut message, main_window_handle, 0, 0) > 0
        {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}