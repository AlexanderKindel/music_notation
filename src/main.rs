extern crate num_bigint;
extern crate num_rational;
extern crate serde_json;
extern crate winapi;

use num_bigint::ToBigInt;
use std::collections::HashMap;
use std::fs::File;
use std::ptr::null_mut;
use winapi::um::errhandlingapi::GetLastError;
use winapi::shared::basetsd::*;
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::shared::windowsx::*;
use winapi::um::commctrl::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

const WHOLE_NOTE_WIDTH: i32 = 120;
const DURATION_RATIO: f32 = 0.61803399;
const DURATIONS: [&str; 4] = ["double whole", "whole", "half", "quarter"];

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
    whole_notes_from_start_of_bar: num_rational::BigRational
}

enum StaffObjectType
{
    //log2_duration denotes the power of two times the duration of a whole note of the note's
    //duration.
    Note{log2_duration: isize, steps_above_middle_c: i8},
    Clef{font_codepoint: u16, staff_spaces_of_baseline_above_bottom_line: u8,
        steps_of_bottom_staff_line_above_middle_c: i8}    
}

struct StaffObject
{
    object_type: StaffObjectType,
    width: i32,
    distance_from_staff_start: i32,
    rhythmic_position: RhythmicPosition,
    is_selected: bool    
}

