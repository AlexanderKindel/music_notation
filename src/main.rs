extern crate num_rational;
extern crate winapi;

use std::collections::HashMap;
use std::ptr::null_mut;
use winapi::um::errhandlingapi::GetLastError;
use winapi::shared::basetsd::*;
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::shared::windowsx::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

const WHOLE_NOTE_WIDTH: i32 = 120;
const DURATION_RATIO: f32 = 0.61803399;

//The add clef dialog returns the button identifiers of the selected clef shape and octave
//transposition ored together, so the nonzero bits of the shape identifiers must not overlap with
//those of the transposition identifiers.
const IDC_ADD_CLEF_G: i32 = 0b1000;
const IDC_ADD_CLEF_C: i32 = 0b1001;
const IDC_ADD_CLEF_F: i32 = 0b1010;
const IDC_ADD_CLEF_UNPITCHED: i32 = 0b1011;
const IDC_ADD_CLEF_15MA: i32 = 0b10000;
const IDC_ADD_CLEF_8VA: i32 = 0b100000;
const IDC_ADD_CLEF_NONE: i32 = 0b110000;
const IDC_ADD_CLEF_8VB: i32 = 0b1000000;
const IDC_ADD_CLEF_15MB: i32 = 0b1010000;
const ADD_CLEF_SHAPE_BITS: isize = 0b1111;
const ADD_CLEF_TRANSPOSITION_BITS: isize = 0b1110000;

static mut BLACK: Option<COLORREF> = None;
static mut GRAY_PEN: Option<HPEN> = None;
static mut GRAY_BRUSH: Option<HBRUSH> = None;
static mut RED: Option<COLORREF> = None; 
static mut RED_PEN: Option<HPEN> = None;
static mut RED_BRUSH: Option<HBRUSH> = None;
static mut ADD_CLEF_DIALOG_TEMPLATE: Option<Vec<u8>> = None;

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
    fn is_selected(&self) -> bool;
    fn set_selection_status(&mut self, selection_status: bool);
    fn move_baseline_up(&mut self);
    fn move_baseline_down(&mut self);
    fn is_clef(&self) -> bool
    {
        false
    }
}

struct Clef
{
    font_codepoint: u16,
    rhythmic_position: RhythmicPosition,
    distance_from_staff_start: i32,
    width: i32,
    staff_spaces_of_baseline_above_bottom_line: u8,
    is_selected: bool
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
            if self.is_selected
            {
                SetTextColor(device_context, RED.unwrap());
            }
            let staff_space_count = parent_staff.line_count as i32 - 1;
            TextOutW(device_context, parent_staff.left_edge + self.distance_from_staff_start +
                parent_staff.height / staff_space_count, parent_staff.bottom_line_y -
                (parent_staff.height * self.staff_spaces_of_baseline_above_bottom_line as i32) /
                staff_space_count, vec![self.font_codepoint, 0].as_ptr(), 1);
            SetTextColor(device_context, BLACK.unwrap());
        }
    }
    fn is_selected(&self) -> bool
    {
        self.is_selected
    }
    fn set_selection_status(&mut self, selection_status: bool)
    {
        self.is_selected = selection_status;
    }
    fn move_baseline_up(&mut self)
    {
        if self.staff_spaces_of_baseline_above_bottom_line < 4
        {
            self.staff_spaces_of_baseline_above_bottom_line += 1;
        }
    }
    fn move_baseline_down(&mut self)
    {
        if self.staff_spaces_of_baseline_above_bottom_line > 0
        {
            self.staff_spaces_of_baseline_above_bottom_line -= 1;
        }
    }
    fn is_clef(&self) -> bool
    {
        true
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
    staff_index: usize,
    staff_contents_index: usize
}

struct SystemSlice
{
    rhythmic_position: RhythmicPosition,
    objects_at_position: Vec<ObjectAddress>
}

struct SizedMusicFont
{
    font: HFONT,
    number_of_staves_with_size: u8 
}

enum Selection
{
    ActiveCursor(ObjectAddress),
    Objects(Vec<ObjectAddress>),
    None
}

struct MainWindowMemory
{
    sized_music_fonts: HashMap<i32, SizedMusicFont>,
    staves: Vec<Staff>,
    system_slices: Vec<SystemSlice>,
    ghost_cursor: Option<ObjectAddress>,
    selection: Selection,
    add_staff_button_handle: HWND,
    add_clef_button_handle: HWND
}

fn wide_char_string(value: &str) -> Vec<u16>
{    
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(value).encode_wide().chain(std::iter::once(0)).collect()
}

fn cancel_selection(window_handle: HWND, window_memory: *mut MainWindowMemory)
{
    unsafe
    {
        match (*window_memory).selection
        {
            Selection::ActiveCursor(ref active_address) =>
            {
                let ref staff = (*window_memory).staves[active_address.staff_index];
                InvalidateRect(window_handle, &RECT{left: staff.left_edge,
                    top: staff.bottom_line_y - staff.height, right: WHOLE_NOTE_WIDTH,
                    bottom: staff.bottom_line_y}, TRUE);
                EnableWindow((*window_memory).add_clef_button_handle, FALSE);
            }
            Selection::Objects(ref addresses) =>
            {
                for address in addresses
                {
                    (*window_memory).staves[address.staff_index].contents[
                        address.staff_contents_index].set_selection_status(false);
                }
                let mut client_rect: RECT = std::mem::uninitialized();
                GetClientRect(window_handle, &mut client_rect);
                InvalidateRect(window_handle, &client_rect, FALSE);
            },
            Selection::None => ()
        }        
        (*window_memory).selection = Selection::None;
    }
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
                SetFocus(window_handle);
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
                    match (*window_memory).selection
                    {
                        Selection::ActiveCursor(ref address) =>
                        {
                            let template =
                            match &ADD_CLEF_DIALOG_TEMPLATE
                            {
                                Some(template) => template.as_ptr(),
                                None => panic!("Add clef dialog template not found.")
                            };
                            let clef_selection = DialogBoxIndirectParamW(null_mut(), template as
                                *const DLGTEMPLATE, window_handle, Some(add_clef_dialog_proc), 0);
                            unsafe fn add_clef(window_handle: HWND, window_memory:
                                *mut MainWindowMemory, address: &ObjectAddress,
                                font_codepoint: u16, baseline_offset: u8)
                            {
                                let staff =
                                    &mut(*window_memory).staves[address.staff_index];
                                let device_context = GetDC(window_handle);
                                SelectObject(device_context, (*window_memory).sized_music_fonts.get(
                                    &staff.height).unwrap().font as *mut winapi::ctypes::c_void);
                                let mut abc_array: [ABC; 1] = std::mem::uninitialized();
                                GetCharABCWidthsW(device_context, font_codepoint as u32,
                                    font_codepoint as u32 + 1, abc_array.as_mut_ptr());
                                let clef = Box::new(Clef{font_codepoint: font_codepoint,
                                    rhythmic_position: RhythmicPosition{bar_number: 0,
                                    whole_notes_from_start_of_bar: num_rational::Ratio::new(0, 1)},
                                    distance_from_staff_start: 0, width: abc_array[0].abcB as i32 +
                                    (2 * staff.height) / (staff.line_count as i32 - 1),
                                    staff_spaces_of_baseline_above_bottom_line: baseline_offset,
                                    is_selected: true});
                                let mut staff_contents_index = address.staff_contents_index;
                                if address.staff_contents_index > 0 && staff.contents[
                                    address.staff_contents_index - 1].is_clef()
                                {
                                    staff.contents[address.staff_contents_index - 1] = clef;
                                    staff_contents_index -= 1;
                                }
                                else if staff.contents.len() > address.staff_contents_index &&
                                    staff.contents[address.staff_contents_index].is_clef()
                                {
                                    staff.contents[address.staff_contents_index] = clef;
                                }
                                else
                                {
                                    staff.contents.insert(address.staff_contents_index, clef);
                                }
                                (*window_memory).selection = Selection::Objects(
                                    vec![ObjectAddress{staff_index: address.staff_index,
                                    staff_contents_index: staff_contents_index}]);
                                let mut client_rect: RECT = std::mem::uninitialized();
                                GetClientRect(window_handle, &mut client_rect);
                                InvalidateRect(window_handle, &client_rect, TRUE);
                            }
                            match (clef_selection & ADD_CLEF_SHAPE_BITS) as i32
                            {                                
                                IDC_ADD_CLEF_G =>
                                {
                                    let codepoint =
                                    match (clef_selection & ADD_CLEF_TRANSPOSITION_BITS) as i32
                                    {                                        
                                        IDC_ADD_CLEF_15MA => 0xe054,
                                        IDC_ADD_CLEF_8VA => 0xe053,
                                        IDC_ADD_CLEF_NONE => 0xe050,
                                        IDC_ADD_CLEF_8VB => 0xe052,
                                        IDC_ADD_CLEF_15MB => 0xe051,
                                        _ => panic!("Unknown clef octave transposition.")
                                    };
                                    add_clef(window_handle, window_memory, address, codepoint, 1);
                                },
                                IDC_ADD_CLEF_C =>
                                {
                                    let codepoint =
                                    match (clef_selection & ADD_CLEF_TRANSPOSITION_BITS) as i32
                                    {
                                        IDC_ADD_CLEF_NONE => 0xe05c,
                                        IDC_ADD_CLEF_8VB => 0xe05d,
                                        _ => panic!("Unknown clef octave transposition.")
                                    };
                                    add_clef(window_handle, window_memory, address, codepoint, 2);
                                },
                                IDC_ADD_CLEF_F =>
                                {
                                    let codepoint =
                                    match (clef_selection & ADD_CLEF_TRANSPOSITION_BITS) as i32
                                    {                                        
                                        IDC_ADD_CLEF_15MA => 0xe066,
                                        IDC_ADD_CLEF_8VA => 0xe065,
                                        IDC_ADD_CLEF_NONE => 0xe062,
                                        IDC_ADD_CLEF_8VB => 0xe064,
                                        IDC_ADD_CLEF_15MB => 0xe063,
                                        _ => panic!("Unknown clef octave transposition.")
                                    };
                                    add_clef(window_handle, window_memory, address, codepoint, 3);
                                },
                                IDC_ADD_CLEF_UNPITCHED =>
                                {
                                    add_clef(window_handle, window_memory, address, 0xe069, 2);
                                },
                                _ => ()
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
        WM_KEYDOWN =>
        {
            match w_param as i32 
            {
                VK_DOWN =>
                {
                    let window_memory =
                        GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
                    if let Selection::Objects(ref addresses) = (*window_memory).selection
                    {
                        for address in addresses
                        {
                            (*window_memory).staves[address.staff_index].contents[
                                address.staff_contents_index].move_baseline_down();
                        }
                        let mut client_rect: RECT = std::mem::uninitialized();
                        GetClientRect(window_handle, &mut client_rect);
                        InvalidateRect(window_handle, &client_rect, TRUE);
                    }
                },
                VK_ESCAPE =>
                {
                    cancel_selection(window_handle,
                        GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory);
                },
                VK_UP =>
                {
                    let window_memory =
                        GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
                    if let Selection::Objects(ref addresses) = (*window_memory).selection
                    {
                        for address in addresses
                        {
                            (*window_memory).staves[address.staff_index].contents[
                                address.staff_contents_index].move_baseline_up();
                        }
                        let mut client_rect: RECT = std::mem::uninitialized();
                        GetClientRect(window_handle, &mut client_rect);
                        InvalidateRect(window_handle, &client_rect, TRUE);
                    }
                },
                _ => ()
            }
            0
        },
        WM_LBUTTONDOWN =>
        {
            let window_memory =
                GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
            match (*window_memory).ghost_cursor
            {
                Some(ref ghost_address) =>
                {
                    cancel_selection(window_handle, window_memory);
                    (*window_memory).ghost_cursor = None;
                    let address_copy = ObjectAddress{staff_index: ghost_address.staff_index,
                        staff_contents_index: ghost_address.staff_contents_index};                    
                    (*window_memory).selection = Selection::ActiveCursor(address_copy);
                    let ref staff = (*window_memory).staves[ghost_address.staff_index];
                    InvalidateRect(window_handle, &RECT{left: staff.left_edge,
                        top: staff.bottom_line_y - staff.height, right: WHOLE_NOTE_WIDTH,
                        bottom: staff.bottom_line_y}, FALSE);
                    EnableWindow((*window_memory).add_clef_button_handle, TRUE);
                },
                _ => ()
            }
            0
        },
        WM_MOUSEMOVE =>
        {
            let window_memory =
                GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
            let cursor_x = GET_X_LPARAM(l_param);
            let cursor_y = GET_Y_LPARAM(l_param);                
            for staff_index in 0..(*window_memory).staves.len()
            {
                let staff = &(*window_memory).staves[staff_index];
                if staff.left_edge <= cursor_x && cursor_x <= staff.left_edge + WHOLE_NOTE_WIDTH &&
                    staff.bottom_line_y - staff.height <= cursor_y &&
                    cursor_y <= staff.bottom_line_y
                {
                    match (*window_memory).selection
                    {
                        Selection::ActiveCursor(ref address) =>
                        {
                            if address.staff_index == staff_index
                            {
                                return 0;
                            }
                        }
                        _ => ()
                    }
                    match (*window_memory).ghost_cursor
                    {
                        Some(ref address) =>
                        {
                            if address.staff_index == staff_index
                            {
                                return 0;
                            }
                            let old_staff = &(*window_memory).staves[address.staff_index];
                            InvalidateRect(window_handle, &RECT{left: old_staff.left_edge,
                                top: old_staff.bottom_line_y - old_staff.height,
                                right: WHOLE_NOTE_WIDTH, bottom: old_staff.bottom_line_y}, TRUE);
                        }
                        None => ()
                    }
                    (*window_memory).ghost_cursor =
                        Some(ObjectAddress{staff_index:staff_index, staff_contents_index: 0});
                    InvalidateRect(window_handle, &RECT{left: staff.left_edge,
                        top: staff.bottom_line_y - staff.height, right: WHOLE_NOTE_WIDTH,
                        bottom: staff.bottom_line_y}, FALSE);
                    return 0;
                }
            }
            match (*window_memory).ghost_cursor
            {
                Some(ref address) =>
                {                     
                    let staff = &(*window_memory).staves[address.staff_index];
                    InvalidateRect(window_handle, &RECT{left: staff.left_edge,
                        top: staff.bottom_line_y - staff.height, right: WHOLE_NOTE_WIDTH,
                        bottom: staff.bottom_line_y}, TRUE);
                    (*window_memory).ghost_cursor = None;
                }
                None => ()
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
            match (*window_memory).ghost_cursor
            {
                Some(ref address) =>
                {
                    let original_pen = SelectObject(device_context,
                        GRAY_PEN.unwrap() as *mut winapi::ctypes::c_void);
                    let original_brush = SelectObject(device_context,
                        GRAY_BRUSH.unwrap() as *mut winapi::ctypes::c_void);
                    let ref staff = (*window_memory).staves[address.staff_index];
                    Rectangle(device_context, staff.left_edge, staff.bottom_line_y - staff.height,
                        staff.left_edge + 1, staff.bottom_line_y);
                    SelectObject(device_context, original_pen);
                    SelectObject(device_context, original_brush);
                },
                None => ()
            }
            match (*window_memory).selection
            {
                Selection::ActiveCursor(ref address) =>
                {
                    let original_pen = SelectObject(device_context,
                        RED_PEN.unwrap() as *mut winapi::ctypes::c_void);
                    let original_brush = SelectObject(device_context,
                        RED_BRUSH.unwrap() as *mut winapi::ctypes::c_void);
                    let ref staff = (*window_memory).staves[address.staff_index];
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

unsafe extern "system" fn add_clef_dialog_proc(dialog_handle: HWND, u_msg: UINT, w_param: WPARAM,
    l_param: LPARAM) -> INT_PTR
{
    match u_msg
    {
        WM_COMMAND =>
        { 
            match LOWORD(w_param as u32) as i32
            {
                IDC_ADD_CLEF_C =>
                {         
                    let fifteen_ma_handle = GetDlgItem(dialog_handle, IDC_ADD_CLEF_15MA);
                    let eight_va_handle = GetDlgItem(dialog_handle, IDC_ADD_CLEF_8VA);
                    let none_handle = GetDlgItem(dialog_handle, IDC_ADD_CLEF_NONE);
                    let eight_vb_handle = GetDlgItem(dialog_handle, IDC_ADD_CLEF_8VB);
                    let fifteen_mb_handle = GetDlgItem(dialog_handle, IDC_ADD_CLEF_15MB);
                    EnableWindow(fifteen_ma_handle, FALSE);
                    EnableWindow(eight_va_handle, FALSE);
                    EnableWindow(none_handle, TRUE);
                    EnableWindow(eight_vb_handle, TRUE);
                    EnableWindow(fifteen_mb_handle, FALSE);                    
                    if SendMessageW(none_handle, BM_GETCHECK, 0, 0) != BST_CHECKED as isize &&
                        SendMessageW(eight_vb_handle, BM_GETCHECK, 0, 0) != BST_CHECKED as isize
                    {
                        SendMessageW(fifteen_ma_handle, BM_SETCHECK, BST_UNCHECKED, 0);
                        SendMessageW(eight_va_handle, BM_SETCHECK, BST_UNCHECKED, 0);
                        SendMessageW(none_handle, BM_SETCHECK, BST_CHECKED, 0);
                        SendMessageW(eight_vb_handle, BM_SETCHECK, BST_UNCHECKED, 0);
                        SendMessageW(fifteen_mb_handle, BM_SETCHECK, BST_UNCHECKED, 0);
                    }
                },
                IDC_ADD_CLEF_UNPITCHED =>
                {
                    EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_CLEF_15MA), FALSE);
                    EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_CLEF_8VA), FALSE);
                    EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_CLEF_NONE), FALSE);
                    EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_CLEF_8VB), FALSE);
                    EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_CLEF_15MB), FALSE);
                },
                IDCANCEL =>
                {
                    EndDialog(dialog_handle, 0);
                },
                IDOK =>
                {
                    let mut selection = 0;
                    for button in [IDC_ADD_CLEF_G, IDC_ADD_CLEF_C, IDC_ADD_CLEF_F,
                        IDC_ADD_CLEF_UNPITCHED].iter()
                    {
                        if SendMessageW(GetDlgItem(dialog_handle, *button), BM_GETCHECK, 0, 0) ==
                            BST_CHECKED as isize
                        {
                            selection |= button;
                            break;
                        }
                    }
                    for button in [IDC_ADD_CLEF_15MA, IDC_ADD_CLEF_8VA, IDC_ADD_CLEF_NONE,
                        IDC_ADD_CLEF_8VB, IDC_ADD_CLEF_15MB].iter()
                    {
                        if SendMessageW(GetDlgItem(dialog_handle, *button), BM_GETCHECK, 0, 0) ==
                            BST_CHECKED as isize
                        {
                            selection |= button;
                            break;
                        }
                    }
                    EndDialog(dialog_handle, selection as isize);
                },
                _ =>
                {
                    EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_CLEF_15MA), TRUE);
                    EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_CLEF_8VA), TRUE);
                    EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_CLEF_NONE), TRUE);
                    EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_CLEF_8VB), TRUE);
                    EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_CLEF_15MB), TRUE);                    
                }                
            }
            TRUE as isize
        },
        WM_INITDIALOG =>
        {
            SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_CLEF_G), BM_SETCHECK, BST_CHECKED, 0);
            SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_CLEF_NONE), BM_SETCHECK, BST_CHECKED, 0);
            TRUE as isize
        },
        _ => FALSE as isize
    }
}