struct Staff
{
    line_count: u8,
    line_thickness: f32,
    left_edge: i32,
    bottom_line_y: i32,
    height: i32,
    contents: Vec<StaffObject>
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

enum Selection<'a>
{
    ActiveCursor(ObjectAddress),
    Objects(Vec<&'a mut StaffObject>),
    None
}

struct MainWindowMemory<'a>
{
    sized_music_fonts: HashMap<i32, SizedMusicFont>,
    staves: Vec<Staff>,
    system_slices: Vec<SystemSlice>,
    ghost_cursor: Option<ObjectAddress>,
    selection: Selection<'a>,
    add_staff_button_handle: HWND,
    add_clef_button_handle: HWND,
    duration_display_handle: HWND,
    duration_spin_handle: HWND,
    default_leger_line_thickness: f32,
    default_leger_line_extension: f32,
    default_staff_line_thickness: f32    
}

fn wide_char_string(value: &str) -> Vec<u16>
{    
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(value).encode_wide().chain(std::iter::once(0)).collect()
}

fn get_distance_and_whole_notes_from_start_of_bar(staff: &Staff, address: &ObjectAddress) ->
    (i32, num_rational::BigRational)
{
    if address.staff_contents_index == 0
    {
        (0, num_rational::Ratio::new(0.to_bigint().unwrap(), 1.to_bigint().unwrap()))
    }
    else
    {
        let object_before_cursor = &staff.contents[address.staff_contents_index - 1];        
        let whole_notes_from_start_of_bar =
        match object_before_cursor.object_type
        {
            StaffObjectType::Clef{..} =>
                object_before_cursor.rhythmic_position.whole_notes_from_start_of_bar.clone(),
            StaffObjectType::Note{log2_duration,..} =>
            {
                let whole_notes_long =
                if log2_duration >= 0
                {
                    num_rational::Ratio::new(num_bigint::BigInt::from(
                        2u32.pow(log2_duration as u32)), 1.to_bigint().unwrap())
                }
                else
                {
                    num_rational::Ratio::new(1.to_bigint().unwrap(),
                        num_bigint::BigInt::from(2u32.pow(-log2_duration as u32)))
                };                
                object_before_cursor.rhythmic_position.whole_notes_from_start_of_bar.clone() +
                    whole_notes_long
            }
        };
        (object_before_cursor.distance_from_staff_start + object_before_cursor.width,
            whole_notes_from_start_of_bar)
    }
}

fn add_note(window_handle: HWND, steps_above_middle_c: i8)
{
    unsafe
    {
        let window_memory =
            GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
        if let Selection::ActiveCursor(ref address) = (*window_memory).selection
        {            
            let log2_duration =
                1 - (SendMessageW((*window_memory).duration_spin_handle, UDM_GETPOS, 0, 0) & 0xff);
            let staff = &mut (*window_memory).staves[address.staff_index];
            let (distance_from_staff_start, whole_notes_from_start_of_bar) =
                get_distance_and_whole_notes_from_start_of_bar(staff, address);
            let width = (WHOLE_NOTE_WIDTH as f32 *
                DURATION_RATIO.powi(-log2_duration as i32)).round() as i32;
            staff.contents.insert(address.staff_contents_index, StaffObject{object_type:
                StaffObjectType::Note{log2_duration: log2_duration, steps_above_middle_c:
                steps_above_middle_c}, width: width, distance_from_staff_start:
                distance_from_staff_start, rhythmic_position: RhythmicPosition{bar_number: 0,
                whole_notes_from_start_of_bar: whole_notes_from_start_of_bar}, is_selected: false});
            for index in address.staff_contents_index + 1..staff.contents.len()
            {
                staff.contents[index].distance_from_staff_start += width;
            }
            (*window_memory).selection = Selection::ActiveCursor(ObjectAddress{staff_index:
                address.staff_index, staff_contents_index: staff.contents.len()});
            let mut client_rect: RECT = std::mem::uninitialized();
            GetClientRect(window_handle, &mut client_rect);
            InvalidateRect(window_handle, &client_rect, TRUE);
        }
    }
}

fn cancel_selection(window_handle: HWND)
{
    unsafe
    {
        let window_memory =
            GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
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
            Selection::Objects(ref mut objects) =>
            {
                for object in objects
                {
                    object.is_selected = false;
                }
                let mut client_rect: RECT = std::mem::uninitialized();
                GetClientRect(window_handle, &mut client_rect);
                InvalidateRect(window_handle, &client_rect, TRUE);
                EnableWindow((*window_memory).add_clef_button_handle, FALSE);
            },
            Selection::None => ()
        }        
        (*window_memory).selection = Selection::None;
    }
}

fn get_cursor_rect(cursor_address: &ObjectAddress, window_memory: *const MainWindowMemory) -> RECT
{
    unsafe
    {
        let staff = &(*window_memory).staves[cursor_address.staff_index];
        let mut cursor_left_edge = staff.left_edge;
        if cursor_address.staff_contents_index > 0
        {
            let object_before_cursor = &staff.contents[cursor_address.staff_contents_index - 1];
            cursor_left_edge +=
                object_before_cursor.distance_from_staff_start + object_before_cursor.width;
        }
        RECT{left: cursor_left_edge, top: staff.bottom_line_y - staff.height,
            right: cursor_left_edge + 1, bottom: staff.bottom_line_y}
    }
}

fn get_character_width(device_context: HDC, window_memory: *const MainWindowMemory, staff: &Staff,
    font_codepoint: u32) -> i32
{
    unsafe
    {
        SelectObject(device_context, (*window_memory).sized_music_fonts.get(
            &staff.height).unwrap().font as *mut winapi::ctypes::c_void);
        let mut abc_array: [ABC; 1] = [ABC{abcA: 0, abcB: 0, abcC: 0}];
        GetCharABCWidthsW(device_context, font_codepoint,
            font_codepoint + 1, abc_array.as_mut_ptr());
        abc_array[0].abcB as i32
    }
}

unsafe extern "system" fn main_window_proc(window_handle: HWND, u_msg: UINT, w_param: WPARAM,
    l_param: LPARAM) -> LRESULT
{
    match u_msg
    {
        WM_CTLCOLORSTATIC =>
        {
            GetStockObject(WHITE_BRUSH as i32) as isize
        },
        WM_COMMAND =>
        {
            if HIWORD(w_param as u32) == BN_CLICKED
            {
                SetFocus(window_handle);
                let window_memory =
                    GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;                
                if l_param == (*window_memory).add_clef_button_handle as isize
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
                            let (codepoint, baseline_offset, steps_of_bottom_line_above_middle_c) =
                            match (clef_selection & ADD_CLEF_SHAPE_BITS) as i32
                            {                                
                                IDC_ADD_CLEF_G =>
                                {
                                    let (codepoint, steps_of_bottom_line_above_middle_c) =
                                    match (clef_selection & ADD_CLEF_TRANSPOSITION_BITS) as i32
                                    {                                        
                                        IDC_ADD_CLEF_15MA => (0xe054, 16),
                                        IDC_ADD_CLEF_8VA => (0xe053, 9),
                                        IDC_ADD_CLEF_NONE => (0xe050, 2),
                                        IDC_ADD_CLEF_8VB => (0xe052, -5),
                                        IDC_ADD_CLEF_15MB => (0xe051, -12),
                                        _ => panic!("Unknown clef octave transposition.")
                                    };
                                    (codepoint, 1, steps_of_bottom_line_above_middle_c)
                                },
                                IDC_ADD_CLEF_C =>
                                {
                                    let (codepoint, steps_of_bottom_line_above_middle_c) =
                                    match (clef_selection & ADD_CLEF_TRANSPOSITION_BITS) as i32
                                    {
                                        IDC_ADD_CLEF_NONE => (0xe05c, -4),
                                        IDC_ADD_CLEF_8VB => (0xe05d, -11),
                                        _ => panic!("Unknown clef octave transposition.")
                                    };
                                    (codepoint, 2, steps_of_bottom_line_above_middle_c)
                                },
                                IDC_ADD_CLEF_F =>
                                {
                                    let (codepoint, steps_of_bottom_line_above_middle_c) =
                                    match (clef_selection & ADD_CLEF_TRANSPOSITION_BITS) as i32
                                    {                                        
                                        IDC_ADD_CLEF_15MA => (0xe066, 4),
                                        IDC_ADD_CLEF_8VA => (0xe065, -3),
                                        IDC_ADD_CLEF_NONE => (0xe062, -10),
                                        IDC_ADD_CLEF_8VB => (0xe064, -17),
                                        IDC_ADD_CLEF_15MB => (0xe063, -24),
                                        _ => panic!("Unknown clef octave transposition.")
                                    };
                                    (codepoint, 3, steps_of_bottom_line_above_middle_c)
                                },
                                IDC_ADD_CLEF_UNPITCHED =>
                                {
                                    (0xe069, 2, 2)
                                },
                                _ => return 0                                
                            };
                            let staff = &mut(*window_memory).staves[address.staff_index];
                            let width = get_character_width(GetDC(window_handle), window_memory,
                                staff, codepoint as u32) + (2 * staff.height) / (staff.line_count as i32 - 1);
                            let (distance_from_staff_start, whole_notes_from_start_of_bar) =
                                get_distance_and_whole_notes_from_start_of_bar(staff, address);
                            let clef = StaffObject{
                                object_type: StaffObjectType::Clef{font_codepoint: codepoint,
                                staff_spaces_of_baseline_above_bottom_line: baseline_offset,
                                steps_of_bottom_staff_line_above_middle_c:
                                steps_of_bottom_line_above_middle_c}, width: width,
                                distance_from_staff_start: distance_from_staff_start,
                                rhythmic_position: RhythmicPosition{bar_number: 0,
                                whole_notes_from_start_of_bar: whole_notes_from_start_of_bar},
                                is_selected: true};
                            fn add_clef_to_staff_contents(cursor_index: usize, staff: &mut Staff,
                                clef: StaffObject) -> usize
                            {
                                if cursor_index > 0
                                {
                                    let clef_index = cursor_index - 1;
                                    if let StaffObjectType::Clef{..} =
                                        staff.contents[clef_index].object_type
                                    {
                                        staff.contents[clef_index] = clef;
                                        return clef_index;
                                    }
                                }
                                if cursor_index < staff.contents.len()
                                {
                                    if let StaffObjectType::Clef{..} =
                                        staff.contents[cursor_index].object_type
                                    {
                                        staff.contents[cursor_index] = clef;
                                        return cursor_index;
                                    }
                                }
                                staff.contents.insert(cursor_index, clef);
                                cursor_index
                            }
                            let clef_index = add_clef_to_staff_contents(
                                address.staff_contents_index, staff, clef);                                                                 
                            for index in clef_index + 1..staff.contents.len()
                            {
                                staff.contents[index].distance_from_staff_start += width;
                            }
                            (*window_memory).selection =
                                Selection::Objects(vec![&mut staff.contents[clef_index]]);
                            let mut client_rect: RECT = std::mem::uninitialized();
                            GetClientRect(window_handle, &mut client_rect);
                            InvalidateRect(window_handle, &client_rect, TRUE);
                        }
                        _ => ()
                    }
                }
                else if l_param == (*window_memory).add_staff_button_handle as isize
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
                    let height = 40;
                    (*window_memory).staves.push(Staff{line_count: 5, line_thickness:
                        (*window_memory).default_staff_line_thickness, left_edge: 20,
                        bottom_line_y: bottom_line_y, height: height, contents: Vec::new()});
                    let mut client_rect: RECT = std::mem::uninitialized();
                    GetClientRect(window_handle, &mut client_rect);
                    InvalidateRect(window_handle, &client_rect, TRUE);
                    match (*window_memory).sized_music_fonts.get_mut(&height)
                    {
                        Some(sized_font) =>
                        {
                            sized_font.number_of_staves_with_size +=1;
                        }
                        None =>
                        {
                            (*window_memory).sized_music_fonts.insert(height, SizedMusicFont{font:
                                CreateFontW(-height, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                wide_char_string("Bravura").as_ptr()),
                                number_of_staves_with_size: 1});
                        }
                    };
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
                0x41 =>
                {
                    add_note(window_handle, 5);
                    0
                },
                0x42 =>
                {
                    add_note(window_handle, 6);
                    0
                },
                0x43 =>
                {
                    add_note(window_handle, 0);
                    0
                },
                0x44 =>
                {
                    add_note(window_handle, 1);
                    0
                },
                0x45 =>
                {
                    add_note(window_handle, 2);
                    0
                },
                0x46 =>
                {
                    add_note(window_handle, 3);
                    0
                },
                0x47 =>
                {                    
                    add_note(window_handle, 4);
                    0
                },
                VK_DOWN =>
                {
                    let window_memory =
                        GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
                    if let Selection::Objects(ref mut objects) = (*window_memory).selection
                    {
                        for object in objects
                        {
                            match object.object_type
                            {
                                StaffObjectType::Clef{
                                    ref mut staff_spaces_of_baseline_above_bottom_line, 
                                    ref mut steps_of_bottom_staff_line_above_middle_c,..} => 
                                {
                                    *staff_spaces_of_baseline_above_bottom_line -= 1;
                                    *steps_of_bottom_staff_line_above_middle_c += 2;
                                },
                                StaffObjectType::Note{ref mut steps_above_middle_c,..} =>
                                    *steps_above_middle_c -= 1
                            }
                        }
                        let mut client_rect: RECT = std::mem::uninitialized();
                        GetClientRect(window_handle, &mut client_rect);
                        InvalidateRect(window_handle, &client_rect, TRUE);
                    }
                    0
                },
                VK_ESCAPE =>
                {
                    cancel_selection(window_handle);
                    0
                },
                VK_LEFT =>
                {
                    let window_memory =
                        GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
                    match (*window_memory).selection
                    {
                        Selection::ActiveCursor(ref mut address) =>
                        {
                            if address.staff_contents_index > 0
                            {                                
                                InvalidateRect(window_handle,
                                    &get_cursor_rect(address, window_memory), TRUE);
                                address.staff_contents_index -= 1;  
                                InvalidateRect(window_handle,
                                    &get_cursor_rect(address, window_memory), TRUE);
                            }
                        }
                        _ => ()
                    }
                    0
                },
                VK_RIGHT =>
                {
                    let window_memory =
                        GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
                    match (*window_memory).selection
                    {
                        Selection::ActiveCursor(ref mut address) =>
                        {
                            if address.staff_contents_index <
                                (*window_memory).staves[address.staff_index].contents.len()
                            {                                
                                InvalidateRect(window_handle,
                                    &get_cursor_rect(address, window_memory), TRUE);
                                address.staff_contents_index += 1;  
                                InvalidateRect(window_handle,
                                    &get_cursor_rect(address, window_memory), TRUE);
                            }
                        }
                        _ => ()
                    }
                    0
                },
                VK_UP =>
                {
                    let window_memory =
                        GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
                    if let Selection::Objects(ref mut objects) = (*window_memory).selection
                    {
                        for object in objects
                        {
                            match object.object_type
                            {
                                StaffObjectType::Clef{
                                    ref mut staff_spaces_of_baseline_above_bottom_line, 
                                    ref mut steps_of_bottom_staff_line_above_middle_c,..} =>
                                {
                                    *staff_spaces_of_baseline_above_bottom_line += 1;
                                    *steps_of_bottom_staff_line_above_middle_c -= 2;
                                },
                                StaffObjectType::Note{ref mut steps_above_middle_c,..} =>
                                    *steps_above_middle_c += 1
                            }
                        }
                        let mut client_rect: RECT = std::mem::uninitialized();
                        GetClientRect(window_handle, &mut client_rect);
                        InvalidateRect(window_handle, &client_rect, TRUE);
                    }
                    0
                },
                _ => DefWindowProcW(window_handle, u_msg, w_param, l_param)
            }
        },
        WM_LBUTTONDOWN =>
        {
            let window_memory =
                GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
            match (*window_memory).ghost_cursor
            {
                Some(ref ghost_address) =>
                {
                    cancel_selection(window_handle);
                    (*window_memory).ghost_cursor = None;
                    let address_copy = ObjectAddress{staff_index: ghost_address.staff_index,
                        staff_contents_index: ghost_address.staff_contents_index};                    
                    (*window_memory).selection = Selection::ActiveCursor(address_copy);
                    let ref staff = (*window_memory).staves[ghost_address.staff_index];
                    InvalidateRect(window_handle, &RECT{left: staff.left_edge,
                        top: staff.bottom_line_y - staff.height, right: WHOLE_NOTE_WIDTH,
                        bottom: staff.bottom_line_y}, TRUE);
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
                        bottom: staff.bottom_line_y}, TRUE);
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
        WM_NOTIFY =>
        {
            if (*(l_param as LPNMHDR)).code == UDN_DELTAPOS
            {
                let lpnmud = l_param as LPNMUPDOWN;
                let new_position = (*lpnmud).iPos + (*lpnmud).iDelta;
                let new_text =                
                if new_position < 0
                {
                    wide_char_string("double whole")
                }
                else if new_position > 31
                {
                    wide_char_string("1073741824th")
                }
                else if new_position > 3
                {
                    let two: u32 = 2;
                    let denominator = two.pow((new_position - 1) as u32);
                    if denominator % 10 == 2
                    {
                        wide_char_string(&format!("{}nd", denominator))
                    }
                    else
                    {
                        wide_char_string(&format!("{}th", denominator))
                    }
                }
                else
                {
                    wide_char_string(DURATIONS[new_position as usize])
                };
                SendMessageW((*(GetWindowLongPtrW(window_handle, GWLP_USERDATA) as
                    *mut MainWindowMemory)).duration_display_handle, WM_SETTEXT, 0,
                    new_text.as_ptr() as isize);
                0
            }
            else
            {
                DefWindowProcW(window_handle, u_msg, w_param, l_param)
            }
        },
        WM_PAINT =>
        {
            let window_memory =
                GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
            let mut ps: PAINTSTRUCT = std::mem::uninitialized();
            let device_context = BeginPaint(window_handle, &mut ps);						
            for staff in &(*window_memory).staves
            {                
                let space_count = staff.line_count as i32 - 1;                
                let mut line_thickness = ((staff.height as f32 * staff.line_thickness) /
                    space_count as f32).round() as i32;
                if line_thickness == 0
                {
                    line_thickness = 1;
                }
                let baseline = staff.bottom_line_y + line_thickness / 2;                	        
                let mut right_edge = staff.left_edge + WHOLE_NOTE_WIDTH;
                if staff.contents.len() > 0
                {
                    let last_object = &staff.contents[staff.contents.len() - 1];
                    right_edge += last_object.distance_from_staff_start + last_object.width;
                }
                let original_device_context = SaveDC(device_context);
                SelectObject(device_context, GetStockObject(BLACK_PEN as i32));
                SelectObject(device_context, GetStockObject(BLACK_BRUSH as i32));
                let draw_lines = |left_edge: i32, right_edge: i32,
                    spaces_of_bottom_line_above_baseline: i32, line_count: i32, line_thickness: i32|
                {
                    let mut staff_height_times_line_number =
                        staff.height * spaces_of_bottom_line_above_baseline;
                    for _ in 0..line_count
                    {		    
                        let current_line_bottom =
                            baseline - staff_height_times_line_number / space_count;
                        Rectangle(device_context, left_edge, current_line_bottom - line_thickness,
                            right_edge, current_line_bottom);
                        staff_height_times_line_number += staff.height;
                    }
                };
                draw_lines(staff.left_edge, right_edge, 0, staff.line_count as i32, line_thickness);
                SelectObject(device_context, (*window_memory).sized_music_fonts.get(
                    &staff.height).unwrap().font as *mut winapi::ctypes::c_void);
                SetBkMode(device_context, TRANSPARENT as i32);
                SetTextAlign(device_context, TA_BASELINE);
                let mut steps_of_bottom_line_above_middle_c = 2;
                for staff_object in &staff.contents
                {
                    if staff_object.is_selected
                    {
                        SetTextColor(device_context, RED.unwrap());
                    }
                    match staff_object.object_type
                    {
                        StaffObjectType::Clef{font_codepoint,
                            staff_spaces_of_baseline_above_bottom_line,
                            steps_of_bottom_staff_line_above_middle_c} =>
                        {
                            steps_of_bottom_line_above_middle_c =
                                steps_of_bottom_staff_line_above_middle_c;
                            let staff_space_count = staff.line_count as i32 - 1;
                            TextOutW(device_context, staff.left_edge +
                                staff_object.distance_from_staff_start +
                                staff.height / staff_space_count, staff.bottom_line_y -
                                (staff.height * staff_spaces_of_baseline_above_bottom_line as i32) /
                                staff_space_count, vec![font_codepoint, 0].as_ptr(), 1);
                            SetTextColor(device_context, BLACK.unwrap());
                        },
                        StaffObjectType::Note{log2_duration, steps_above_middle_c} =>
                        {   
                            let font_codepoint =
                            match log2_duration
                            {
                                1 => 0xe0a0,
                                0 => 0xe0a2,
                                -1 => 0xe0a3,
                                _ => 0xe0a4
                            };
                            let steps_above_bottom_line =
                                steps_above_middle_c - steps_of_bottom_line_above_middle_c;
                            let get_leger_line_metrics = || -> (i32, i32, i32)
                            {
                                let mut thickness = ((staff.height as f32 *
                                    (*window_memory).default_leger_line_thickness) /
                                    space_count as f32).round() as i32;
                                let extension = ((staff.height as f32 *
                                    (*window_memory).default_leger_line_extension) /
                                    space_count as f32).round() as i32;
                                if thickness == 0
                                {
                                    thickness = 1;
                                }
                                let note_left_edge =
                                    staff.left_edge + staff_object.distance_from_staff_start;
                                let left_edge = note_left_edge - extension;
                                let right_edge = note_left_edge +
                                    get_character_width(device_context, window_memory, staff,
                                    font_codepoint as u32) + extension;
                                (thickness, left_edge, right_edge)
                            };
                            if steps_above_bottom_line < -1
                            {
                                let (leger_line_thickness, left_edge, right_edge) =
                                    get_leger_line_metrics();
                                let mut staff_height_times_line_number = staff.height;
                                for _ in 1..=-steps_above_bottom_line / 2
                                {		    
                                    let current_line_bottom =
                                        baseline + staff_height_times_line_number / space_count;
                                    Rectangle(device_context, left_edge, current_line_bottom -
                                        leger_line_thickness, right_edge, current_line_bottom);
                                    staff_height_times_line_number += staff.height;
                                }
                            } 
                            else if steps_above_bottom_line >= 2 * staff.line_count as i8
                            {
                                let (line_thickness, left_edge, right_edge) =
                                    get_leger_line_metrics();
                                draw_lines(left_edge, right_edge, staff.line_count as i32,
                                    steps_above_bottom_line as i32 / 2 - space_count,
                                    line_thickness);                                
                            }
                            TextOutW(device_context, staff.left_edge +
                                staff_object.distance_from_staff_start, staff.bottom_line_y -
                                (staff.height * steps_above_bottom_line as i32) /
                                (2 * (staff.line_count as i32 - 1)),
                                vec![font_codepoint, 0].as_ptr(), 1);                            
                        }
                    }
                    SetTextColor(device_context, BLACK.unwrap());
                }	
                RestoreDC(device_context, original_device_context);
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
                    let cursor_rect = get_cursor_rect(address, window_memory);
                    Rectangle(device_context, cursor_rect.left, cursor_rect.top, cursor_rect.right,
                        cursor_rect.bottom);
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
    _l_param: LPARAM) -> INT_PTR
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
            FALSE as isize
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

unsafe fn init<'a>() -> (HWND, MainWindowMemory<'a>)
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
    let mut add_clef_dialog_ok = create_dialog_control_template(BS_PUSHBUTTON | WS_VISIBLE, 45, 70,
        30, 10, IDOK as u32, &button_string, &wide_char_string("OK"));
    let mut add_clef_dialog_cancel = create_dialog_control_template(BS_PUSHBUTTON | WS_VISIBLE, 75,
        70, 30, 10, IDCANCEL as u32, &button_string, &wide_char_string("Cancel"));
    let mut add_clef_dialog_shape = create_dialog_control_template(SS_LEFT | WS_CHILD | WS_VISIBLE,
        5, 0, 40, 10, 0, &static_string, &wide_char_string("Clef shape:"));
    let mut add_clef_dialog_octave = create_dialog_control_template(SS_LEFT | WS_CHILD | WS_VISIBLE,
        75, 0, 70, 10, 0, &static_string, &wide_char_string("Octave transposition:"));
    let mut add_clef_dialog_g_clef = create_dialog_control_template(BS_AUTORADIOBUTTON | WS_GROUP |
        WS_VISIBLE, 10, 20, 45, 10, IDC_ADD_CLEF_G as u32, &button_string, &wide_char_string("G"));
    let mut add_clef_dialog_c_clef = create_dialog_control_template(BS_AUTORADIOBUTTON | WS_VISIBLE,
        10, 30, 45, 10, IDC_ADD_CLEF_C as u32, &button_string, &wide_char_string("C"));
    let mut add_clef_dialog_f_clef = create_dialog_control_template(BS_AUTORADIOBUTTON | WS_VISIBLE,
        10, 40, 45, 10, IDC_ADD_CLEF_F as u32, &button_string, &wide_char_string("F"));
    let mut add_clef_dialog_unpitched_clef = create_dialog_control_template(BS_AUTORADIOBUTTON |
        WS_VISIBLE, 10, 50, 45, 10, IDC_ADD_CLEF_UNPITCHED as u32, &button_string,
        &wide_char_string("Unpitched"));
    let mut add_clef_dialog_15ma = create_dialog_control_template(BS_AUTORADIOBUTTON | WS_GROUP |
        WS_VISIBLE, 80, 15, 30, 10, IDC_ADD_CLEF_15MA as u32, &button_string,
        &wide_char_string("15ma"));
    let mut add_clef_dialog_8va = create_dialog_control_template(BS_AUTORADIOBUTTON | WS_VISIBLE,
        80, 25, 30, 10, IDC_ADD_CLEF_8VA as u32, &button_string, &wide_char_string("8va"));
    let mut add_clef_dialog_none = create_dialog_control_template(BS_AUTORADIOBUTTON | WS_VISIBLE,
        80, 35, 30, 10, IDC_ADD_CLEF_NONE as u32, &button_string, &wide_char_string("None"));
    let mut add_clef_dialog_8vb = create_dialog_control_template(BS_AUTORADIOBUTTON | WS_VISIBLE,
        80, 45, 30, 10, IDC_ADD_CLEF_8VB as u32, &button_string, &wide_char_string("8vb"));
    let mut add_clef_dialog_15mb = create_dialog_control_template(BS_AUTORADIOBUTTON | WS_VISIBLE,
        80, 55, 30, 10, IDC_ADD_CLEF_15MB as u32, &button_string, &wide_char_string("15mb"));
        ADD_CLEF_DIALOG_TEMPLATE = Some(create_dialog_template(DS_CENTER, 0, 0, 160, 100,
    wide_char_string("Add Clef"), vec![&mut add_clef_dialog_ok, &mut add_clef_dialog_cancel,
        &mut add_clef_dialog_shape, &mut add_clef_dialog_octave, &mut add_clef_dialog_g_clef,
        &mut add_clef_dialog_c_clef, &mut add_clef_dialog_f_clef,
        &mut add_clef_dialog_unpitched_clef, &mut add_clef_dialog_15ma, &mut add_clef_dialog_8va,
        &mut add_clef_dialog_none, &mut add_clef_dialog_8vb, &mut add_clef_dialog_15mb]));
    let main_window_name = wide_char_string("main");
    let cursor = LoadCursorW(null_mut(), IDC_ARROW);
    if cursor == winapi::shared::ntdef::NULL as HICON
    {
        panic!("Failed to load cursor; error code {}", GetLastError());
    }
    let instance = winapi::um::libloaderapi::GetModuleHandleW(null_mut());
    if instance == winapi::shared::ntdef::NULL as HINSTANCE
    {
        panic!("Failed to get module handle; error code {}", GetLastError());
    }
    if RegisterClassW(&WNDCLASSW{style: CS_HREDRAW | CS_OWNDC, lpfnWndProc:
        Some(main_window_proc as unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT),
        cbClsExtra: 0, cbWndExtra: std::mem::size_of::<isize>() as i32, hInstance: instance,
        hIcon: null_mut(), hCursor: cursor, hbrBackground: (COLOR_WINDOW + 1) as HBRUSH,
        lpszMenuName: null_mut(), lpszClassName: main_window_name.as_ptr()}) == 0
    {
        panic!("Failed to register main window class; error code {}", GetLastError());
    }    
    let main_window_handle = CreateWindowExW(0, main_window_name.as_ptr(),
        wide_char_string("Music Notation").as_ptr(), WS_OVERLAPPEDWINDOW | WS_VISIBLE,
        CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, null_mut(), null_mut(),
        instance, null_mut());
    if main_window_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create main window; error code {}", GetLastError());
    }
    let add_clef_button_handle = CreateWindowExW(0, button_string.as_ptr(),
        wide_char_string("Add clef").as_ptr(), WS_DISABLED | WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON |
        BS_VCENTER, 70, 0, 70, 20, main_window_handle, null_mut(), instance, null_mut());
    if add_clef_button_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create add clef button; error code {}", GetLastError());
    }
    let add_staff_button_handle = CreateWindowExW(0, button_string.as_ptr(),
        wide_char_string("Add staff").as_ptr(), WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON | BS_VCENTER,
        0, 0, 70, 20, main_window_handle, null_mut(), instance, null_mut());
    if add_staff_button_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create add staff button; error code {}", GetLastError());
    }
    if CreateWindowExW(0, static_string.as_ptr(), wide_char_string("Selected duration:").as_ptr(),
        SS_CENTER | WS_VISIBLE | WS_CHILD, 140, 0, 140, 20, main_window_handle, null_mut(),
        instance, null_mut()) == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create selected duration text; error code {}", GetLastError());
    }
    let duration_display_handle = CreateWindowExW(0, static_string.as_ptr(),
        wide_char_string("quarter").as_ptr(), WS_BORDER | WS_VISIBLE | WS_CHILD, 280, 0, 110, 20,
        main_window_handle, null_mut(), instance, null_mut());
    if duration_display_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create select duration edit; error code {}", GetLastError());
    }
    SendMessageW(duration_display_handle, WM_SETTEXT, 0,
        wide_char_string("quarter").as_ptr() as isize);
    SendMessageW(duration_display_handle, EM_NOSETFOCUS, 0, 0);
    let duration_spin_handle = CreateWindowExW(0, wide_char_string(UPDOWN_CLASS).as_ptr(),
        null_mut(), UDS_ALIGNRIGHT | WS_VISIBLE | WS_CHILD, 390, 0, 395, 20, main_window_handle,
        null_mut(), instance, null_mut());
    if duration_spin_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create select duration spin; error code {}", GetLastError());
    }
    SendMessageW(duration_spin_handle, UDM_SETPOS, 0, 3);  
    SendMessageW(duration_spin_handle, UDM_SETRANGE, 0, 31 << 16);
    let bravura_metadata_file =
        File::open("bravura_metadata.json").expect("Failed to open bravura_metadata.json");
    let bravura_metadata: serde_json::Value =
        serde_json::from_reader(bravura_metadata_file).unwrap();
    let main_window_memory = MainWindowMemory{sized_music_fonts: HashMap::new(),
        staves: Vec::new(), system_slices: Vec::new(), ghost_cursor: None,
        selection: Selection::None, add_staff_button_handle: add_staff_button_handle,
        add_clef_button_handle: add_clef_button_handle, duration_display_handle:
        duration_display_handle, duration_spin_handle: duration_spin_handle,
        default_leger_line_extension:
        bravura_metadata["engravingDefaults"]["legerLineExtension"].as_f64().unwrap() as f32,
        default_leger_line_thickness:
        bravura_metadata["engravingDefaults"]["legerLineThickness"].as_f64().unwrap() as f32,
        default_staff_line_thickness:
        bravura_metadata["engravingDefaults"]["staffLineThickness"].as_f64().unwrap() as f32};
    (main_window_handle, main_window_memory)
}

fn main()
{    
    unsafe
    {        
        let (main_window_handle, main_window_memory) = init();		
        if SetWindowLongPtrW(main_window_handle, GWLP_USERDATA,
            &main_window_memory as *const _ as isize) == 0xe050
        {
            panic!("Failed to set main window extra memory; error code {}", GetLastError());
        }
        ShowWindow(main_window_handle, SW_MAXIMIZE);
        let mut message: MSG = MSG{hwnd: null_mut(), message: 0, wParam: 0, lParam: 0, time:0,
            pt: POINT{x: 0, y: 0}};        
        while GetMessageW(&mut message, main_window_handle, 0, 0) > 0
        {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}