fn add_u16(template: &mut Vec<u8>, value: u16)
{
    template.push((value & 0xff) as u8);
    template.push(((value & 0xff00) >> 8) as u8);
}

fn add_u32(template: &mut Vec<u8>, value: u32)
{
    template.push((value & 0xff) as u8);
    template.push(((value & 0xff00) >> 8) as u8);
    template.push(((value & 0xff0000) >> 16) as u8);
    template.push(((value & 0xff000000) >> 24) as u8);
}

fn create_dialog_control_template(style: DWORD,  left_edge: u16, top_edge: u16, width: u16,
    height: u16, id: u32, window_class: &Vec<u16>, text: &Vec<u16>) -> Vec<u8>
{
    let mut template: Vec<u8> = vec![0; 8];
    add_u32(&mut template, style);
    add_u16(&mut template, left_edge);
    add_u16(&mut template, top_edge);
    add_u16(&mut template, width);
    add_u16(&mut template, height);
    add_u32(&mut template, id);
    for character in window_class
    {
        add_u16(&mut template, *character);
    }
    for character in text
    {
        add_u16(&mut template, *character);
    }
    template.push(0);
    template.push(0);
    template
}

fn create_dialog_template(style: DWORD, left_edge: u16, top_edge: u16, width: u16,
    height: u16, title: Vec<u16>, controls: Vec<&mut Vec<u8>>) -> Vec<u8>
{        
    let mut template: Vec<u8> = vec![1, 0, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0];
    add_u32(&mut template, style);
    add_u16(&mut template, controls.len() as u16);
    add_u16(&mut template, left_edge);
    add_u16(&mut template, top_edge);
    add_u16(&mut template, width);
    add_u16(&mut template, height);
    template.push(0);
    template.push(0);
    template.push(0);
    template.push(0);
    for character in title
    {
        add_u16(&mut template, character);
    }    
    for control in controls
    {
        if template.len() % 4 != 0
        {
            template.push(0);
            template.push(0);
        }
        template.append(control);
    }
    template    
}

fn main()
{
    unsafe
    {
        BLACK = Some(RGB(0, 0, 0));
        let gray = RGB(127, 127, 127);
        GRAY_PEN = Some(CreatePen(PS_SOLID as i32, 1, gray));
        GRAY_BRUSH = Some(CreateSolidBrush(gray));
        let red = RGB(255, 0, 0);
        RED_PEN = Some(CreatePen(PS_SOLID as i32, 1, red));
        RED_BRUSH = Some(CreateSolidBrush(red));
        RED = Some(red);
        let button_string = wide_char_string("button");
        let static_string = wide_char_string("static");
        let mut add_clef_dialog_ok = create_dialog_control_template(BS_PUSHBUTTON | WS_CHILD |
            WS_VISIBLE, 45, 70, 30, 10, IDOK as u32, &button_string,
            &wide_char_string("OK"));
        let mut add_clef_dialog_cancel = create_dialog_control_template(BS_PUSHBUTTON | WS_CHILD |
            WS_VISIBLE, 75, 70, 30, 10, IDCANCEL as u32, &button_string,
            &wide_char_string("Cancel"));
        let mut add_clef_dialog_shape = create_dialog_control_template(SS_LEFT | WS_CHILD |
            WS_VISIBLE, 5, 0, 40, 10, 0, &static_string, &wide_char_string("Clef shape:"));
        let mut add_clef_dialog_octave = create_dialog_control_template(SS_LEFT | WS_CHILD |
            WS_VISIBLE, 75, 0, 70, 10, 0, &static_string,
            &wide_char_string("Octave transposition:"));
        let mut add_clef_dialog_shape_frame = create_dialog_control_template(SS_GRAYFRAME |
            WS_CHILD | WS_VISIBLE, 5, 10, 70, 60, 0, &static_string, &vec![0]);
        let mut add_clef_dialog_transposition_frame = create_dialog_control_template(SS_GRAYFRAME |
            WS_CHILD | WS_VISIBLE, 75, 10, 70, 60, 0, &static_string, &vec![0]);
        let mut add_clef_dialog_g_clef = create_dialog_control_template(BS_AUTORADIOBUTTON |
            WS_CHILD | WS_GROUP | WS_VISIBLE, 10, 20, 45, 10, IDC_ADD_CLEF_G as u32,
            &button_string, &wide_char_string("G"));
        let mut add_clef_dialog_c_clef = create_dialog_control_template(BS_AUTORADIOBUTTON |
            WS_CHILD | WS_VISIBLE, 10, 30, 45, 10, IDC_ADD_CLEF_C as u32, &button_string,
            &wide_char_string("C"));
        let mut add_clef_dialog_f_clef = create_dialog_control_template(BS_AUTORADIOBUTTON |
            WS_CHILD | WS_VISIBLE, 10, 40, 45, 10, IDC_ADD_CLEF_F as u32, &button_string,
            &wide_char_string("F"));
        let mut add_clef_dialog_unpitched_clef = create_dialog_control_template(BS_AUTORADIOBUTTON |
            WS_CHILD | WS_VISIBLE, 10, 50, 45, 10, IDC_ADD_CLEF_UNPITCHED as u32, &button_string,
            &wide_char_string("Unpitched"));
        let mut add_clef_dialog_15ma = create_dialog_control_template(BS_AUTORADIOBUTTON |
            WS_CHILD | WS_GROUP | WS_VISIBLE, 80, 15, 30, 10, IDC_ADD_CLEF_15MA as u32,
            &button_string, &wide_char_string("15ma"));
        let mut add_clef_dialog_8va = create_dialog_control_template(BS_AUTORADIOBUTTON |
            WS_CHILD | WS_VISIBLE, 80, 25, 30, 10, IDC_ADD_CLEF_8VA as u32, &button_string,
            &wide_char_string("8va"));
        let mut add_clef_dialog_none = create_dialog_control_template(BS_AUTORADIOBUTTON |
            WS_CHILD | WS_VISIBLE, 80, 35, 30, 10, IDC_ADD_CLEF_NONE as u32, &button_string,
            &wide_char_string("None"));
        let mut add_clef_dialog_8vb = create_dialog_control_template(BS_AUTORADIOBUTTON |
            WS_CHILD | WS_VISIBLE, 80, 45, 30, 10, IDC_ADD_CLEF_8VB as u32, &button_string,
            &wide_char_string("8vb"));
        let mut add_clef_dialog_15mb = create_dialog_control_template(BS_AUTORADIOBUTTON |
            WS_CHILD | WS_VISIBLE, 80, 55, 30, 10, IDC_ADD_CLEF_15MB as u32, &button_string,
            &wide_char_string("15mb"));
        ADD_CLEF_DIALOG_TEMPLATE = Some(create_dialog_template(DS_CENTER, 0, 0, 160, 100,
            wide_char_string("Add Clef"), vec![&mut add_clef_dialog_ok, &mut add_clef_dialog_cancel,
            &mut add_clef_dialog_shape, &mut add_clef_dialog_octave,
            &mut add_clef_dialog_shape_frame, &mut add_clef_dialog_transposition_frame,
            &mut add_clef_dialog_g_clef, &mut add_clef_dialog_c_clef, &mut add_clef_dialog_f_clef,
            &mut add_clef_dialog_unpitched_clef, &mut add_clef_dialog_15ma,
            &mut add_clef_dialog_8va, &mut add_clef_dialog_none, &mut add_clef_dialog_8vb,
            &mut add_clef_dialog_15mb]));
        let h_instance = winapi::um::libloaderapi::GetModuleHandleW(null_mut());
        if h_instance == winapi::shared::ntdef::NULL as HINSTANCE
        {
            panic!("Failed to get module handle; error code {}", GetLastError());
        }
        let main_window_name = wide_char_string("main");
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
            lpszClassName: main_window_name.as_ptr()}) == 0
        {
            panic!("Failed to register main window class; error code {}", GetLastError());
        }
        let main_window_handle = CreateWindowExW(0, main_window_name.as_ptr(),
            wide_char_string("Music Notation").as_ptr(), WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, null_mut(), null_mut(),
            h_instance, null_mut());
        if main_window_handle == winapi::shared::ntdef::NULL as HWND
        {
            panic!("Failed to create main window; error code {}", GetLastError());
        }        
        let add_staff_button_handle = CreateWindowExW(0, button_string.as_ptr(),
            wide_char_string("Add staff").as_ptr(), WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON |
            BS_VCENTER, 0, 0, 70, 20, main_window_handle, null_mut(), h_instance, null_mut());
        if add_staff_button_handle == winapi::shared::ntdef::NULL as HWND
        {
            panic!("Failed to create add staff button; error code {}", GetLastError());
        }
        let add_clef_button_handle = CreateWindowExW(0, button_string.as_ptr(),
            wide_char_string("Add clef").as_ptr(), WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON |
            BS_VCENTER, 70, 0, 70, 20, main_window_handle, null_mut(), h_instance, null_mut());
        if add_clef_button_handle == winapi::shared::ntdef::NULL as HWND
        {
            panic!("Failed to create add clef button; error code {}", GetLastError());
        }
        EnableWindow(add_clef_button_handle, FALSE);
        let main_window_memory = MainWindowMemory{sized_music_fonts: HashMap::new(),
            staves: Vec::new(), system_slices: Vec::new(), ghost_cursor: None,
            selection: Selection::None, add_staff_button_handle: add_staff_button_handle,
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