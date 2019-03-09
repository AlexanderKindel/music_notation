extern crate num_bigint;
extern crate num_integer;
extern crate num_rational;
extern crate winapi;

mod shared;

use shared::*;
use num_integer::Integer;
use std::ptr::null_mut;
use winapi::um::errhandlingapi::GetLastError;
use winapi::shared::basetsd::*;
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::shared::windowsx::*;
use winapi::um::commctrl::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

include!("constants.rs");

const DEFAULT_STAFF_MIDDLE_PITCH: i8 = 6;
const DISTANCE_BETWEEN_AUGMENTATION_DOTS: f32 = 0.12;
const DWLP_USER: i32 = (std::mem::size_of::<LRESULT>() + std::mem::size_of::<DLGPROC>()) as i32;
const MAX_LOG2_DURATION: i32 = 1;
const MIN_LOG2_DURATION: i32 = -10;
const TRACKBAR_MIDDLE: isize = 32767;

const STAFF_TAB_INDEX: isize = 0;
const CLEF_TAB_INDEX: isize = 1;
const KEY_SIG_TAB_INDEX: isize = 2;
const NOTE_TAB_INDEX: isize = 3;

static mut GRAY_PEN: Option<HPEN> = None;
static mut GRAY_BRUSH: Option<HBRUSH> = None;
static mut RED_PEN: Option<HPEN> = None;
static mut RED_BRUSH: Option<HBRUSH> = None;

#[derive(Clone, Copy, PartialEq)]
enum Accidental
{
    DoubleSharp,
    Sharp,
    Natural,
    Flat,
    DoubleFlat
}

#[derive(Clone, Copy, PartialEq)]
enum AccidentalPattern
{
    Flats,
    Sharps
}

#[derive(Clone, Copy)]
struct Address
{
    staff_index: usize,
    object_address: StaffObjectAddress
}

struct Clef
{
    codepoint: u16,
    baseline_offset: i8//With respect to staff middle.
}

struct DisplayedPitch
{
    pitch: Pitch,
    show_accidental: bool
}

struct FontSet
{
    full_size: HFONT,
    two_thirds_size: HFONT
}

struct Object
{
    object_type: ObjectType,
    is_selected: bool
}

struct ObjectRange
{
    slice_index: usize,
    other_objects: Vec<RangeObject>,
    slice_object: Object
}

enum ObjectType
{
    Clef
    {
        codepoint: u16,
        baseline_offset: i8//With respect to staff middle.
    },
    Duration
    {
        pitch: Option<DisplayedPitch>,

        //Denotes the power of two times the duration of a whole note of the object's duration.
        log2_duration: i8,
        augmentation_dot_count: u8
    },
    KeySignature
    {
        pattern: AccidentalPattern,
        naturals: bool,
        accidental_count: u8
    },
    None
}

#[derive(Clone, Copy)]
struct Pitch
{
    accidental: Accidental,
    steps_above_c4: i8
}

struct Project
{
    default_staff_space_height: f32,
    staff_scales: Vec<StaffScale>,
    slices: Vec<Slice>,
    staves: Vec<Staff>,
    system_left_edge: i32,
    ghost_cursor: Option<Address>,
    selection: Selection,
    selected_clef_shape: i32,
    selected_clef_octave_transposition: i32,
    control_tabs_handle: HWND,
    staff_tab_handle: HWND,
    add_staff_button_handle: HWND,
    clef_tab_handle: HWND,
    select_clef_button_handle: HWND,
    add_clef_button_handle: HWND,
    key_sig_tab_handle: HWND,
    add_key_sig_button_handle: HWND,
    note_tab_handle: HWND,
    duration_display_handle: HWND,
    duration_spin_handle: HWND,
    augmentation_dot_spin_handle: HWND,
    zoom_trackbar_handle: HWND
}

struct RangeAddress
{
    staff_index: usize,
    range_index: usize
}

struct RangeObject
{    
    object: Object,
    distance_to_slice_object: i32
}

enum Selection
{
    ActiveCursor
    {
        address: Address,
        range_floor: i8
    },
    Object(Address),
    None
}

struct Slice
{
    objects: Vec<RangeAddress>,
    rhythmic_position: Option<num_rational::Ratio<num_bigint::BigUint>>,
    distance_from_previous_slice: i32
}

struct Staff
{
    scale_index: usize,
    object_ranges: Vec<ObjectRange>,
    vertical_center: i32,
    line_count: u8
}

#[derive(Clone, Copy, PartialEq)]
struct StaffObjectAddress
{
    range_index: usize,
    object_index: Option<usize>
}

struct StaffScale
{
    name: Vec<u16>,
    value: f32
}

struct VerticalInterval
{
    top: i32,
    bottom: i32
}

fn accidental_codepoint(accidental: Accidental) -> u16
{
    match accidental
    {
        Accidental::DoubleSharp => 0xe263,
        Accidental::Sharp => 0xe262,
        Accidental::Natural => 0xe261,
        Accidental::Flat => 0xe260,
        Accidental::DoubleFlat => 0xe264
    }
}

unsafe extern "system" fn add_key_sig_dialog_proc(dialog_handle: HWND, u_msg: UINT, w_param: WPARAM,
    l_param: LPARAM) -> INT_PTR
{
    match u_msg
    {
        WM_COMMAND =>
        { 
            match LOWORD(w_param as u32) as i32
            {
                IDCANCEL =>
                {
                    EndDialog(dialog_handle, 0);
                    TRUE as isize
                },
                IDOK =>
                {      
                    let project =
                        &mut *(GetWindowLongPtrW(dialog_handle, DWLP_USER) as *mut Project);
                    if let Selection::ActiveCursor{address,..} = &mut project.selection
                    {
                        let key_sig_address =
                        key_sig_address(&address, &mut project.slices, &mut project.staves);
                        let mut maybe_next_address = next_address(
                            &project.staves[address.staff_index], &key_sig_address);
                        let mut new_pattern = AccidentalPattern::Flats;
                        let new_accidental_count = SendMessageW(GetDlgItem(dialog_handle,
                            IDC_ADD_KEY_SIG_ACCIDENTAL_COUNT), UDM_GETPOS32, 0, 0) as u8;
                        let new_naturals = new_accidental_count == 0;
                        let key_sig_accidentals;
                        if new_naturals
                        {
                            key_sig_accidentals = [Accidental::Natural; 7];
                            let mut maybe_previous_address = previous_address(
                                &project.staves[address.staff_index], &key_sig_address);
                            loop
                            {
                                if let Some(previous_address) = &maybe_previous_address
                                {
                                    if let ObjectType::KeySignature{pattern, naturals,
                                        accidental_count} =
                                        &resolve_address(&project.staves[address.staff_index],
                                        previous_address).object_type
                                    {
                                        if *naturals
                                        {
                                            remove_durationless_object(&mut project.slices,
                                                &mut project.staves, address.staff_index,
                                                &key_sig_address);
                                            if let Some(next_address) = &mut maybe_next_address
                                            {
                                                correct_address_after_removal(&key_sig_address,
                                                    next_address);
                                            }
                                            break;
                                        }
                                        resolve_address_mut(
                                            &mut project.staves[address.staff_index],
                                            &key_sig_address).object_type =
                                            ObjectType::KeySignature{pattern: *pattern,
                                            naturals: true, accidental_count: *accidental_count};
                                        break;
                                    }
                                    maybe_previous_address = self::previous_address(
                                        &project.staves[address.staff_index], previous_address);
                                }
                                else
                                {
                                    remove_durationless_object(&mut project.slices,
                                        &mut project.staves, address.staff_index, &key_sig_address);
                                    if let Some(next_address) = &mut maybe_next_address
                                    {
                                        correct_address_after_removal(&key_sig_address,
                                            next_address);
                                    }
                                    break;
                                }
                            }
                        }
                        else
                        {
                            new_pattern =
                            if SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_KEY_SIG_FLATS),
                                BM_GETCHECK, 0, 0) == BST_CHECKED as isize
                            {
                                AccidentalPattern::Flats
                            }
                            else
                            {
                                AccidentalPattern::Sharps
                            };
                            key_sig_accidentals = scale_degree_accidentals_from_key_sig(new_pattern,
                                new_naturals, new_accidental_count);
                            resolve_address_mut(&mut project.staves[address.staff_index], 
                                &key_sig_address).object_type =
                                ObjectType::KeySignature{pattern: new_pattern, naturals: false,
                                accidental_count: new_accidental_count};
                        }
                        let main_window_handle = GetWindow(dialog_handle, GW_OWNER);
                        let device_context = GetDC(main_window_handle);
                        let space_heights = staff_space_heights(&project.staves,
                            &project.staff_scales, project.default_staff_space_height);
                        respace(device_context, &mut project.slices, &mut project.staves,
                            &space_heights, address.staff_index, key_sig_address.range_index);
                        address.object_address = next_address(&project.staves[address.staff_index],
                            &key_sig_address).unwrap();
                        reset_accidental_displays(device_context, &mut project.slices,
                            &mut project.staves, &space_heights, address.staff_index,
                            &mut maybe_next_address, &key_sig_accidentals);
                        if let Some(next_address) = &maybe_next_address
                        {
                            if let ObjectType::KeySignature{pattern, naturals, accidental_count} =
                                &mut resolve_address_mut(
                                &mut project.staves[address.staff_index], next_address).object_type 
                            {
                                let remove =
                                if *naturals
                                {
                                    new_naturals
                                }
                                else
                                {
                                    (*pattern == new_pattern) &&
                                        (*accidental_count == new_accidental_count)
                                };
                                if remove
                                {
                                    let slice_index = project.staves[address.staff_index].
                                        object_ranges[next_address.range_index].slice_index;
                                    remove_durationless_object(&mut project.slices,
                                        &mut project.staves, address.staff_index, next_address);
                                    reset_distance_from_previous_slice(device_context,
                                        &mut project.slices, &mut project.staves, &space_heights,
                                        slice_index);
                                }
                            }                            
                        }
                        invalidate_work_region(main_window_handle);
                        ReleaseDC(main_window_handle, device_context);
                    }
                    EndDialog(dialog_handle, 0);
                    TRUE as isize
                },
                _ => FALSE as isize               
            }
        },
        WM_INITDIALOG =>
        {
            size_dialog(dialog_handle);
            SetWindowLongPtrW(dialog_handle, DWLP_USER, l_param);
            let accidental_count_spin_handle =
                GetDlgItem(dialog_handle, IDC_ADD_KEY_SIG_ACCIDENTAL_COUNT);
            SendMessageW(accidental_count_spin_handle, UDM_SETRANGE32, 0, 7);
            SendMessageW(accidental_count_spin_handle, UDM_SETPOS32, 0, 1);
            SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_KEY_SIG_SHARPS),
                BM_SETCHECK, BST_CHECKED, 0);
            TRUE as isize
        },
        WM_NOTIFY =>
        {
            let lpmhdr = l_param as LPNMHDR;
            if (*lpmhdr).code == UDN_DELTAPOS
            {
                let lpnmud = l_param as LPNMUPDOWN;
                let enable =
                if (*lpnmud).iPos + (*lpnmud).iDelta <= 0
                {
                    FALSE
                }
                else
                {
                    TRUE
                };
                EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_KEY_SIG_FLATS), enable);
                EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_KEY_SIG_SHARPS), enable);
            }
            0
        },
        _ => FALSE as isize
    }
}

unsafe extern "system" fn add_staff_dialog_proc(dialog_handle: HWND, u_msg: UINT, w_param: WPARAM,
    l_param: LPARAM) -> INT_PTR
{
    match u_msg
    {
        WM_COMMAND =>
        { 
            match LOWORD(w_param as u32) as i32
            {
                IDC_ADD_STAFF_ADD_SCALE =>
                {
                    let staff_scales = &mut(*(GetWindowLongPtrW(dialog_handle, DWLP_USER)
                        as *mut Project)).staff_scales;
                    let new_scale =
                        StaffScale{name: unterminated_wide_char_string("New"), value: 1.0};
                    let insertion_index = insert_staff_scale(staff_scales, new_scale);
                    let scale_list_handle = GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST);
                    SendMessageW(scale_list_handle, CB_INSERTSTRING, insertion_index,
                        to_string(&staff_scales[insertion_index]).as_ptr() as isize);
                    SendMessageW(scale_list_handle, CB_SETCURSEL, insertion_index, 0);
                    EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_STAFF_EDIT_SCALE), TRUE);
                    EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_STAFF_REMOVE_SCALE), TRUE);
                    TRUE as isize
                },
                IDC_ADD_STAFF_EDIT_SCALE =>
                {
                    let project =
                        &mut *(GetWindowLongPtrW(dialog_handle, DWLP_USER) as *mut Project);
                    let scale_index = SendMessageW(GetDlgItem(dialog_handle,
                        IDC_ADD_STAFF_SCALE_LIST), CB_GETCURSEL, 0, 0) as usize;
                    DialogBoxIndirectParamW(null_mut(),
                        EDIT_STAFF_SCALE_DIALOG_TEMPLATE.data.as_ptr() as *mut DLGTEMPLATE,
                        dialog_handle, Some(edit_staff_scale_dialog_proc),
                        &mut project.staff_scales[scale_index] as *mut _ as isize);
                    let edited_scale = project.staff_scales.remove(scale_index);
                    let edited_scale_index =
                        insert_staff_scale(&mut project.staff_scales, edited_scale);
                    let scale_list_handle = GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST);
                    SendMessageW(scale_list_handle, CB_DELETESTRING, scale_index, 0);
                    SendMessageW(scale_list_handle, CB_INSERTSTRING, edited_scale_index,
                        to_string(&project.staff_scales[edited_scale_index]).as_ptr() as isize);
                    SendMessageW(scale_list_handle, CB_SETCURSEL, edited_scale_index, 0);
                    if scale_index == edited_scale_index
                    {
                        return TRUE as isize;
                    }
                    let increment_operation: fn(&mut usize);
                    let min_index;
                    let max_index;
                    if scale_index < edited_scale_index
                    {
                        increment_operation = decrement;
                        min_index = scale_index;
                        max_index = edited_scale_index;
                    }
                    else
                    {
                        increment_operation = increment;
                        min_index = edited_scale_index;
                        max_index = scale_index;
                    }
                    for staff in &mut project.staves
                    {
                        if staff.scale_index == scale_index
                        {
                            staff.scale_index = edited_scale_index;
                        }
                        else if min_index <= staff.scale_index && staff.scale_index <= max_index
                        {
                            increment_operation(&mut staff.scale_index);
                        }
                    }
                    TRUE as isize
                },
                IDC_ADD_STAFF_REMOVE_SCALE =>
                {
                    let scale_list_handle = GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST);
                    let removal_index =
                        SendMessageW(scale_list_handle, CB_GETCURSEL, 0, 0) as usize;
                    let project =
                        &mut *(GetWindowLongPtrW(dialog_handle, DWLP_USER) as *mut Project);
                    let mut scale_is_used = false;
                    for staff_index in 0..project.staves.len()
                    {
                        if project.staves[staff_index].scale_index == removal_index
                        {
                            scale_is_used = true;
                            break;
                        }
                    }
                    let remapped_index;
                    if scale_is_used
                    {
                        let mut reassignment_candidates = vec![]; 
                        for scale_index in 0..project.staff_scales.len()
                        {
                            if scale_index == removal_index
                            {
                                continue;
                            }
                            let text: Vec<u16> = vec![0; SendMessageW(scale_list_handle,
                                CB_GETLBTEXTLEN, scale_index, 0) as usize + 1];
                            SendMessageW(scale_list_handle, CB_GETLBTEXT, scale_index,
                                text.as_ptr() as isize);
                            reassignment_candidates.push(text);
                        }
                        remapped_index = DialogBoxIndirectParamW(null_mut(),
                            REMAP_STAFF_SCALE_DIALOG_TEMPLATE.data.as_ptr() as *mut DLGTEMPLATE,
                            dialog_handle, Some(remap_staff_scale_dialog_proc),
                            &reassignment_candidates as *const _ as isize);
                        if remapped_index < 0
                        {
                            return TRUE as isize;
                        }
                    }
                    else
                    {
                        remapped_index = 0;
                    }
                    let remapped_index = remapped_index as usize;
                    project.staff_scales.remove(removal_index);
                    for staff in &mut project.staves
                    {
                        if staff.scale_index == removal_index
                        {
                            staff.scale_index = remapped_index;
                        }
                        else if staff.scale_index > removal_index
                        {
                            staff.scale_index -= 1;
                        }
                    }
                    SendMessageW(scale_list_handle, CB_DELETESTRING, removal_index, 0);
                    SendMessageW(scale_list_handle, CB_SETCURSEL, remapped_index, 0);
                    TRUE as isize
                },
                IDC_ADD_STAFF_SCALE_LIST =>
                {
                    if HIWORD(w_param as u32) as u16 == CBN_SELCHANGE
                    {
                        let enable_editing =
                        if SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST),
                            CB_GETCURSEL, 0, 0) == 0
                        {
                            FALSE
                        }
                        else
                        {
                            TRUE
                        };
                        EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_STAFF_EDIT_SCALE),
                            enable_editing);
                        EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_STAFF_REMOVE_SCALE),
                            enable_editing);
                        TRUE as isize
                    }
                    else
                    {
                        TRUE as isize
                    }
                },
                IDCANCEL =>
                {
                    EndDialog(dialog_handle, 0);
                    TRUE as isize
                },
                IDOK =>
                {
                    let project =
                        &mut *(GetWindowLongPtrW(dialog_handle, DWLP_USER) as *mut Project);
                    let vertical_center = 
                    if project.staves.len() == 0
                    {
                        135
                    }
                    else
                    {
                        project.staves[project.staves.len() - 1].vertical_center + 80
                    };
                    let scale_index = SendMessageW(GetDlgItem(dialog_handle,
                        IDC_ADD_STAFF_SCALE_LIST), CB_GETCURSEL, 0, 0) as usize;
                    let staff_index = project.staves.len();
                    project.staves.push(Staff{scale_index: scale_index,
                        object_ranges: vec![], vertical_center: vertical_center,
                        line_count: SendMessageW(GetDlgItem(dialog_handle,
                        IDC_ADD_STAFF_LINE_COUNT_SPIN), UDM_GETPOS32, 0, 0) as u8});
                    register_rhythmic_position(&mut project.slices, &mut project.staves, &mut 0,
                        num_rational::Ratio::new(num_bigint::BigUint::new(vec![]),
                        num_bigint::BigUint::new(vec![1])), staff_index, 0);
                    EndDialog(dialog_handle, 1);
                    TRUE as isize
                },
                _ => FALSE as isize               
            }
        },
        WM_CTLCOLORSTATIC =>
        {
            if l_param as HWND == GetDlgItem(dialog_handle, IDC_ADD_STAFF_LINE_COUNT_DISPLAY)
            {
                GetStockObject(WHITE_BRUSH as i32) as isize
            }
            else
            {
                FALSE as isize
            }
        },
        WM_INITDIALOG =>
        {
            size_dialog(dialog_handle);
            let line_count_spin_handle = GetDlgItem(dialog_handle, IDC_ADD_STAFF_LINE_COUNT_SPIN);
            SendMessageW(line_count_spin_handle, UDM_SETRANGE32, 1, 5);
            SendMessageW(line_count_spin_handle, UDM_SETPOS32, 0, 5);
            let scale_list_handle = GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST);
            SendMessageW(scale_list_handle, CB_ADDSTRING, 0,
                wide_char_string("Default").as_ptr() as isize);
            SetWindowLongPtrW(dialog_handle, DWLP_USER, l_param);
            let staff_scales =
                &(*(GetWindowLongPtrW(dialog_handle, DWLP_USER) as *mut Project)).staff_scales;
            for scale_index in 1..staff_scales.len()
            {
                SendMessageW(scale_list_handle, CB_ADDSTRING, 0,
                    to_string(&staff_scales[scale_index]).as_ptr() as isize);
            }
            SendMessageW(scale_list_handle, CB_SETCURSEL, 0, 0);
            EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_STAFF_EDIT_SCALE), FALSE);
            EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_STAFF_REMOVE_SCALE), FALSE);
            TRUE as isize
        },
        _ => FALSE as isize
    }
}

fn address_of_clicked_staff_object(window_handle: HWND, buffer_device_context: HDC,
    slices: &Vec<Slice>, staff: &mut Staff, staff_space_height: f32, system_left_edge: i32,
    click_x: i32, click_y: i32, zoom_factor: f32) -> Option<StaffObjectAddress>
{
    let mut x = system_left_edge;
    if click_x < to_screen_coordinate(x as f32, zoom_factor)
    {
        return None;
    }
    let zoomed_font_set = staff_font_set(zoom_factor * staff_space_height);
    let mut staff_middle_pitch = 6;
    let mut slice_index = 0;
    for range_index in 0..staff.object_ranges.len()
    {
        let object_range = &staff.object_ranges[range_index];
        while slice_index <= object_range.slice_index
        {
            x += slices[slice_index].distance_from_previous_slice;
            slice_index += 1;
        }
        for object_index in 0..object_range.other_objects.len()
        {
            let range_object = &object_range.other_objects[object_index];
            let object_x = x - range_object.distance_to_slice_object;
            if click_x < to_screen_coordinate(object_x as f32, zoom_factor)
            {
                release_font_set(&zoomed_font_set);
                return None;
            }
            draw(buffer_device_context, &zoomed_font_set, staff, staff_space_height,
                &range_object.object, object_range.slice_index, object_x, &mut staff_middle_pitch,
                zoom_factor);
            unsafe
            {
                if GetPixel(buffer_device_context, click_x, click_y) == WHITE
                {
                    cancel_selection(window_handle);
                    staff.object_ranges[range_index].other_objects[object_index].object.
                        is_selected = true;
                    release_font_set(&zoomed_font_set);
                    return Some(StaffObjectAddress{range_index: range_index,
                        object_index: Some(object_index)});
                }
            }
        }
        if click_x < to_screen_coordinate(x as f32, zoom_factor)
        {
            release_font_set(&zoomed_font_set);
            return None;
        }
        draw(buffer_device_context, &zoomed_font_set, staff, staff_space_height,
            &staff.object_ranges[range_index].slice_object, object_range.slice_index, x,
            &mut staff_middle_pitch, zoom_factor);
        unsafe
        {
            if GetPixel(buffer_device_context, click_x, click_y) == WHITE
            {
                cancel_selection(window_handle);
                staff.object_ranges[range_index].slice_object.is_selected = true;
                release_font_set(&zoomed_font_set);
                return Some(StaffObjectAddress{range_index: range_index, object_index: None});
            }
        }
    }
    release_font_set(&zoomed_font_set);
    None
}

fn bottom_line_pitch(staff_line_count: u8, middle_pitch: i8) -> i8
{
    middle_pitch - staff_line_count as i8 + 1
}

fn cancel_selection(main_window_handle: HWND)
{
    let project = project_memory(main_window_handle);
    match &project.selection
    {
        Selection::ActiveCursor{..} =>
        {
            invalidate_work_region(main_window_handle);
            unsafe
            {
                EnableWindow(project.select_clef_button_handle, FALSE);
            }
        }
        Selection::Object(address) =>
        {
            resolve_address_mut(&mut project.staves[address.staff_index],
                &address.object_address).is_selected = false;
            invalidate_work_region(main_window_handle);
            unsafe
            {
                EnableWindow(project.add_clef_button_handle, FALSE);
                EnableWindow(project.add_key_sig_button_handle, FALSE);
            }
        },
        Selection::None => ()
    }        
    project.selection = Selection::None;
}

fn character_width(device_context: HDC, font: HFONT, codepoint: u32) -> i32
{
    unsafe
    {
        let old_font = SelectObject(device_context, font as *mut winapi::ctypes::c_void);
        let mut abc_array: [ABC; 1] = [ABC{abcA: 0, abcB: 0, abcC: 0}];
        GetCharABCWidthsW(device_context, codepoint, codepoint + 1, abc_array.as_mut_ptr());
        SelectObject(device_context, old_font);
        abc_array[0].abcB as i32
    }
}

fn clef_baseline(staff: &Staff, staff_space_height: f32,
    steps_of_clef_baseline_above_middle: i8) -> f32
{
    y_of_steps_above_bottom_line(staff, staff_space_height,
        staff.line_count as i8 - 1 + steps_of_clef_baseline_above_middle)
}

fn clef_from_selection(shape: i32, octave_transposition: i32) -> Clef
{
    let baseline_offset;
    let codepoint =
    match shape
    {
        IDC_SELECT_CLEF_C =>
        {
            baseline_offset = 0;
            match octave_transposition
            {
                IDC_SELECT_CLEF_NONE => 0xe05c,
                IDC_SELECT_CLEF_8VB => 0xe05d,
                _ => panic!("Unknown clef octave transposition.")
            }
        },
        IDC_SELECT_CLEF_F =>
        {
            baseline_offset = 2;
            match octave_transposition
            {
                IDC_SELECT_CLEF_15MA => 0xe066,
                IDC_SELECT_CLEF_8VA => 0xe065,
                IDC_SELECT_CLEF_NONE => 0xe062,
                IDC_SELECT_CLEF_8VB => 0xe064,
                IDC_SELECT_CLEF_15MB => 0xe063,
                _ => panic!("Unknown clef octave transposition.")
            }
        },
        IDC_SELECT_CLEF_G =>
        {
            baseline_offset = -2;
            match octave_transposition
            {
                IDC_SELECT_CLEF_15MA => 0xe054,
                IDC_SELECT_CLEF_8VA => 0xe053,
                IDC_SELECT_CLEF_NONE => 0xe050,
                IDC_SELECT_CLEF_8VB => 0xe052,
                IDC_SELECT_CLEF_15MB => 0xe051,
                _ => panic!("Unknown clef octave transposition.")
            }
        },
        IDC_SELECT_CLEF_UNPITCHED =>
        {
            baseline_offset = 0;
            0xe069
        },
        _ => panic!("Unknown clef shape.")
    };
    Clef{codepoint: codepoint, baseline_offset: baseline_offset}
}

unsafe extern "system" fn clef_tab_proc(window_handle: HWND, u_msg: UINT, w_param: WPARAM,
    l_param: LPARAM, _id_subclass: UINT_PTR, _dw_ref_data: DWORD_PTR) -> LRESULT
{
    match u_msg
    {
        WM_COMMAND =>
        {
            if HIWORD(w_param as u32) == BN_CLICKED
            {
                let main_window_handle = GetParent(GetParent(window_handle));
                SetFocus(main_window_handle);
                let project = project_memory(main_window_handle);
                if l_param == project.add_clef_button_handle as isize
                {
                    if let Selection::ActiveCursor{address,..} = &mut project.selection
                    {
                        let clef = clef_from_selection(project.selected_clef_shape,
                            project.selected_clef_octave_transposition);
                        let clef_address = insert_clef(address, &mut project.slices,
                            &mut project.staves, clef.codepoint, clef.baseline_offset);
                        let space_heights = staff_space_heights(&project.staves,
                            &project.staff_scales, project.default_staff_space_height);
                        let device_context = GetDC(main_window_handle);
                        respace(device_context, &mut project.slices, &mut project.staves,
                            &space_heights, address.staff_index, clef_address.range_index);
                        address.object_address = next_address(&project.staves[address.staff_index],
                            &clef_address).unwrap();
                        invalidate_work_region(main_window_handle);
                        ReleaseDC(main_window_handle, device_context);
                    }
                    return 0;
                }
                if l_param == project.select_clef_button_handle as isize
                {
                    DialogBoxIndirectParamW(null_mut(),
                        SELECT_CLEF_DIALOG_TEMPLATE.data.as_ptr() as *const DLGTEMPLATE,
                        main_window_handle, Some(select_clef_dialog_proc),
                        project as *mut _ as isize);
                    InvalidateRect(GetParent(window_handle), &RECT{left: 85, top: 0, right: 110,
                        bottom: 65}, TRUE);
                    return 0;
                }
            }
        },
        WM_PAINT =>
        {
            let device_context = GetDC(window_handle);
            SaveDC(device_context);
            SetBkMode(device_context, TRANSPARENT as i32);
            SetTextAlign(device_context, TA_BASELINE);
            SelectObject(device_context, GetWindowLongPtrW(window_handle, GWLP_USERDATA)
                as *mut winapi::ctypes::c_void);
            let project = project_memory(GetParent(GetParent(window_handle)));
            let clef = clef_from_selection(project.selected_clef_shape,
                project.selected_clef_octave_transposition);
            TextOutW(device_context, 85, 20 - 2 * clef.baseline_offset as i32,
                vec![clef.codepoint].as_ptr(), 1);
            RestoreDC(device_context, -1);
            ReleaseDC(window_handle, device_context);
        }
        _ => ()
    }
    DefWindowProcW(window_handle, u_msg, w_param, l_param)
}

fn correct_address_after_removal(removal_address: &StaffObjectAddress,
    later_address: &mut StaffObjectAddress)
{
    if let Some(_) = removal_address.object_index
    {
        if removal_address.range_index == later_address.range_index
        {
            if let Some(object_index) = &mut later_address.object_index
            {
                *object_index -= 1;
            }
        }
    }
    else
    {
        later_address.range_index -= 1;
    }
}

fn cursor_x(slices: &Vec<Slice>, staff: &Staff, system_left_edge: i32,
    address: &StaffObjectAddress) -> i32
{
    let mut x = system_left_edge;
    for slice_index in 0..=staff.object_ranges[address.range_index].slice_index
    {
        x += slices[slice_index].distance_from_previous_slice;
    }
    if let Some(object_index) = address.object_index
    {
        x -= staff.object_ranges[address.range_index].other_objects[object_index].
            distance_to_slice_object;
    }
    x
}

fn decrement(index: &mut usize)
{
    *index -= 1;
}

fn decrement_range_floor(range_floor: &mut i8, decrement_size: u8)
{
    if *range_floor < i8::min_value() + decrement_size as i8
    {
        *range_floor = i8::min_value();
    }
    else
    {
        *range_floor -= decrement_size as i8;
    }
}

fn default_pitch_of_steps_above_c4(staff: &Staff, address: &StaffObjectAddress,
    steps_above_c4: i8) -> DisplayedPitch
{
    let mut accidental = Accidental::Natural;
    let mut pitch_in_other_octaves = vec![];
    let mut maybe_previous_address = previous_address(staff, address);
    loop
    {
        if let Some(previous_address) = &maybe_previous_address
        {
            match &resolve_address(staff, &previous_address).object_type
            {
                ObjectType::Duration{pitch,..} =>
                {
                    if let Some(displayed_pitch) = pitch
                    {
                        if displayed_pitch.pitch.steps_above_c4 == steps_above_c4
                        {
                            accidental = displayed_pitch.pitch.accidental;
                            break;
                        }
                        else if displayed_pitch.pitch.steps_above_c4 % 7 == steps_above_c4 % 7
                        {
                            let mut pitch_index = 0;
                            loop
                            {
                                if pitch_index == pitch_in_other_octaves.len()
                                {
                                    pitch_in_other_octaves.push(&displayed_pitch.pitch);
                                    break;
                                }
                                let pitch = &mut pitch_in_other_octaves[pitch_index];
                                if pitch.steps_above_c4 == displayed_pitch.pitch.steps_above_c4
                                {
                                    break;
                                }
                                pitch_index += 1;
                            }
                        }
                    }
                },
                ObjectType::KeySignature{pattern, naturals, accidental_count,..} =>
                {
                    accidental = scale_degree_accidentals_from_key_sig(*pattern, *naturals,
                        *accidental_count)[steps_above_c4 as usize % 7];
                    break;
                },
                _ => ()
            }
            maybe_previous_address = self::previous_address(staff, previous_address);
        }
        else
        {
            break;
        }
    }
    let mut show_accidental = false;
    for pitch in pitch_in_other_octaves
    {
        if pitch.accidental != accidental
        {
            show_accidental = true;
            break;
        }
    }
    DisplayedPitch{pitch: Pitch{accidental: accidental, steps_above_c4: steps_above_c4},
        show_accidental: show_accidental}
}

fn delete_object(window_handle: HWND, slices: &mut Vec<Slice>, staves: &mut Vec<Staff>,
    staff_scales: &Vec<StaffScale>, default_staff_space_height: f32, address: &mut Address)
{
    let device_context =
    unsafe
    {
        GetDC(window_handle)
    };
    let space_heights = staff_space_heights(staves, staff_scales, default_staff_space_height);
    let object =
        &mut resolve_address_mut(&mut staves[address.staff_index], &address.object_address);
    match &mut object.object_type
    {
        ObjectType::Duration{pitch,..} =>
        {
            *pitch = None;
            object.is_selected = false;
        },
        ObjectType::KeySignature{..} =>
        {
            let mut new_address =
                next_address(&staves[address.staff_index], &address.object_address).unwrap();
            remove_durationless_object(slices, staves, address.staff_index,
                &address.object_address);
            correct_address_after_removal(&address.object_address, &mut new_address);
            address.object_address = new_address;
            reset_accidental_displays_from_previous_key_sig(device_context, slices, staves,
                &space_heights, address);
        },
        ObjectType::None => return,
        _ =>
        {
            let mut new_address =
                next_address(&staves[address.staff_index], &address.object_address).unwrap();
            remove_durationless_object(slices, staves, address.staff_index,
                &address.object_address);
            correct_address_after_removal(&address.object_address, &mut new_address);
            address.object_address = new_address;
        }
    }
    respace(device_context, slices, staves, &space_heights, address.staff_index,
        address.object_address.range_index);
    invalidate_work_region(window_handle);
    unsafe
    {
        ReleaseDC(window_handle, device_context);
    }
}

fn draw(device_context: HDC, zoomed_font_set: &FontSet, staff: &Staff, staff_space_height: f32,
    object: &Object, slice_index: usize, x: i32, staff_middle_pitch: &mut i8, zoom_factor: f32)
{
    match &object.object_type
    {
        ObjectType::Clef{codepoint, baseline_offset} =>
        {
            let font =
            if slice_index == 0
            {
                zoomed_font_set.full_size
            }
            else
            {
                zoomed_font_set.two_thirds_size
            };
            draw_clef(device_context, font, staff, staff_space_height, *codepoint, *baseline_offset,
                x, staff_middle_pitch, zoom_factor);
        },
        ObjectType::Duration{pitch, log2_duration, augmentation_dot_count} =>
        {
            let duration_codepoint = duration_codepoint(pitch, *log2_duration);
            let unzoomed_font = staff_font(staff_space_height, 1.0);
            let mut duration_left_edge = x - left_edge_to_origin_distance(device_context,
                unzoomed_font, staff_space_height, pitch, *log2_duration);
            let duration_right_edge;
            let duration_y;
            let augmentation_dot_y;
            if let Some(displayed_pitch) = pitch
            {        
                let steps_above_bottom_line = displayed_pitch.pitch.steps_above_c4 -
                    bottom_line_pitch(staff.line_count, *staff_middle_pitch);
                duration_y = y_of_steps_above_bottom_line(staff, staff_space_height,
                    steps_above_bottom_line);
                if displayed_pitch.show_accidental
                {
                    let accidental_codepoint =
                        accidental_codepoint(displayed_pitch.pitch.accidental);
                    draw_character(device_context, zoomed_font_set.full_size, accidental_codepoint,
                        duration_left_edge as f32, duration_y, zoom_factor);
                    duration_left_edge += character_width(device_context, unzoomed_font,
                        accidental_codepoint as u32);
                }
                augmentation_dot_y =
                if steps_above_bottom_line % 2 == 0
                {
                    y_of_steps_above_bottom_line(staff, staff_space_height,
                        steps_above_bottom_line + 1)
                }
                else
                {
                    duration_y
                };
                if *log2_duration < 0
                {             
                    let stem_left_edge;
                    let stem_right_edge;
                    let mut stem_bottom;
                    let mut stem_top;
                    let space_count = staff.line_count as i8 - 1;
                    if space_count > steps_above_bottom_line
                    {
                        stem_top = y_of_steps_above_bottom_line(staff, staff_space_height,
                            std::cmp::max(steps_above_bottom_line + 7, space_count));
                        if *log2_duration == -1
                        {
                            stem_right_edge = x as f32 +
                                staff_space_height * BRAVURA_METADATA.half_notehead_stem_up_se.x;
                            stem_left_edge = stem_right_edge -
                                staff_space_height * BRAVURA_METADATA.stem_thickness;
                            stem_bottom = duration_y as f32 -
                                staff_space_height * BRAVURA_METADATA.half_notehead_stem_up_se.y;                        
                        }
                        else
                        {
                            stem_right_edge = x as f32 +
                                staff_space_height * BRAVURA_METADATA.black_notehead_stem_up_se.x;
                            stem_left_edge = stem_right_edge -
                                staff_space_height * BRAVURA_METADATA.stem_thickness;
                            stem_bottom = duration_y as f32 -
                                staff_space_height * BRAVURA_METADATA.black_notehead_stem_up_se.y;
                            if *log2_duration == -3
                            {
                                draw_character(device_context, zoomed_font_set.full_size, 0xe240,
                                    stem_left_edge, stem_top, zoom_factor);
                            }
                            else if *log2_duration < -3
                            {
                                draw_character(device_context, zoomed_font_set.full_size, 0xe242,
                                    stem_left_edge, stem_top, zoom_factor);
                                let flag_spacing = staff_space_height *
                                    (BRAVURA_METADATA.beam_spacing +
                                    BRAVURA_METADATA.beam_thickness);
                                for _ in 0..-log2_duration - 4
                                {
                                    stem_top -= flag_spacing;
                                    draw_character(device_context, zoomed_font_set.full_size,
                                        0xe250, stem_left_edge, stem_top, zoom_factor);
                                }
                            }
                        }
                    }
                    else
                    {
                        stem_bottom = y_of_steps_above_bottom_line(staff, staff_space_height,
                            std::cmp::min(steps_above_bottom_line - 7, space_count));
                        if *log2_duration == -1
                        {
                            stem_left_edge = x as f32 +
                                staff_space_height * BRAVURA_METADATA.half_notehead_stem_down_nw.x;
                            stem_top = duration_y as f32 -
                                staff_space_height * BRAVURA_METADATA.half_notehead_stem_down_nw.y;
                        }
                        else
                        {
                            stem_left_edge = x as f32 +
                                staff_space_height * BRAVURA_METADATA.black_notehead_stem_down_nw.x;
                            stem_top = duration_y as f32 -
                                staff_space_height * BRAVURA_METADATA.black_notehead_stem_down_nw.y;
                            if *log2_duration == -3
                            {
                                draw_character(device_context, zoomed_font_set.full_size, 0xe241,
                                    stem_left_edge, stem_bottom, zoom_factor);
                            }
                            else if *log2_duration < -3
                            {
                                draw_character(device_context, zoomed_font_set.full_size, 0xe243,
                                    stem_left_edge, stem_bottom, zoom_factor);
                                let flag_spacing = staff_space_height * 
                                    (BRAVURA_METADATA.beam_spacing +
                                    BRAVURA_METADATA.beam_thickness);
                                for _ in 0..-log2_duration - 4
                                {      
                                    stem_bottom += flag_spacing;
                                    draw_character(device_context, zoomed_font_set.full_size,
                                        0xe251, stem_left_edge, stem_bottom, zoom_factor);
                                }
                            }                  
                        }
                        stem_right_edge =
                            stem_left_edge + staff_space_height * BRAVURA_METADATA.stem_thickness;
                    }
                    unsafe
                    {
                        Rectangle(device_context, to_screen_coordinate(stem_left_edge, zoom_factor),
                            to_screen_coordinate(stem_top, zoom_factor),
                            to_screen_coordinate(stem_right_edge, zoom_factor),
                            to_screen_coordinate(stem_bottom, zoom_factor));
                    }
                }
                duration_right_edge = duration_left_edge +
                    character_width(device_context, unzoomed_font, duration_codepoint as u32);
                let leger_extension = staff_space_height * BRAVURA_METADATA.leger_line_extension;
                let leger_thickness = staff_space_height * BRAVURA_METADATA.leger_line_thickness;
                let leger_left_edge = duration_left_edge as f32 - leger_extension;
                let leger_right_edge = duration_right_edge as f32 + leger_extension;
                if steps_above_bottom_line < -1
                {
                    for line_index in steps_above_bottom_line / 2..0
                    {
                        draw_horizontal_line(device_context, leger_left_edge, leger_right_edge,
                            y_of_steps_above_bottom_line(staff, staff_space_height,
                            2 * line_index as i8), leger_thickness, zoom_factor);
                    }
                }
                else if steps_above_bottom_line >= 2 * staff.line_count as i8
                {
                    for line_index in staff.line_count as i8..=steps_above_bottom_line / 2
                    {
                        draw_horizontal_line(device_context, leger_left_edge, leger_right_edge,
                            y_of_steps_above_bottom_line(staff, staff_space_height, 2 * line_index),
                            leger_thickness, zoom_factor);
                    }
                }
            }
            else
            {
                let spaces_above_bottom_line =
                if *log2_duration == 0
                {
                    if staff.line_count == 1
                    {
                        0
                    }
                    else
                    {
                        staff.line_count / 2 + staff.line_count % 2
                    }
                }
                else
                {                        
                    staff.line_count / 2 + staff.line_count % 2 - 1
                }; 
                duration_right_edge = duration_left_edge +
                    character_width(device_context, unzoomed_font, duration_codepoint as u32);          
                duration_y = y_of_steps_above_bottom_line(staff, staff_space_height,
                    2 * spaces_above_bottom_line as i8);
                augmentation_dot_y = y_of_steps_above_bottom_line(staff, staff_space_height,
                    2 * spaces_above_bottom_line as i8 + 1);
            }
            let dot_separation = staff_space_height * DISTANCE_BETWEEN_AUGMENTATION_DOTS;
            let mut next_dot_left_edge = duration_right_edge as f32;
            let dot_offset =
                dot_separation + character_width(device_context, unzoomed_font, 0xe1e7) as f32;
            draw_character(device_context, zoomed_font_set.full_size, duration_codepoint,
                duration_left_edge as f32, duration_y, zoom_factor);        
            for _ in 0..*augmentation_dot_count
            {
                draw_character(device_context, zoomed_font_set.full_size, 0xe1e7,
                    next_dot_left_edge as f32, augmentation_dot_y, zoom_factor);
                next_dot_left_edge += dot_offset;
            }
            unsafe
            {
                DeleteObject(unzoomed_font as *mut winapi::ctypes::c_void);
            }
        },
        ObjectType::KeySignature{pattern, naturals, accidental_count,..} =>
        {
            let codepoint;
            let stride;
            let mut steps_of_accidental_above_floor;
            let steps_of_floor_above_middle;
            match pattern
            {
                AccidentalPattern::Flats =>
                {
                    if *naturals
                    {
                        codepoint = 0xe261;
                    }
                    else
                    {
                        codepoint = 0xe260;
                    }   
                    stride = 3;
                    let steps_of_middle_above_g = (*staff_middle_pitch + 3) % 7;
                    if steps_of_middle_above_g > 4
                    {
                        steps_of_floor_above_middle = 1 - steps_of_middle_above_g;
                        steps_of_accidental_above_floor = 1;
                    }
                    else
                    {
                        steps_of_floor_above_middle = -1 - steps_of_middle_above_g;
                        steps_of_accidental_above_floor = 3;
                    }
                },
                AccidentalPattern::Sharps =>
                {
                    if *naturals
                    {
                        codepoint = 0xe261;
                    }
                    else
                    {
                        codepoint = 0xe262;
                    }
                    stride = 4;
                    let steps_of_middle_above_b = (*staff_middle_pitch + 1) % 7;
                    if steps_of_middle_above_b > 4
                    {
                        steps_of_floor_above_middle = 4 - steps_of_middle_above_b;
                        steps_of_accidental_above_floor = 0;
                    }
                    else
                    {
                        steps_of_floor_above_middle = -1 - steps_of_middle_above_b;
                        steps_of_accidental_above_floor = 5;
                    }
                }
            }
            let steps_of_floor_above_bottom_line =
                steps_of_floor_above_middle + staff.line_count as i8 - 1;
            let unzoomed_font = staff_font(staff_space_height, 1.0);
            let accidental_width =
                character_width(device_context, unzoomed_font, codepoint);
            unsafe
            {
                DeleteObject(unzoomed_font as *mut winapi::ctypes::c_void);
            }
            let mut x = x;
            for _ in 0..*accidental_count
            {
                draw_character(device_context, zoomed_font_set.full_size, codepoint as u16,
                    x as f32, y_of_steps_above_bottom_line(staff, staff_space_height,
                    steps_of_accidental_above_floor + steps_of_floor_above_bottom_line),
                    zoom_factor);
                steps_of_accidental_above_floor = (steps_of_accidental_above_floor + stride) % 7;
                x += accidental_width;
            }
        },
        ObjectType::None => ()
    }
}

fn draw_character(device_context: HDC, zoomed_font: HFONT, codepoint: u16, x: f32, y: f32,
    zoom_factor: f32)
{
    unsafe
    {
        SelectObject(device_context, zoomed_font as *mut winapi::ctypes::c_void);
        TextOutW(device_context, to_screen_coordinate(x, zoom_factor),
            to_screen_coordinate(y, zoom_factor), vec![codepoint, 0].as_ptr(), 1);
    }
}

fn draw_clef(device_context: HDC, zoomed_font: HFONT, staff: &Staff, staff_space_height: f32,
    codepoint: u16, steps_of_baseline_above_middle: i8, x: i32, staff_middle_pitch: &mut i8,
    zoom_factor: f32)
{
    *staff_middle_pitch = self::staff_middle_pitch(codepoint, steps_of_baseline_above_middle);
    draw_character(device_context, zoomed_font, codepoint, x as f32,
        clef_baseline(staff, staff_space_height, steps_of_baseline_above_middle), zoom_factor);
}

fn draw_horizontal_line(device_context: HDC, left_end: f32, right_end: f32, vertical_center: f32,
    thickness: f32, zoom_factor: f32)
{
    let vertical_bounds = horizontal_line_vertical_bounds(vertical_center, thickness, zoom_factor);
    unsafe
    {
        Rectangle(device_context, to_screen_coordinate(left_end, zoom_factor),
            vertical_bounds.top, to_screen_coordinate(right_end, zoom_factor),
            vertical_bounds.bottom);
    }
}

fn draw_with_highlight(device_context: HDC, zoomed_font_set: &FontSet, staff: &Staff,
    staff_space_height: f32, object: &Object, slice_index: usize, x: i32,
    staff_middle_pitch: &mut i8, zoom_factor: f32)
{
    if object.is_selected
    {
        unsafe
        {
            SetTextColor(device_context, RED);
            draw(device_context, zoomed_font_set, staff, staff_space_height, object, slice_index, x,
                staff_middle_pitch, zoom_factor);
            SetTextColor(device_context, BLACK);
        }
    }
    else
    {
        draw(device_context, zoomed_font_set, staff, staff_space_height, object, slice_index, x,
            staff_middle_pitch, zoom_factor);
    }
}

fn duration_codepoint(pitch: &Option<DisplayedPitch>, log2_duration: i8) -> u16
{
    match pitch
    {
        Some(_) =>
        {
            match log2_duration
            {
                1 => 0xe0a0,
                0 => 0xe0a2,
                -1 => 0xe0a3,
                _ => 0xe0a4
            }
        },
        None =>
        {
            (0xe4e3 - log2_duration as i32) as u16
        }
    }
}

fn duration_width(staff_space_height: f32, log2_duration: i8, augmentation_dot_count: u8) -> i32
{
    if augmentation_dot_count == 0
    {
        return (WHOLE_NOTE_WIDTH * staff_space_height *
            DURATION_RATIO.powi(log2_duration as i32)).round() as i32;
    }
    let whole_notes_long = whole_notes_long(log2_duration, augmentation_dot_count);
    let mut division = whole_notes_long.numer().div_rem(whole_notes_long.denom());
    let mut duration_float = division.0.to_bytes_le()[0] as f32;
    let zero = num_bigint::BigUint::new(vec![]);
    let two = num_bigint::BigUint::new(vec![2]);
    let mut place_value = 2.0;
    while place_value > 0.0
    {
        division = (&two * division.1).div_rem(whole_notes_long.denom());
        duration_float += division.0.to_bytes_le()[0] as f32 / place_value;
        if division.1 == zero
        {
            break;
        }
        place_value *= 2.0;
    }
    (WHOLE_NOTE_WIDTH * staff_space_height * DURATION_RATIO.powf(duration_float.log2())).
        round() as i32
}

fn edit_staff_scale_dialog_memory<'a>(dialog_handle: HWND) -> &'a mut StaffScale
{
    unsafe
    {
        &mut *(GetWindowLongPtrW(dialog_handle, DWLP_USER) as *mut StaffScale)
    }
}

unsafe extern "system" fn edit_staff_scale_dialog_proc(dialog_handle: HWND, u_msg: UINT,
    w_param: WPARAM, l_param: LPARAM) -> INT_PTR
{
    match u_msg
    {
        WM_COMMAND =>
        { 
            match LOWORD(w_param as u32) as i32
            {
                IDCANCEL =>
                {
                    EndDialog(dialog_handle, 0);
                    TRUE as isize
                },
                IDOK =>
                {      
                    let value_edit = GetDlgItem(dialog_handle, IDC_EDIT_STAFF_SCALE_VALUE);
                    let value_length =
                        SendMessageW(value_edit, WM_GETTEXTLENGTH, 0, 0) as usize + 1;
                    let value: Vec<u16> = vec![0; value_length];
                    SendMessageW(value_edit, WM_GETTEXT, value_length, value.as_ptr() as isize);
                    use std::os::windows::prelude::*;
                    if let Ok(ref mut value) = std::ffi::OsString::from_wide(&value).into_string()
                    {
                        value.pop();
                        if let Ok(value) = value.parse::<f32>()
                        {
                            if value < 0.0
                            {
                                MessageBoxW(dialog_handle, wide_char_string(
                                    "The value must be a non-negative decimal number.").as_ptr(),
                                    null_mut(), MB_OK);
                                return TRUE as isize;
                            }
                            edit_staff_scale_dialog_memory(dialog_handle).value = value;
                            let name_edit = GetDlgItem(dialog_handle, IDC_EDIT_STAFF_SCALE_NAME);
                            let name_length =
                                SendMessageW(name_edit, WM_GETTEXTLENGTH, 0, 0) as usize + 1;
                            let mut name: Vec<u16> = vec![0; name_length];
                            SendMessageW(name_edit, WM_GETTEXT, name_length,
                                name.as_ptr() as isize);
                            name.pop();
                            edit_staff_scale_dialog_memory(dialog_handle).name = name;
                            EndDialog(dialog_handle, 0);
                            return TRUE as isize;
                        }
                    }
                    MessageBoxW(dialog_handle, wide_char_string(
                        "The value must be a non-negative decimal number.").as_ptr(),
                        null_mut(), MB_OK);
                    TRUE as isize
                },
                _ => FALSE as isize               
            }
        },
        WM_INITDIALOG =>
        {
            size_dialog(dialog_handle);
            SetWindowLongPtrW(dialog_handle, DWLP_USER, l_param);
            let staff_scale = edit_staff_scale_dialog_memory(dialog_handle);
            let mut name = staff_scale.name.clone();
            name.push(0);
            SendMessageW(GetDlgItem(dialog_handle, IDC_EDIT_STAFF_SCALE_NAME), WM_SETTEXT, 0,
                name.as_ptr() as isize);
            SendMessageW(GetDlgItem(dialog_handle, IDC_EDIT_STAFF_SCALE_VALUE), WM_SETTEXT, 0,
                wide_char_string(&staff_scale.value.to_string()).as_ptr() as isize);
            TRUE as isize
        },
        _ => FALSE as isize
    }
}

fn horizontal_line_vertical_bounds(vertical_center: f32, thickness: f32, zoom_factor: f32) ->
    VerticalInterval
{
    let bottom = vertical_center + thickness / 2.0;
    let mut top = to_screen_coordinate(bottom - thickness, zoom_factor);
    let bottom = to_screen_coordinate(bottom, zoom_factor);
    if top == bottom
    {
        top -= 1;
    }
    VerticalInterval{top: top, bottom: bottom}
}

fn increment(index: &mut usize)
{
    *index += 1;
}

fn increment_range_indices(staff: &Staff, slices: &mut Vec<Slice>, range_address: &RangeAddress,
    increment_operation: fn(&mut usize))
{
    for index in range_address.range_index..staff.object_ranges.len()
    {
        let slice_objects = &mut slices[staff.object_ranges[index].slice_index].objects;
        for object_address_index in 0..slice_objects.len()
        {
            if slice_objects[object_address_index].staff_index == range_address.staff_index
            {
                increment_operation(&mut slice_objects[object_address_index].range_index);
                break;
            }
        }
    }
}

fn increment_slice_indices(slices: &mut Vec<Slice>, staves: &mut Vec<Staff>,
    starting_slice_index: usize, increment_operation: fn(&mut usize))
{
    for slice_index in starting_slice_index..slices.len()
    {
        for address in &slices[slice_index].objects
        {
            increment_operation(&mut staves[address.staff_index].
                object_ranges[address.range_index].slice_index);
        }
    }
}

unsafe fn init() -> (HWND, Project)
{
    let gray = RGB(127, 127, 127);
    GRAY_PEN = Some(CreatePen(PS_SOLID as i32, 1, gray));
    GRAY_BRUSH = Some(CreateSolidBrush(gray));
    RED_PEN = Some(CreatePen(PS_SOLID as i32, 1, RED));
    RED_BRUSH = Some(CreateSolidBrush(RED));
    let button_string = wide_char_string("button");
    let static_string = wide_char_string("static");    
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
    let common_controls =
        INITCOMMONCONTROLSEX{dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
        dwICC: ICC_BAR_CLASSES | ICC_STANDARD_CLASSES | ICC_TAB_CLASSES | ICC_UPDOWN_CLASS};
    InitCommonControlsEx(&common_controls as *const _);
    if RegisterClassW(&WNDCLASSW{style: CS_HREDRAW | CS_OWNDC, lpfnWndProc:
        Some(main_window_proc as unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT),
        cbClsExtra: 0, cbWndExtra: std::mem::size_of::<usize>() as i32, hInstance: instance,
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
    let mut metrics: NONCLIENTMETRICSA = std::mem::uninitialized();
    metrics.cbSize = std::mem::size_of::<NONCLIENTMETRICSA>() as u32;
    SystemParametersInfoA(SPI_GETNONCLIENTMETRICS, metrics.cbSize,
        &mut metrics as *mut _ as *mut winapi::ctypes::c_void, 0);
    let text_font = CreateFontIndirectA(&metrics.lfMessageFont as *const _);
    let control_tabs_handle = CreateWindowExW(0, wide_char_string("SysTabControl32").as_ptr(),
        null_mut(), WS_CHILD | WS_VISIBLE, 0, 0, 0, 0, main_window_handle, null_mut(), instance,
        null_mut());
    if control_tabs_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create control tabs; error code {}", GetLastError());
    }
    SendMessageW(control_tabs_handle, WM_SETFONT, text_font as usize, 0);
    let tab_top = 25;
    let staff_tab = TCITEMW{mask: TCIF_TEXT, dwState: 0, dwStateMask: 0,
        pszText: wide_char_string("Staves").as_mut_ptr(), cchTextMax: 0, iImage: -1, lParam: 0};
    SendMessageW(control_tabs_handle, TCM_INSERTITEMW, STAFF_TAB_INDEX as usize,
        &staff_tab as *const _ as isize);
    let staff_tab_handle = CreateWindowExW(0, static_string.as_ptr(), null_mut(),
        WS_CHILD | WS_VISIBLE, 0, tab_top, 500, 40, control_tabs_handle, null_mut(), instance,
        null_mut());
    if staff_tab_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create staff tab; error code {}", GetLastError());
    }
    SetWindowSubclass(staff_tab_handle, Some(staff_tab_proc), 0, 0);
    let add_staff_button_handle = CreateWindowExW(0, button_string.as_ptr(),
        wide_char_string("Add staff").as_ptr(), WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON | BS_VCENTER,
        0, 0, 55, 20, staff_tab_handle, null_mut(), instance, null_mut());
    if add_staff_button_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create add staff button; error code {}", GetLastError());
    } 
    SendMessageW(add_staff_button_handle, WM_SETFONT, text_font as usize, 0);
    let clef_tab = TCITEMW{mask: TCIF_TEXT, dwState: 0, dwStateMask: 0,
        pszText: wide_char_string("Clefs").as_mut_ptr(), cchTextMax: 0, iImage: -1, lParam: 0};
    SendMessageW(control_tabs_handle, TCM_INSERTITEMW, CLEF_TAB_INDEX as usize,
        &clef_tab as *const _ as isize);
    let clef_tab_handle = CreateWindowExW(0, static_string.as_ptr(), null_mut(), WS_CHILD, 0, 
        tab_top, 500, 40, control_tabs_handle, null_mut(), instance, null_mut());
    if clef_tab_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create clef tab; error code {}", GetLastError());
    }
    SetWindowSubclass(clef_tab_handle, Some(clef_tab_proc), 0, 0);
    SetWindowLongPtrW(clef_tab_handle, GWLP_USERDATA, CreateFontW(-16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, wide_char_string("Bravura").as_ptr()) as isize);
    let clef_selection_label_handle = CreateWindowExW(0, static_string.as_ptr(),
        wide_char_string("Selected clef:").as_ptr(), WS_CHILD | WS_VISIBLE, 5, 10, 70, 20,
        clef_tab_handle, null_mut(), instance, null_mut());
    if clef_selection_label_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create clef selection label; error code {}", GetLastError());
    }
    SendMessageW(clef_selection_label_handle, WM_SETFONT, text_font as usize, 0);
    let select_clef_button_handle = CreateWindowExW(0, button_string.as_ptr(),
        wide_char_string("Change selection").as_ptr(), BS_PUSHBUTTON | WS_CHILD | WS_VISIBLE |
        BS_VCENTER, 110, 0, 100, 20, clef_tab_handle, null_mut(), instance, null_mut());
    if select_clef_button_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create select clef button; error code {}", GetLastError());
    }
    SendMessageW(select_clef_button_handle, WM_SETFONT, text_font as usize, 0);
    let add_clef_button_handle = CreateWindowExW(0, button_string.as_ptr(),
        wide_char_string("Add clef").as_ptr(), BS_PUSHBUTTON | WS_DISABLED | WS_CHILD |
        WS_VISIBLE | BS_VCENTER, 110, 20, 100, 20, clef_tab_handle, null_mut(), instance,
        null_mut());
    if add_clef_button_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create add clef button; error code {}", GetLastError());
    }
    SendMessageW(add_clef_button_handle, WM_SETFONT, text_font as usize, 0);
    let key_sig_tab = TCITEMW{mask: TCIF_TEXT, dwState: 0, dwStateMask: 0,
        pszText: wide_char_string("Key Sigs").as_mut_ptr(), cchTextMax: 0, iImage: -1, lParam: 0};
    SendMessageW(control_tabs_handle, TCM_INSERTITEMW, KEY_SIG_TAB_INDEX as usize,
        &key_sig_tab as *const _ as isize);
    let key_sig_tab_handle = CreateWindowExW(0, static_string.as_ptr(), null_mut(), WS_CHILD, 0,
        tab_top, 500, 40, control_tabs_handle, null_mut(), instance, null_mut());
    if key_sig_tab_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create key sig tab; error code {}", GetLastError());
    }
    SetWindowSubclass(key_sig_tab_handle, Some(key_sig_tab_proc), 0, 0);
    let add_key_sig_button_handle = CreateWindowExW(0, button_string.as_ptr(),
        wide_char_string("Add key signature").as_ptr(), BS_PUSHBUTTON | WS_DISABLED | WS_CHILD |
        WS_VISIBLE | BS_VCENTER, 0, 0, 105, 20, key_sig_tab_handle, null_mut(), instance,
        null_mut());
    if add_key_sig_button_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create add key signature button; error code {}", GetLastError());
    }
    SendMessageW(add_key_sig_button_handle, WM_SETFONT, text_font as usize, 0);
    let note_tab = TCITEMW{mask: TCIF_TEXT, dwState: 0, dwStateMask: 0,
        pszText: wide_char_string("Notes").as_mut_ptr(), cchTextMax: 0, iImage: -1, lParam: 0};
    SendMessageW(control_tabs_handle, TCM_INSERTITEMW, NOTE_TAB_INDEX as usize,
        &note_tab as *const _ as isize);
    let note_tab_handle = CreateWindowExW(0, static_string.as_ptr(), null_mut(), WS_CHILD, 0,
        tab_top, 500, 40, control_tabs_handle, null_mut(), instance, null_mut());
    if note_tab_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create note tab; error code {}", GetLastError());
    }
    SetWindowSubclass(note_tab_handle, Some(note_tab_proc), 0, 0);
    let mut x = 0;
    let label_height = 20;
    let duration_label_handle = CreateWindowExW(0, static_string.as_ptr(),
        wide_char_string("Duration:").as_ptr(), SS_CENTER | WS_CHILD | WS_VISIBLE, 0, 0,
        110, label_height, note_tab_handle, null_mut(), instance, null_mut());
    if duration_label_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create duration label; error code {}", GetLastError());
    }
    SendMessageW(duration_label_handle, WM_SETFONT, text_font as usize, 0);
    let duration_display_handle = CreateWindowExW(0, static_string.as_ptr(),
        wide_char_string("quarter").as_ptr(), WS_BORDER | WS_CHILD | WS_VISIBLE, x, label_height,
        110, label_height, note_tab_handle, null_mut(), instance, null_mut());
    if duration_display_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create duration display; error code {}", GetLastError());
    }
    SendMessageW(duration_display_handle, WM_SETFONT, text_font as usize, 0);
    let duration_spin_handle = CreateWindowExW(0, wide_char_string(UPDOWN_CLASS).as_ptr(),
        null_mut(), UDS_ALIGNRIGHT | UDS_AUTOBUDDY | WS_CHILD | WS_VISIBLE, 0, 0, 0, 0,
        note_tab_handle, null_mut(), instance, null_mut());
    if duration_spin_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create duration spin; error code {}", GetLastError());
    }
    SendMessageW(duration_spin_handle, UDM_SETRANGE32, MIN_LOG2_DURATION as usize,
        MAX_LOG2_DURATION as isize);
    SendMessageW(duration_spin_handle, UDM_SETPOS32, 0, -2);
    x += 110;
    let augmentation_dot_label_handle = CreateWindowExW(0, static_string.as_ptr(),
        wide_char_string("Augmentation dots:").as_ptr(), SS_CENTER | WS_CHILD | WS_VISIBLE, x, 0,
        110, 20, note_tab_handle, null_mut(), instance, null_mut());
    if augmentation_dot_label_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create augmentation dot label; error code {}", GetLastError());
    }
    SendMessageW(augmentation_dot_label_handle, WM_SETFONT, text_font as usize, 0);
    let augmentation_dot_display_handle =  CreateWindowExW(0, static_string.as_ptr(),
        wide_char_string("0").as_ptr(), WS_BORDER | WS_VISIBLE | WS_CHILD, x, label_height, 110, 20,
        note_tab_handle, null_mut(), instance, null_mut());
    if augmentation_dot_display_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create augmentation dot display; error code {}", GetLastError());
    }
    SendMessageW(augmentation_dot_display_handle, WM_SETFONT, text_font as usize, 0);
    let augmentation_dot_spin_handle = CreateWindowExW(0, wide_char_string(UPDOWN_CLASS).as_ptr(),
        null_mut(), UDS_ALIGNRIGHT | UDS_AUTOBUDDY | UDS_SETBUDDYINT | WS_CHILD | WS_VISIBLE, 0, 0,
        0, 0, note_tab_handle, null_mut(), instance, null_mut());
    if augmentation_dot_spin_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create augmentation dot spin; error code {}", GetLastError());
    } 
    SendMessageW(augmentation_dot_spin_handle, UDM_SETRANGE32, 0,
        (-2 - MIN_LOG2_DURATION) as isize);
    let zoom_trackbar_handle = CreateWindowExW(0, wide_char_string(TRACKBAR_CLASS).as_ptr(),
        null_mut(), WS_CHILD | WS_VISIBLE, 0, 0, 0, 0, main_window_handle, null_mut(), instance,
        null_mut());
    if zoom_trackbar_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create zoom trackbar; error code {}", GetLastError());
    }
    SendMessageW(zoom_trackbar_handle, TBM_SETRANGEMIN, 0, 0);
    SendMessageW(zoom_trackbar_handle, TBM_SETRANGEMAX, 0, 2 * TRACKBAR_MIDDLE);
    SendMessageW(zoom_trackbar_handle, TBM_SETTIC, 0, TRACKBAR_MIDDLE);
    SendMessageW(zoom_trackbar_handle, TBM_SETPOS, 1, TRACKBAR_MIDDLE);
    let main_window_memory = Project{default_staff_space_height: 10.0,
        staff_scales: vec![StaffScale{name: unterminated_wide_char_string("Default"), value: 1.0},
        StaffScale{name: unterminated_wide_char_string("Cue"), value: 0.75}],
        slices: vec![Slice{objects: vec![], rhythmic_position: None,
        distance_from_previous_slice: 0}, Slice{objects: vec![],
        rhythmic_position: None, distance_from_previous_slice: 0}], staves: vec![],
        system_left_edge: 20, ghost_cursor: None, selection: Selection::None,
        selected_clef_octave_transposition: IDC_SELECT_CLEF_NONE,
        selected_clef_shape: IDC_SELECT_CLEF_G, control_tabs_handle: control_tabs_handle,
        staff_tab_handle: staff_tab_handle, add_staff_button_handle: add_staff_button_handle,
        clef_tab_handle: clef_tab_handle, select_clef_button_handle: select_clef_button_handle,
        add_clef_button_handle: add_clef_button_handle, key_sig_tab_handle: key_sig_tab_handle,
        add_key_sig_button_handle: add_key_sig_button_handle, note_tab_handle: note_tab_handle,
        duration_display_handle: duration_display_handle,
        duration_spin_handle: duration_spin_handle,
        augmentation_dot_spin_handle: augmentation_dot_spin_handle,
        zoom_trackbar_handle: zoom_trackbar_handle};        
    (main_window_handle, main_window_memory)
}

fn insert_clef(cursor_address: &Address, slices: &mut Vec<Slice>, staves: &mut Vec<Staff>,
    codepoint: u16, baseline_offset: i8) -> StaffObjectAddress
{
    let object = resolve_address_mut(&mut staves[cursor_address.staff_index],
        &cursor_address.object_address);
    if let ObjectType::Clef{..} = object.object_type
    {
        *object = Object{object_type: ObjectType::Clef{codepoint: codepoint,
            baseline_offset: baseline_offset}, is_selected: false};
        return cursor_address.object_address;
    }
    if let Some(previous_address) =
        previous_address(&staves[cursor_address.staff_index], &cursor_address.object_address)
    {
        let previous_object =
            resolve_address_mut(&mut staves[cursor_address.staff_index], &previous_address);
        if let ObjectType::Clef{..} = previous_object.object_type
        {
            *previous_object = Object{object_type: ObjectType::Clef{codepoint: codepoint,
                baseline_offset: baseline_offset}, is_selected: false};
            return previous_address;
        }
    }
    else
    {
        insert_object_range(slices, &mut staves[cursor_address.staff_index], &RangeAddress{
            staff_index: cursor_address.staff_index, range_index: 0}, 0);    
        slices[0].objects.push(
            RangeAddress{staff_index: cursor_address.staff_index, range_index: 0});    
        staves[cursor_address.staff_index].object_ranges[0].slice_object =
            Object{object_type: ObjectType::Clef{codepoint: codepoint,
            baseline_offset: baseline_offset}, is_selected: false};
        return StaffObjectAddress{range_index: 0, object_index: None};
    }
    let other_objects = &mut staves[cursor_address.staff_index].
        object_ranges[cursor_address.object_address.range_index].other_objects;
    let object_index =
    if let Some(index) = cursor_address.object_address.object_index
    {
        index
    }
    else
    {
        other_objects.len()
    };
    other_objects.insert(object_index, RangeObject{object: Object{object_type:
        ObjectType::Clef{codepoint: codepoint, baseline_offset: baseline_offset},
        is_selected: false}, distance_to_slice_object: 0});
    StaffObjectAddress{range_index: cursor_address.object_address.range_index,
        object_index: Some(object_index)}
}

fn insert_duration(device_context: HDC, slices: &mut Vec<Slice>, staves: &mut Vec<Staff>,
    staff_space_heights: &Vec<f32>, log2_duration: i8, pitch: Option<DisplayedPitch>,
    augmentation_dot_count: u8, cursor_address: &Address) -> StaffObjectAddress
{
    let mut range_index = cursor_address.object_address.range_index;
    if let Some(object_index) = cursor_address.object_address.object_index
    {
        staves[cursor_address.staff_index].object_ranges[range_index].
            other_objects.split_off(object_index);
    }    
    let mut rest_rhythmic_position;
    let mut slice_index;
    loop
    {       
        let staff = &mut staves[cursor_address.staff_index];
        slice_index = staff.object_ranges[range_index].slice_index;        
        if let Some(rhythmic_position) = &slices[slice_index].rhythmic_position
        {
            rest_rhythmic_position =
                rhythmic_position + whole_notes_long(log2_duration, augmentation_dot_count);
            staff.object_ranges[range_index].slice_object.object_type =
                ObjectType::Duration{log2_duration: log2_duration, pitch: pitch,
                augmentation_dot_count: augmentation_dot_count};            
            break;
        }
        range_index += 1;
    }
    reset_distance_from_previous_slice(device_context, slices, staves, staff_space_heights,
        slice_index);
    range_index += 1;
    let mut rest_duration;    
    loop 
    {
        if range_index == staves[cursor_address.staff_index].object_ranges.len()
        {
            register_rhythmic_position(slices, staves, &mut slice_index, rest_rhythmic_position,
                cursor_address.staff_index, range_index);
            reset_distance_from_previous_slice(device_context, slices, staves, staff_space_heights,
                slice_index);
            slice_index += 1;
            if slice_index < slices.len()
            {
                reset_distance_from_previous_slice(device_context, slices, staves,
                    staff_space_heights, slice_index);
            }
            return StaffObjectAddress{range_index: range_index, object_index: None};
        }
        let slice_index = staves[cursor_address.staff_index].object_ranges[range_index].slice_index;
        if let Some(rhythmic_position) = &slices[slice_index].rhythmic_position
        {
            if *rhythmic_position < rest_rhythmic_position
            {
                remove_object_range(staves, slices, cursor_address.staff_index, range_index,
                    slice_index);
            }
            else
            {
                rest_duration = rhythmic_position - &rest_rhythmic_position;
                break;
            }
        }
        else
        {
            remove_object_range(staves, slices, cursor_address.staff_index, range_index,
                slice_index);
        }
    }
    let mut denominator = rest_duration.denom().clone();
    let mut numerator = rest_duration.numer().clone();
    let mut division;
    let mut rest_log2_duration = 0;
    let zero = num_bigint::BigUint::new(vec![]);
    let two = num_bigint::BigUint::new(vec![2]);
    while denominator != zero
    {
        division = numerator.div_rem(&denominator);
        denominator /= &two;
        if division.0 != zero
        {
            let old_rest_rhythmic_position = rest_rhythmic_position;
            rest_rhythmic_position =
                &old_rest_rhythmic_position + whole_notes_long(rest_log2_duration, 0);
            register_rhythmic_position(slices, staves, &mut slice_index, old_rest_rhythmic_position,
                cursor_address.staff_index, range_index);
            staves[cursor_address.staff_index].object_ranges[range_index].slice_object.object_type =
                ObjectType::Duration{log2_duration: log2_duration, pitch: None,
                augmentation_dot_count: augmentation_dot_count};
            reset_distance_from_previous_slice(device_context, slices, staves, staff_space_heights,
                slice_index);
            numerator = division.1;            
            range_index += 1;
        }
        rest_log2_duration -= 1;
    }
    slice_index += 1;
    reset_distance_from_previous_slice(device_context, slices, staves, staff_space_heights,
        slice_index);
    slice_index += 1;
    if slice_index < slices.len()
    {
        reset_distance_from_previous_slice(device_context, slices, staves, staff_space_heights,
            slice_index);
    }
    StaffObjectAddress{range_index: range_index, object_index: None}
}

fn insert_object_range(slices: &mut Vec<Slice>, staff: &mut Staff, range_address: &RangeAddress,
    slice_index: usize)
{
    increment_range_indices(staff, slices, range_address, increment);
    staff.object_ranges.insert(range_address.range_index,
        ObjectRange{slice_index: slice_index, other_objects: vec![],
        slice_object: Object{object_type: ObjectType::None, is_selected: false}});
}

fn insert_slice(slices: &mut Vec<Slice>, staves: &mut Vec<Staff>, insertion_index: usize,
    new_slice: Slice)
{
    increment_slice_indices(slices, staves, insertion_index, increment);
    slices.insert(insertion_index, new_slice);
}

fn insert_staff_scale(staff_scales: &mut Vec<StaffScale>, scale_to_insert: StaffScale) -> usize
{
    let scale_count = staff_scales.len();
    for scale_index in 1..scale_count
    {
        if scale_to_insert.value > staff_scales[scale_index].value
        {
            staff_scales.insert(scale_index, scale_to_insert);
            return scale_index;
        }
    }
    staff_scales.push(scale_to_insert);
    scale_count
}

fn invalidate_work_region(window_handle: HWND)
{
    unsafe
    {
        let mut client_rect: RECT = std::mem::uninitialized();
        GetClientRect(window_handle, &mut client_rect);
        client_rect.top = 65;
        InvalidateRect(window_handle, &client_rect, TRUE);
    }
}

fn key_sig_address(cursor_address: &Address, slices: &mut Vec<Slice>,
    staves: &mut Vec<Staff>) -> StaffObjectAddress
{
    let object = resolve_address_mut(&mut staves[cursor_address.staff_index],
        &cursor_address.object_address);
    if let ObjectType::KeySignature{..} = &object.object_type
    {
        *object = Object{object_type: ObjectType::None, is_selected: false};
        return cursor_address.object_address;
    }
    if let Some(previous_address) =
        previous_address(&staves[cursor_address.staff_index], &cursor_address.object_address)
    {
        if staves[cursor_address.staff_index].object_ranges[previous_address.range_index].
            slice_index == 0
        {
            insert_object_range(slices, &mut staves[cursor_address.staff_index],
                &RangeAddress{staff_index: cursor_address.staff_index, range_index: 1}, 1);    
            slices[1].objects.push(RangeAddress{staff_index: cursor_address.staff_index,
                range_index: 1});    
            staves[cursor_address.staff_index].object_ranges[1].slice_object =
                Object{object_type: ObjectType::None, is_selected: false};
            return StaffObjectAddress{range_index: 1, object_index: None};
        }
        let previous_object =
            resolve_address_mut(&mut staves[cursor_address.staff_index], &previous_address);
        if let ObjectType::KeySignature{..} = &previous_object.object_type
        {
            *previous_object = Object{object_type: ObjectType::None, is_selected: false};
            return previous_address;
        }
    }
    else
    {
        insert_object_range(slices, &mut staves[cursor_address.staff_index], &RangeAddress{
            staff_index: cursor_address.staff_index, range_index: 0}, 1);    
        slices[1].objects.push(
            RangeAddress{staff_index: cursor_address.staff_index, range_index: 0});    
        staves[cursor_address.staff_index].object_ranges[0].slice_object =
            Object{object_type: ObjectType::None, is_selected: false};
        return StaffObjectAddress{range_index: 0, object_index: None};
    }
    let other_objects = &mut staves[cursor_address.staff_index].
        object_ranges[cursor_address.object_address.range_index].other_objects;
    let object_index = other_objects.len();
    other_objects.insert(object_index, RangeObject{object: Object{object_type:
        ObjectType::None, is_selected: false}, distance_to_slice_object: 0});
    StaffObjectAddress{range_index: cursor_address.object_address.range_index,
        object_index: Some(object_index)}
}

unsafe extern "system" fn key_sig_tab_proc(window_handle: HWND, u_msg: UINT, w_param: WPARAM,
    l_param: LPARAM, _id_subclass: UINT_PTR, _ref_data: DWORD_PTR) -> LRESULT
{
    match u_msg
    {
        WM_COMMAND =>
        {
            if HIWORD(w_param as u32) == BN_CLICKED
            {
                let main_window_handle = GetParent(GetParent(window_handle));
                SetFocus(main_window_handle);
                let project = project_memory(main_window_handle);
                if l_param == project.add_key_sig_button_handle as isize
                {
                    DialogBoxIndirectParamW(null_mut(), ADD_KEY_SIG_DIALOG_TEMPLATE.data.as_ptr()
                        as *const DLGTEMPLATE, main_window_handle, Some(add_key_sig_dialog_proc),
                        project as *mut _ as isize);
                    return 0;
                }
            }
        },
        _ => ()
    }
    DefWindowProcW(window_handle, u_msg, w_param, l_param)
}

fn left_edge_to_origin_distance(device_context: HDC, font: HFONT, staff_space_height: f32,
    pitch: &Option<DisplayedPitch>, log2_duration: i8) -> i32
{
    let mut distance = 0;
    if let Some(pitch) = pitch
    {
        if log2_duration == 1
        {
            distance += (staff_space_height *
                BRAVURA_METADATA.double_whole_notehead_x_offset).round() as i32;
        }
        distance += note_accidental_width(device_context, font, pitch);
    }
    distance
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
        let mut message: MSG =
            MSG{hwnd: null_mut(), message: 0, wParam: 0, lParam: 0, time: 0, pt: POINT{x: 0, y: 0}};        
        while GetMessageW(&mut message, main_window_handle, 0, 0) > 0
        {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

unsafe extern "system" fn main_window_proc(window_handle: HWND, u_msg: UINT, w_param: WPARAM,
    l_param: LPARAM) -> LRESULT
{
    match u_msg
    {          
        WM_HSCROLL =>
        {
            SetFocus(window_handle);
            invalidate_work_region(window_handle);
            return 0;       
        },
        WM_KEYDOWN =>
        {
            match w_param as i32
            {
                65..=71 =>
                {
                    let project = project_memory(window_handle);
                    if let Selection::ActiveCursor{address, range_floor} =
                        &mut (*project).selection
                    {
                        let scale_degree = (w_param as i8 - 60) % 7;
                        let mut octave4_cursor_range_floor = *range_floor % 7;
                        let mut octaves_of_range_floor_above_octave4 = *range_floor / 7;
                        if octave4_cursor_range_floor < 0
                        {
                            octave4_cursor_range_floor += 7;
                            octaves_of_range_floor_above_octave4 -= 1;
                        }
                        let mut steps_above_c4 =
                            7 * octaves_of_range_floor_above_octave4 + scale_degree;
                        if octave4_cursor_range_floor > scale_degree
                        {
                            steps_above_c4 += 7;
                        }
                        let pitch =
                            default_pitch_of_steps_above_c4(&project.staves[address.staff_index],
                            &address.object_address, steps_above_c4);
                        let space_heights = staff_space_heights(&project.staves,
                            &project.staff_scales, project.default_staff_space_height);
                        let device_context = GetDC(window_handle);
                        let next_duration_address = insert_duration(device_context,
                            &mut project.slices, &mut project.staves, &space_heights,
                            SendMessageW(project.duration_spin_handle, UDM_GETPOS32, 0, 0) as i8,
                            Some(pitch), SendMessageW(project.augmentation_dot_spin_handle,
                            UDM_GETPOS32, 0, 0) as u8, &address);
                        ReleaseDC(window_handle, device_context);
                        *address = Address{staff_index: address.staff_index,
                            object_address: next_duration_address};
                        *range_floor = steps_above_c4 - 3;
                        invalidate_work_region(window_handle);
                    }
                    return 0;
                },
                VK_BACK =>
                {
                    let project = project_memory(window_handle);
                    match &mut project.selection
                    {
                        Selection::ActiveCursor{address, range_floor} =>
                        {
                            if move_cursor_left(window_handle, &project.staves, address,
                                range_floor)
                            {
                                delete_object(window_handle, &mut project.slices,
                                    &mut project.staves, &project.staff_scales,
                                    project.default_staff_space_height, address);
                            }
                        },
                        Selection::None => (),
                        Selection::Object(address) =>
                        {
                            delete_object(window_handle, &mut project.slices, &mut project.staves,
                                &project.staff_scales, project.default_staff_space_height, address);
                        }
                    }
                    return 0;
                },
                VK_DELETE =>
                {
                    let project = project_memory(window_handle);
                    if let Selection::Object(address) = &mut project.selection
                    {
                        delete_object(window_handle, &mut project.slices, &mut project.staves,
                            &project.staff_scales, project.default_staff_space_height, address);
                        project.selection = Selection::None;
                    }
                    return 0;
                },
                VK_DOWN =>
                {
                    let project = project_memory(window_handle);
                    match &mut project.selection
                    {
                        Selection::ActiveCursor{range_floor,..} =>
                        {
                            decrement_range_floor(range_floor, 7);
                        },
                        Selection::Object(address) =>
                        {
                            let staff_line_count =
                                project.staves[address.staff_index].line_count as i8;
                            match resolve_address_mut(&mut project.staves[address.staff_index],
                                &address.object_address).object_type
                            {
                                ObjectType::Clef{ref mut baseline_offset,..} =>
                                {
                                    let new_baseline = *baseline_offset - 1;
                                    if new_baseline > -staff_line_count
                                    {
                                        *baseline_offset = new_baseline;
                                    }
                                },
                                ObjectType::Duration{log2_duration, ref mut pitch,
                                    augmentation_dot_count} =>
                                {
                                    if let Some(displayed_pitch) = pitch
                                    {
                                        let new_pitch =
                                        if HIBYTE(GetKeyState(VK_SHIFT) as u16) == 0        
                                        {                            
                                            if displayed_pitch.pitch.steps_above_c4 >
                                                i8::min_value()
                                            {
                                                displayed_pitch.pitch.steps_above_c4 -= 1;
                                            }
                                            let new_steps_above_c4 =
                                                displayed_pitch.pitch.steps_above_c4;
                                            default_pitch_of_steps_above_c4(
                                                &project.staves[address.staff_index],
                                                &address.object_address, new_steps_above_c4)
                                        }
                                        else
                                        {
                                            let new_accidental =
                                            match displayed_pitch.pitch.accidental
                                            {
                                                Accidental::DoubleSharp => Accidental::Sharp,
                                                Accidental::Sharp => Accidental::Natural,
                                                Accidental::Natural => Accidental::Flat,
                                                Accidental::Flat => Accidental::DoubleFlat,
                                                Accidental::DoubleFlat => return 0
                                            };
                                            let new_pitch =
                                                Pitch{accidental: new_accidental, steps_above_c4:
                                                displayed_pitch.pitch.steps_above_c4};
                                            let show_accidental =
                                            default_pitch_of_steps_above_c4(
                                                &project.staves[address.staff_index],
                                                &address.object_address, new_pitch.steps_above_c4).
                                                pitch.accidental != new_pitch.accidental;
                                            DisplayedPitch{pitch: new_pitch,
                                                show_accidental: show_accidental}
                                        };
                                        resolve_address_mut(
                                            &mut project.staves[address.staff_index],
                                            &address.object_address).object_type =
                                            ObjectType::Duration{log2_duration: log2_duration,
                                            pitch: Some(new_pitch),
                                            augmentation_dot_count: augmentation_dot_count};
                                        let space_heights = staff_space_heights(
                                            &project.staves, &project.staff_scales,
                                            project.default_staff_space_height);
                                        let device_context = GetDC(window_handle);
                                        reset_accidental_displays_from_previous_key_sig(
                                            device_context, &mut project.slices,
                                            &mut project.staves, &space_heights, address);
                                        ReleaseDC(window_handle, device_context);
                                    }
                                },
                                _ => ()
                            }
                        },
                        Selection::None => return 0
                    }
                    invalidate_work_region(window_handle);
                    return 0;
                },
                VK_ESCAPE =>
                {
                    cancel_selection(window_handle);
                    return 0;
                },
                VK_LEFT =>
                {
                    let project = project_memory(window_handle);
                    if let Selection::ActiveCursor{address, range_floor} = &mut project.selection
                    {
                        move_cursor_left(window_handle, &project.staves, address, range_floor);
                    }                   
                    return 0;
                },
                VK_RIGHT =>
                {
                    let project = project_memory(window_handle);
                    if let Selection::ActiveCursor{address, range_floor} = &mut project.selection
                    {
                        let staff = &project.staves[address.staff_index];
                        if let Some(next_address) = next_address(staff, &address.object_address)
                        {
                            if let ObjectType::Duration{pitch,..} =
                                &resolve_address(&staff, &next_address).object_type
                            {
                                if let Some(displayed_pitch) = pitch
                                {
                                    *range_floor = displayed_pitch.pitch.steps_above_c4;
                                    decrement_range_floor(range_floor, 3);
                                }
                            }
                            address.object_address = next_address;
                            invalidate_work_region(window_handle);
                        }
                    }
                    return 0;
                },
                VK_SPACE =>
                {
                    let project = project_memory(window_handle);
                    if let Selection::ActiveCursor{ref mut address,..} = project.selection
                    {
                        let space_heights = staff_space_heights(&project.staves,
                            &project.staff_scales, project.default_staff_space_height);
                        let device_context = GetDC(window_handle);
                        let next_duration_address = insert_duration(device_context,
                            &mut project.slices, &mut project.staves, &space_heights,
                            SendMessageW(project.duration_spin_handle, UDM_GETPOS32, 0, 0) as i8,
                            None, SendMessageW(project.augmentation_dot_spin_handle, UDM_GETPOS32,
                            0, 0) as u8, address);
                        ReleaseDC(window_handle, device_context);
                        *address = Address{staff_index: address.staff_index,
                            object_address: next_duration_address};
                        invalidate_work_region(window_handle);
                    }
                    return 0;
                },
                VK_UP =>
                {
                    let project = project_memory(window_handle);
                    match &mut project.selection
                    {
                        Selection::ActiveCursor{range_floor,..} =>
                        {
                            if *range_floor > i8::max_value() - 7
                            {
                                *range_floor = i8::max_value();
                            }
                            else
                            {
                                *range_floor += 7;
                            }
                        },
                        Selection::Object(address) =>
                        {
                            let staff_line_count =
                                project.staves[address.staff_index].line_count as i8;
                            match resolve_address_mut(&mut project.staves[address.staff_index],
                                &address.object_address).object_type
                            {
                                ObjectType::Clef{ref mut baseline_offset,..} =>
                                {
                                    let new_baseline = *baseline_offset + 1;
                                    if new_baseline < staff_line_count
                                    {
                                        *baseline_offset = new_baseline;
                                    }
                                },
                                ObjectType::Duration{log2_duration, ref mut pitch,
                                    augmentation_dot_count} =>
                                {
                                    if let Some(displayed_pitch) = pitch
                                    {
                                        let new_pitch =
                                        if HIBYTE(GetKeyState(VK_SHIFT) as u16) == 0        
                                        {                            
                                            if displayed_pitch.pitch.steps_above_c4 <
                                                i8::max_value()
                                            {
                                                displayed_pitch.pitch.steps_above_c4 += 1;
                                            }
                                            let new_steps_above_c4 =
                                                displayed_pitch.pitch.steps_above_c4;
                                            default_pitch_of_steps_above_c4(
                                                &project.staves[address.staff_index],
                                                &address.object_address, new_steps_above_c4)
                                        }
                                        else
                                        {
                                            let new_accidental =
                                            match displayed_pitch.pitch.accidental
                                            {
                                                Accidental::DoubleSharp => return 0,
                                                Accidental::Sharp => Accidental::DoubleSharp,
                                                Accidental::Natural => Accidental::Sharp,
                                                Accidental::Flat => Accidental::Natural,
                                                Accidental::DoubleFlat => Accidental::Flat
                                            };
                                            let new_pitch =
                                                Pitch{accidental: new_accidental, steps_above_c4:
                                                displayed_pitch.pitch.steps_above_c4};
                                            let show_accidental =
                                            default_pitch_of_steps_above_c4(
                                                &project.staves[address.staff_index],
                                                &address.object_address, new_pitch.steps_above_c4).
                                                pitch.accidental != new_pitch.accidental;
                                            DisplayedPitch{pitch: new_pitch,
                                                show_accidental: show_accidental}
                                        };
                                        resolve_address_mut(
                                            &mut project.staves[address.staff_index],
                                            &address.object_address).object_type =
                                            ObjectType::Duration{log2_duration: log2_duration,
                                            pitch: Some(new_pitch),
                                            augmentation_dot_count: augmentation_dot_count};
                                        let space_heights = staff_space_heights(
                                            &project.staves, &project.staff_scales,
                                            project.default_staff_space_height);
                                        let device_context = GetDC(window_handle);
                                        reset_accidental_displays_from_previous_key_sig(
                                            device_context, &mut project.slices,
                                            &mut project.staves, &space_heights, address);
                                        ReleaseDC(window_handle, device_context);
                                    }
                                },
                                _ => ()
                            }
                        },
                        Selection::None => return 0
                    }
                    invalidate_work_region(window_handle);
                    return 0;
                },
                _ => ()
            }            
        },
        WM_LBUTTONDOWN =>
        {
            let project = project_memory(window_handle);
            let zoom_factor = zoom_factor(project.zoom_trackbar_handle);
            let click_x = GET_X_LPARAM(l_param);
            let click_y = GET_Y_LPARAM(l_param);
            let device_context = GetDC(window_handle);
            let buffer_device_context = CreateCompatibleDC(device_context);
            ReleaseDC(window_handle, device_context);
            SaveDC(buffer_device_context);
            let mut client_rect: RECT = std::mem::uninitialized();
            GetClientRect(window_handle, &mut client_rect);
            let buffer = CreateCompatibleBitmap(device_context,
                client_rect.right - client_rect.left, client_rect.bottom - client_rect.top);
            SelectObject(buffer_device_context, buffer as *mut winapi::ctypes::c_void);
            SetBkMode(buffer_device_context, TRANSPARENT as i32);            
            SetTextAlign(buffer_device_context, TA_BASELINE);
            SetTextColor(buffer_device_context, WHITE);
            SelectObject(buffer_device_context, GetStockObject(WHITE_PEN as i32));
            SelectObject(buffer_device_context, GetStockObject(WHITE_BRUSH as i32));
            for staff_index in 0..project.staves.len()
            {
                let staff = &mut project.staves[staff_index];
                let space_height = staff_space_height(staff, &project.staff_scales,
                    project.default_staff_space_height);
                let address = address_of_clicked_staff_object(window_handle, buffer_device_context,
                    &project.slices, staff, space_height, project.system_left_edge,
                    click_x, click_y, zoom_factor);                            
                if let Some(address) = address
                {
                    project.selection = Selection::Object(Address{staff_index: staff_index,
                        object_address: address});
                    invalidate_work_region(window_handle);
                    RestoreDC(buffer_device_context, -1);
                    ReleaseDC(window_handle, buffer_device_context);
                    return 0;
                }
            }
            DeleteObject(buffer as *mut winapi::ctypes::c_void);
            match project.ghost_cursor
            {
                Some(_) =>
                {
                    cancel_selection(window_handle);
                    project.selection = Selection::ActiveCursor{address: std::mem::replace(
                        &mut project.ghost_cursor, None).unwrap(), range_floor: 3}; 
                    EnableWindow(project.add_clef_button_handle, TRUE);
                    EnableWindow(project.add_key_sig_button_handle, TRUE);
                    invalidate_work_region(window_handle);
                },
                _ => ()
            }
            RestoreDC(buffer_device_context, -1);
            ReleaseDC(window_handle, buffer_device_context);
            return 0;
        },
        WM_MOUSEMOVE =>
        {
            let project = project_memory(window_handle);
            let zoom_factor = zoom_factor(project.zoom_trackbar_handle);
            let mouse_x = GET_X_LPARAM(l_param);
            let mouse_y = GET_Y_LPARAM(l_param);                
            for staff_index in 0..project.staves.len()
            {
                let staff = &project.staves[staff_index];
                let vertical_bounds = staff_vertical_bounds(&staff, staff_space_height(&staff,
                    &project.staff_scales, project.default_staff_space_height), zoom_factor);
                if vertical_bounds.top <= mouse_y && mouse_y <= vertical_bounds.bottom
                {
                    let first_object_index =
                    if staff.object_ranges[0].other_objects.len() == 0
                    {
                        None
                    }
                    else
                    {
                        Some(0)
                    };
                    let mut current_address =
                        StaffObjectAddress{range_index: 0, object_index: first_object_index};
                    loop
                    {
                        let next_address = self::next_address(staff, &current_address);
                        if let Some(next_address) = next_address
                        {
                            if mouse_x < to_screen_coordinate(cursor_x(&project.slices, staff,
                                project.system_left_edge, &next_address) as f32, zoom_factor)
                            {
                                break;
                            }
                            current_address = next_address;
                        }
                        else
                        {
                            break;
                        }
                    }
                    if let Some(address) = &project.ghost_cursor
                    {
                        if address.object_address == current_address
                        {
                            return 0;
                        }
                    }     
                    project.ghost_cursor =
                        Some(Address{staff_index: staff_index, object_address: current_address});
                    invalidate_work_region(window_handle);               
                    return 0;
                }
            }
            match project.ghost_cursor
            {
                Some(_) =>
                {                     
                    invalidate_work_region(window_handle);
                    project.ghost_cursor = None;
                }
                None => ()
            }
            return 0;
        },
        WM_NOTIFY =>
        {
            let lpmhdr = l_param as LPNMHDR;
            match (*lpmhdr).code
            {
                TCN_SELCHANGE =>
                {
                    let project = project_memory(window_handle);
                    match SendMessageW(project.control_tabs_handle, TCM_GETCURSEL, 0, 0)
                    {
                        STAFF_TAB_INDEX =>
                        {
                            ShowWindow(project.staff_tab_handle, SW_SHOW);
                            SendMessageW(project.staff_tab_handle, WM_ENABLE, TRUE as usize, 0);
                        },
                        CLEF_TAB_INDEX =>
                        {
                            ShowWindow(project.clef_tab_handle, SW_SHOW);
                            SendMessageW(project.clef_tab_handle, WM_ENABLE, TRUE as usize, 0);
                        },
                        KEY_SIG_TAB_INDEX =>
                        {
                            ShowWindow(project.key_sig_tab_handle, SW_SHOW);
                            SendMessageW(project.key_sig_tab_handle, WM_ENABLE, TRUE as usize, 0);
                        },
                        NOTE_TAB_INDEX =>
                        {
                            ShowWindow(project.note_tab_handle, SW_SHOW);
                            SendMessageW(project.note_tab_handle, WM_ENABLE, TRUE as usize, 0);
                        },
                        _ => ()
                    }
                    return 0;
                },
                TCN_SELCHANGING =>
                {
                    let project = project_memory(window_handle);
                    match SendMessageW(project.control_tabs_handle, TCM_GETCURSEL, 0, 0)
                    {
                        STAFF_TAB_INDEX =>
                        {
                            ShowWindow(project.staff_tab_handle, SW_HIDE);
                            SendMessageW(project.staff_tab_handle, WM_ENABLE, FALSE as usize, 0);
                        },
                        CLEF_TAB_INDEX =>
                        {
                            ShowWindow(project.clef_tab_handle, SW_HIDE);
                            SendMessageW(project.clef_tab_handle, WM_ENABLE, FALSE as usize, 0);
                        },
                        KEY_SIG_TAB_INDEX =>
                        {
                            ShowWindow(project.key_sig_tab_handle, SW_HIDE);
                            SendMessageW(project.key_sig_tab_handle, WM_ENABLE, FALSE as usize, 0);
                        },
                        NOTE_TAB_INDEX =>
                        {
                            ShowWindow(project.note_tab_handle, SW_HIDE);
                            SendMessageW(project.note_tab_handle, WM_ENABLE, FALSE as usize, 0);
                        },
                        _ => ()
                    }
                    return 0;
                },                
                _ => ()
            }
        },
        WM_PAINT =>
        {
            let project = project_memory(window_handle);
            let zoom_factor = 10.0f32.powf(((SendMessageW(project.zoom_trackbar_handle, TBM_GETPOS,
                0, 0) - TRACKBAR_MIDDLE) as f32) / TRACKBAR_MIDDLE as f32);
            let mut paint_struct: PAINTSTRUCT = std::mem::uninitialized();
            let device_context = BeginPaint(window_handle, &mut paint_struct as *mut _);
            SaveDC(device_context);
            SetBkMode(device_context, TRANSPARENT as i32);
            SetTextAlign(device_context, TA_BASELINE);
            SelectObject(device_context, GetStockObject(BLACK_PEN as i32));
            SelectObject(device_context, GetStockObject(BLACK_BRUSH as i32)); 
            SetTextColor(device_context, BLACK);
            let mut client_rect: RECT = std::mem::uninitialized();
            GetClientRect(window_handle, &mut client_rect);
            for staff in &project.staves
            {
                let space_height = staff_space_height(staff, &project.staff_scales,
                    project.default_staff_space_height);
                let zoomed_font_set = staff_font_set(zoom_factor * space_height);
                for line_index in 0..staff.line_count
                {
                    draw_horizontal_line(device_context, project.system_left_edge as f32,                        
                        client_rect.right as f32, y_of_steps_above_bottom_line(staff, space_height,
                        2 * line_index as i8), space_height * BRAVURA_METADATA.staff_line_thickness,
                        zoom_factor);
                }
                let mut x = project.system_left_edge;
                let mut slice_index = 0;
                let mut staff_middle_pitch = 6;
                for index in 0..staff.object_ranges.len()
                {
                    let object_range = &staff.object_ranges[index];
                    while slice_index <= object_range.slice_index
                    {
                        x += project.slices[slice_index].distance_from_previous_slice;
                        slice_index += 1;
                    }
                    for range_object in &object_range.other_objects
                    {
                        draw_with_highlight(device_context, &zoomed_font_set, staff, space_height,
                            &range_object.object, object_range.slice_index,
                            x - range_object.distance_to_slice_object,
                            &mut staff_middle_pitch, zoom_factor);
                    }
                    draw_with_highlight(device_context, &zoomed_font_set, staff, space_height,
                        &object_range.slice_object, object_range.slice_index, x,
                        &mut staff_middle_pitch, zoom_factor);
                    release_font_set(&zoomed_font_set);
                }
            }            
            if let Some(address) = &project.ghost_cursor
            {
                SelectObject(device_context, GRAY_PEN.unwrap() as *mut winapi::ctypes::c_void);
                SelectObject(device_context, GRAY_BRUSH.unwrap() as *mut winapi::ctypes::c_void);
                let staff = &project.staves[address.staff_index];
                let cursor_x = cursor_x(&project.slices, staff, project.system_left_edge,
                    &address.object_address);
                let vertical_bounds = staff_vertical_bounds(staff, staff_space_height(staff,
                    &project.staff_scales, project.default_staff_space_height), zoom_factor);
                let left_edge = to_screen_coordinate(cursor_x as f32, zoom_factor);
                Rectangle(device_context, left_edge, vertical_bounds.top, left_edge + 1,
                    vertical_bounds.bottom);               
            }
            if let Selection::ActiveCursor{address, range_floor,..} = &project.selection
            {
                SelectObject(device_context, RED_PEN.unwrap() as *mut winapi::ctypes::c_void);
                SelectObject(device_context, RED_BRUSH.unwrap() as *mut winapi::ctypes::c_void);
                let staff = &project.staves[address.staff_index];
                let cursor_x = cursor_x(&project.slices, staff, project.system_left_edge,
                    &address.object_address);   
                let staff_space_height = staff_space_height(staff, &project.staff_scales,
                    project.default_staff_space_height);           
                let steps_of_floor_above_bottom_line =
                    range_floor - bottom_line_pitch(staff.line_count,
                    staff_middle_pitch_at_address(staff, &address.object_address));                    
                let range_indicator_bottom = y_of_steps_above_bottom_line(staff, staff_space_height,
                    steps_of_floor_above_bottom_line);
                let range_indicator_top = y_of_steps_above_bottom_line(staff, staff_space_height,
                    steps_of_floor_above_bottom_line + 6);
                let range_indicator_right_edge = cursor_x as f32 + staff_space_height;
                let line_thickness = staff_space_height * BRAVURA_METADATA.staff_line_thickness;
                draw_horizontal_line(device_context, cursor_x as f32, range_indicator_right_edge,
                    range_indicator_bottom, line_thickness, zoom_factor);
                draw_horizontal_line(device_context, cursor_x as f32, range_indicator_right_edge,
                    range_indicator_top, line_thickness, zoom_factor);
                let leger_left_edge = cursor_x as f32 - staff_space_height;
                let cursor_bottom =
                if steps_of_floor_above_bottom_line < 0
                {
                    for line_index in steps_of_floor_above_bottom_line / 2..0
                    {
                        draw_horizontal_line(device_context, leger_left_edge, cursor_x as f32,
                            y_of_steps_above_bottom_line(staff, staff_space_height, 2 * line_index),
                            line_thickness, zoom_factor);
                    }
                    range_indicator_bottom
                }
                else
                {
                    y_of_steps_above_bottom_line(staff, staff_space_height, 0)
                };
                let steps_of_ceiling_above_bottom_line = steps_of_floor_above_bottom_line + 6;
                let cursor_top =
                if steps_of_ceiling_above_bottom_line > 2 * (staff.line_count - 1) as i8
                {
                    for line_index in
                        staff.line_count as i8..=steps_of_ceiling_above_bottom_line / 2
                    {
                        draw_horizontal_line(device_context, leger_left_edge, cursor_x as f32,
                            y_of_steps_above_bottom_line(staff, staff_space_height, 2 * line_index),
                            line_thickness, zoom_factor);
                    }
                    range_indicator_top
                }
                else
                {
                    y_of_steps_above_bottom_line(staff, staff_space_height,
                        2 * (staff.line_count as i8 - 1))
                };
                let cursor_left_edge = to_screen_coordinate(cursor_x as f32, zoom_factor);
                Rectangle(device_context, cursor_left_edge,
                    to_screen_coordinate(cursor_top, zoom_factor), cursor_left_edge + 1,
                    to_screen_coordinate(cursor_bottom, zoom_factor));
            }
            RestoreDC(device_context, -1);
            EndPaint(window_handle, &mut paint_struct as *mut _);
        },
        WM_SIZE =>
        {
            let project = GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut Project;
            if project != null_mut()
            {
                let project = &mut *project;
                let mut client_rect = RECT{bottom: 0, left: 0, right: 0, top: 0};
                GetClientRect(window_handle, &mut client_rect);
                SetWindowPos(project.control_tabs_handle, null_mut(), client_rect.left, 0,
                    client_rect.right - client_rect.left, 65, 0);
                SetWindowPos(project.zoom_trackbar_handle, null_mut(),
                    (client_rect.right - client_rect.left) / 2 - 70,
                    client_rect.bottom - 20, 140, 20, 0);
            }
            return 0;
        }, 
        _ => ()
    }
    DefWindowProcW(window_handle, u_msg, w_param, l_param)
}

fn move_cursor_left(window_handle: HWND, staves: &Vec<Staff>, cursor_address: &mut Address,
    range_floor: &mut i8) -> bool
{
    let staff = &staves[cursor_address.staff_index];
    if let Some(previous_address) = previous_address(staff, &cursor_address.object_address)
    {
        if let ObjectType::Duration{pitch,..} =
            &resolve_address(staff, &previous_address).object_type
        {
            if let Some(displayed_pitch) = pitch
            {
                *range_floor = displayed_pitch.pitch.steps_above_c4;
                decrement_range_floor(range_floor, 3);
            }
        }
        cursor_address.object_address = previous_address;
        invalidate_work_region(window_handle);
        return true;
    }
    false
}

fn next_address(staff: &Staff, address: &StaffObjectAddress) -> Option<StaffObjectAddress>
{
    let mut range_index = address.range_index;
    let object_index =
    if let Some(index) = address.object_index
    {
        index + 1
    }
    else
    {
        range_index += 1; 
        if range_index == staff.object_ranges.len()
        {
            return None;
        }
        0
    };
    if object_index >= staff.object_ranges[range_index].other_objects.len()
    {
        return Some(StaffObjectAddress{range_index: range_index, object_index: None});
    }
    Some(StaffObjectAddress{range_index: range_index, object_index: Some(object_index)})
}

fn note_accidental_width(device_context: HDC, font: HFONT, displayed_pitch: &DisplayedPitch) -> i32
{
    if displayed_pitch.show_accidental
    {
        return character_width(device_context, font,
            accidental_codepoint(displayed_pitch.pitch.accidental) as u32)
    }
    0
}

unsafe extern "system" fn note_tab_proc(window_handle: HWND, u_msg: UINT, w_param: WPARAM,
    l_param: LPARAM, _id_subclass: UINT_PTR, _ref_data: DWORD_PTR) -> LRESULT
{
    match u_msg
    {
        WM_NOTIFY =>
        {
            let lpmhdr = l_param as LPNMHDR;
            if (*lpmhdr).code == UDN_DELTAPOS
            {
                let project = project_memory(GetParent(GetParent(window_handle)));
                let lpnmud = l_param as LPNMUPDOWN;
                let new_position = (*lpnmud).iPos + (*lpnmud).iDelta;
                if (*lpmhdr).hwndFrom == project.duration_spin_handle
                {
                    let new_text =                
                    if new_position > MAX_LOG2_DURATION
                    {
                        SendMessageW(project.augmentation_dot_spin_handle, UDM_SETRANGE32, 0, 11);                            
                        wide_char_string("double whole")
                    }
                    else if new_position < MIN_LOG2_DURATION
                    {
                        SendMessageW(project.augmentation_dot_spin_handle, UDM_SETRANGE32, 0, 0);
                        SendMessageW(project.augmentation_dot_spin_handle, UDM_SETPOS32, 0, 0);
                        wide_char_string("1024th")                        
                    }
                    else
                    {
                        let new_max_dot_count = (new_position - MIN_LOG2_DURATION) as isize;
                        if SendMessageW(project.augmentation_dot_spin_handle, UDM_GETPOS32, 0, 0) >
                            new_max_dot_count
                        {
                            SendMessageW(project.augmentation_dot_spin_handle, UDM_SETPOS32, 0,
                                new_max_dot_count);
                        }
                        SendMessageW(project.augmentation_dot_spin_handle, UDM_SETRANGE32, 0,
                            new_max_dot_count);
                        match new_position
                        {
                            1 => wide_char_string("double whole"),
                            0 => wide_char_string("whole"),
                            -1 => wide_char_string("half"),
                            -2 => wide_char_string("quarter"),
                            _ =>
                            {
                                let denominator = 2u32.pow(-new_position as u32);
                                if denominator % 10 == 2
                                {
                                    wide_char_string(&format!("{}nd", denominator))
                                }
                                else
                                {
                                    wide_char_string(&format!("{}th", denominator))
                                }
                            }
                        }
                    };
                    SendMessageW(project.duration_display_handle, WM_SETTEXT, 0,
                        new_text.as_ptr() as isize); 
                    return 0;               
                }
            }
        },
        _ => ()
    }
    DefWindowProcW(window_handle, u_msg, w_param, l_param)
}

fn object_width(device_context: HDC, font_set: &FontSet, staff_space_height: f32,
    object: &ObjectType, slice_index: usize) -> i32
{
    match object
    {
        ObjectType::Clef{codepoint,..} =>
        {
            let font =
            if slice_index == 0
            {
                font_set.full_size
            }
            else
            {
                font_set.two_thirds_size
            };
            character_width(device_context, font, *codepoint as u32)
        },
        ObjectType::Duration{log2_duration, pitch, augmentation_dot_count} =>
        {
            let mut width = *augmentation_dot_count as i32 *
                ((staff_space_height * DISTANCE_BETWEEN_AUGMENTATION_DOTS).round() as i32 +
                character_width(device_context, font_set.full_size, 0xe1e7)) +
                character_width(device_context, font_set.full_size,
                duration_codepoint(pitch, *log2_duration) as u32);
            if let Some(pitch) = pitch
            {
                width += note_accidental_width(device_context, font_set.full_size, pitch);
            }
            width
        },
        ObjectType::KeySignature{pattern, naturals, accidental_count,..} =>
        {
            let codepoint =
            if *naturals
            {
                0xe261
            }
            else
            {
                match *pattern
                {
                    AccidentalPattern::Flats => 0xe260,
                    AccidentalPattern::Sharps => 0xe262
                }
            };
            *accidental_count as i32 *
                character_width(device_context, font_set.full_size, codepoint as u32)
        }
        ObjectType::None => 0
    }
}

fn previous_address(staff: &Staff, address: &StaffObjectAddress) -> Option<StaffObjectAddress>
{
    let object_index =
    if let Some(index) = address.object_index
    {
        index
    }
    else
    {
        staff.object_ranges[address.range_index].other_objects.len()
    };
    if object_index == 0
    {
        if address.range_index == 0
        {
            return None;
        }
        return Some(StaffObjectAddress{range_index: address.range_index - 1, object_index: None})
    }
    Some(StaffObjectAddress{range_index: address.range_index, object_index: Some(object_index - 1)})
}

fn project_memory<'a>(main_window_handle: HWND) -> &'a mut Project
{
    unsafe
    {
        &mut *(GetWindowLongPtrW(main_window_handle, GWLP_USERDATA) as *mut Project)
    }
}

fn register_rhythmic_position(slices: &mut Vec<Slice>, staves: &mut Vec<Staff>,
    slice_index: &mut usize, rhythmic_position: num_rational::Ratio<num_bigint::BigUint>,
    staff_index: usize, range_index: usize)
{
    let position = rhythmic_position;
    loop
    {
        if *slice_index == slices.len()
        {
            insert_slice(slices, staves, *slice_index, Slice{objects: vec![], rhythmic_position:
                Some(position), distance_from_previous_slice: 0});
            break;
        }        
        if let Some(rhythmic_position) = &slices[*slice_index].rhythmic_position
        {
            if *rhythmic_position > position
            {
                insert_slice(slices, staves, *slice_index, Slice{objects: vec![],
                    rhythmic_position: Some(position), distance_from_previous_slice: 0});
                break;
            }
            if *rhythmic_position == position
            {
                break;
            }
        }
        *slice_index += 1;
    }
    let range_address = RangeAddress{staff_index: staff_index, range_index: range_index};
    insert_object_range(slices, &mut staves[staff_index], &range_address, *slice_index);
    slices[*slice_index].objects.push(range_address);
}

fn release_font_set(font_set: &FontSet)
{
    unsafe
    {
        DeleteObject(font_set.full_size as *mut winapi::ctypes::c_void);
        DeleteObject(font_set.two_thirds_size as *mut winapi::ctypes::c_void);
    }
}

unsafe extern "system" fn remap_staff_scale_dialog_proc(dialog_handle: HWND, u_msg: UINT,
    w_param: WPARAM, l_param: LPARAM) -> INT_PTR
{
    match u_msg
    {
        WM_COMMAND =>
        { 
            match LOWORD(w_param as u32) as i32
            {                
                IDCANCEL =>
                {
                    EndDialog(dialog_handle, -1);
                    TRUE as isize
                },
                IDOK =>
                {
                    EndDialog(dialog_handle, 0);
                    TRUE as isize
                },
                _ => FALSE as isize               
            }
        },
        WM_INITDIALOG =>
        {
            size_dialog(dialog_handle);
            let scale_list_handle = GetDlgItem(dialog_handle, IDC_REMAP_STAFF_SCALE_LIST);
            let staff_scales = &*(l_param as *const Vec<Vec<u16>>);
            for scale in staff_scales
            {
                SendMessageW(scale_list_handle, CB_ADDSTRING, 0, scale.as_ptr() as isize);
            }
            SendMessageW(scale_list_handle, CB_SETCURSEL, 0, 0);
            TRUE as isize
        },
        _ => FALSE as isize
    }
}

fn remove_durationless_object(slices: &mut Vec<Slice>, staves: &mut Vec<Staff>, staff_index: usize,
    object_address: &StaffObjectAddress)
{
    let range = &mut staves[staff_index].object_ranges[object_address.range_index];
    let slice_index = range.slice_index;
    if let Some(object_index) = object_address.object_index
    {
        range.other_objects.remove(object_index);
        return;
    }
    let mut range = remove_object_range(staves, slices, staff_index, object_address.range_index,
        slice_index);
    let next_range =
        &mut staves[staff_index].object_ranges[object_address.range_index].other_objects;
    range.other_objects.append(next_range);
    std::mem::swap(&mut range.other_objects, next_range);
}

fn remove_object_range(staves: &mut Vec<Staff>, slices: &mut Vec<Slice>, staff_index: usize,
    range_index: usize, slice_index: usize) -> ObjectRange
{
    let objects_in_slice_count = slices[slice_index].objects.len();
    if objects_in_slice_count == 1 && slice_index > 1
    {
        slices.remove(slice_index);
        increment_slice_indices(slices, staves, slice_index, decrement);                
    }
    else
    {
        for object_address_index in 0..objects_in_slice_count
        {
            if slices[slice_index].objects[object_address_index].staff_index == staff_index
            {
                slices[slice_index].objects.remove(object_address_index);
                break;
            }
        }
    }
    let range = staves[staff_index].object_ranges.remove(range_index);
    increment_range_indices(&staves[staff_index], slices,
        &RangeAddress{staff_index: staff_index, range_index: range_index}, decrement);
    range
}

fn reset_accidental_displays(device_context: HDC, slices: &mut Vec<Slice>, staves: &mut Vec<Staff>,
    staff_space_heights: &Vec<f32>, staff_index: usize, address: &mut Option<StaffObjectAddress>,
    key_sig_accidentals: &[Accidental; 7])
{
    let mut note_pitches = vec![vec![], vec![], vec![], vec![], vec![], vec![], vec![]];
    loop
    {
        if let Some(next_address) = address
        {
            match &mut resolve_address_mut(&mut staves[staff_index], next_address).object_type
            {
                ObjectType::Duration{pitch,..} =>
                {
                    if let Some(displayed_pitch) = pitch
                    {
                        let scale_degree = displayed_pitch.pitch.steps_above_c4 as usize % 7;
                        let scale_degree_pitches: &mut Vec<Pitch> = &mut note_pitches[scale_degree];
                        let show_accidental;
                        let mut pitch_index = scale_degree_pitches.len();
                        loop
                        {
                            if pitch_index == 0
                            {
                                show_accidental = key_sig_accidentals[scale_degree] !=
                                    displayed_pitch.pitch.accidental;
                                scale_degree_pitches.push(displayed_pitch.pitch);
                                break;
                            }
                            pitch_index -= 1;
                            let pitch = &mut scale_degree_pitches[pitch_index];
                            if pitch.steps_above_c4 == displayed_pitch.pitch.steps_above_c4
                            {
                                show_accidental =
                                    pitch.accidental != displayed_pitch.pitch.accidental;
                                *pitch = displayed_pitch.pitch;
                                break;
                            }
                            if scale_degree_pitches[pitch_index].accidental !=
                                displayed_pitch.pitch.accidental
                            {
                                show_accidental = true;
                                scale_degree_pitches.push(displayed_pitch.pitch);
                                break;
                            }
                        }
                        if show_accidental != displayed_pitch.show_accidental
                        {
                            displayed_pitch.show_accidental = show_accidental;
                            let slice_index = staves[staff_index].
                                object_ranges[next_address.range_index].slice_index;
                            reset_distance_from_previous_slice(device_context, slices, staves,
                                staff_space_heights, slice_index);
                        }
                    }
                },
                ObjectType::KeySignature{..} => 
                {
                    return;
                },
                _ => ()
            }
            *address = self::next_address(&staves[staff_index], next_address);
        }
        else
        {
            return;
        }
    }
}

fn reset_accidental_displays_from_previous_key_sig(device_context: HDC, slices: &mut Vec<Slice>,
    staves: &mut Vec<Staff>, staff_space_heights: &Vec<f32>, address: &Address)
{
    let key_sig_accidentals;
    let mut start_of_reset = address.object_address;
    let mut maybe_previous_address =
        previous_address(&staves[address.staff_index], &address.object_address);
    loop
    {
        if let Some(previous_address) = &maybe_previous_address
        {
            if let ObjectType::KeySignature{pattern, naturals, accidental_count,..} =
                &resolve_address(&staves[address.staff_index], previous_address).object_type
            {
                key_sig_accidentals = scale_degree_accidentals_from_key_sig(
                    *pattern, *naturals, *accidental_count);
                break;
            }
            start_of_reset = *previous_address;
        }
        else
        {
            key_sig_accidentals = [Accidental::Natural; 7];
            break;
        }
        maybe_previous_address = previous_address(&staves[address.staff_index], &start_of_reset);
    }
    reset_accidental_displays(device_context, slices, staves, staff_space_heights,
        address.staff_index, &mut Some(start_of_reset), &key_sig_accidentals);
}

fn reset_distance_from_previous_slice(device_context: HDC, slices: &mut Vec<Slice>,
    staves: &mut Vec<Staff>, staff_space_heights: &Vec<f32>, slice_index: usize)
{
    let mut distance_from_previous_slice = 0;
    for address in &slices[slice_index].objects
    {
        let staff = &mut staves[address.staff_index];
        let space_height = staff_space_heights[address.staff_index];
        let font_set = staff_font_set(space_height);
        let mut range_width = 0;
        let mut spacer = space_between_objects(
            &staff.object_ranges[address.range_index].slice_object.object_type);
        if let ObjectType::Duration{pitch, log2_duration,..} =
            &staff.object_ranges[address.range_index].slice_object.object_type
        {
            range_width += left_edge_to_origin_distance(device_context, font_set.full_size,
                space_height, pitch, *log2_duration);
        }
        for object_index in (0..staff.object_ranges[address.range_index].other_objects.len()).rev()
        {
            let range_object =
                &mut staff.object_ranges[address.range_index].other_objects[object_index];
            let object = &range_object.object.object_type;
            range_width +=
                object_width(device_context, &font_set, space_height, object, slice_index) +
                spacer(object, space_height, slice_index);
            spacer = space_between_objects(object);
            range_object.distance_to_slice_object = range_width;
        }
        if address.range_index > 0
        {
            let previous_range = &staff.object_ranges[address.range_index - 1];
            let previous_slice_object = &previous_range.slice_object.object_type;
            range_width += object_width(device_context, &font_set, space_height,
                previous_slice_object, previous_range.slice_index) +
                spacer(previous_slice_object, space_height, previous_range.slice_index);
            if let ObjectType::Duration{pitch, log2_duration, augmentation_dot_count} =
                previous_slice_object
            {
                    range_width -= left_edge_to_origin_distance(device_context, font_set.full_size,
                        space_height, pitch, *log2_duration);
                    range_width = std::cmp::max(range_width,
                        duration_width(space_height, *log2_duration, *augmentation_dot_count));
            }
            release_font_set(&font_set);
            for slice_index in previous_range.slice_index + 1..slice_index
            {
                range_width -= slices[slice_index].distance_from_previous_slice;
            }
        }
        else
        {
            range_width += space_height.round() as i32;
        }
        distance_from_previous_slice = std::cmp::max(distance_from_previous_slice, range_width);
    }
    slices[slice_index].distance_from_previous_slice = distance_from_previous_slice;
}

fn resolve_address<'a>(staff: &'a Staff, address: &StaffObjectAddress) -> &'a Object
{
    let range = &staff.object_ranges[address.range_index];
    if let Some(object_index) = address.object_index
    {
        &range.other_objects[object_index].object
    }
    else
    {
        &range.slice_object
    }
}

fn resolve_address_mut<'a>(staff: &'a mut Staff, address: &StaffObjectAddress) -> &'a mut Object
{
    let range = &mut staff.object_ranges[address.range_index];
    if let Some(object_index) = address.object_index
    {
        &mut range.other_objects[object_index].object
    }
    else
    {
        &mut range.slice_object
    }
}

fn respace(device_context: HDC, slices: &mut Vec<Slice>, staves: &mut Vec<Staff>,
    staff_space_heights: &Vec<f32>, staff_index: usize, mut range_index: usize)
{
    let slice_index = staves[staff_index].object_ranges[range_index].slice_index;
    reset_distance_from_previous_slice(device_context, slices, staves, staff_space_heights,
        slice_index);
    range_index += 1;
    if range_index < staves[staff_index].object_ranges.len()
    {
        let slice_index = staves[staff_index].object_ranges[range_index].slice_index;
        reset_distance_from_previous_slice(device_context, slices, staves, staff_space_heights,
            slice_index);
    }
}

fn scale_degree_accidentals_from_key_sig(pattern: AccidentalPattern, naturals: bool,
    accidental_count: u8) -> [Accidental; 7]
{
    let mut scale_degree_accidentals = [Accidental::Natural; 7];
    if naturals
    {
        return scale_degree_accidentals;
    }
    let accidental;
    let stride;
    let mut accidental_scale_degree;
    match pattern
    {
        AccidentalPattern::Flats =>
        {
            accidental = Accidental::Flat;
            stride = 3;
            accidental_scale_degree = 6;
        },
        AccidentalPattern::Sharps =>
        {
            accidental = Accidental::Sharp;
            stride = 4;
            accidental_scale_degree = 3;
        }
    }
    for _ in 0..accidental_count
    {
        scale_degree_accidentals[accidental_scale_degree] = accidental;
        accidental_scale_degree = (accidental_scale_degree + stride) % 7;
    }
    scale_degree_accidentals
}

unsafe extern "system" fn select_clef_dialog_proc(dialog_handle: HWND, u_msg: UINT, w_param: WPARAM,
    l_param: LPARAM) -> INT_PTR
{
    match u_msg
    {
        WM_COMMAND =>
        { 
            match LOWORD(w_param as u32) as i32
            {
                IDC_SELECT_CLEF_C =>
                {         
                    let fifteen_ma_handle = GetDlgItem(dialog_handle, IDC_SELECT_CLEF_15MA);
                    let eight_va_handle = GetDlgItem(dialog_handle, IDC_SELECT_CLEF_8VA);
                    let none_handle = GetDlgItem(dialog_handle, IDC_SELECT_CLEF_NONE);
                    let eight_vb_handle = GetDlgItem(dialog_handle, IDC_SELECT_CLEF_8VB);
                    let fifteen_mb_handle = GetDlgItem(dialog_handle, IDC_SELECT_CLEF_15MB);
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
                IDC_SELECT_CLEF_UNPITCHED =>
                {
                    EnableWindow(GetDlgItem(dialog_handle, IDC_SELECT_CLEF_15MA), FALSE);
                    EnableWindow(GetDlgItem(dialog_handle, IDC_SELECT_CLEF_8VA), FALSE);
                    EnableWindow(GetDlgItem(dialog_handle, IDC_SELECT_CLEF_NONE), FALSE);
                    EnableWindow(GetDlgItem(dialog_handle, IDC_SELECT_CLEF_8VB), FALSE);
                    EnableWindow(GetDlgItem(dialog_handle, IDC_SELECT_CLEF_15MB), FALSE);
                },
                IDCANCEL =>
                {
                    EndDialog(dialog_handle, 0);
                },
                IDOK =>
                {
                    let project =
                        &mut *(GetWindowLongPtrW(dialog_handle, DWLP_USER) as *mut Project);
                    for id in [IDC_SELECT_CLEF_C, IDC_SELECT_CLEF_F, IDC_SELECT_CLEF_G,
                        IDC_SELECT_CLEF_UNPITCHED].iter()
                    {
                        if SendMessageW(GetDlgItem(dialog_handle, *id), BM_GETCHECK, 0, 0) ==
                            BST_CHECKED as isize
                        {
                            project.selected_clef_shape = *id;
                            break;
                        }
                    }
                    for id in [IDC_SELECT_CLEF_15MA, IDC_SELECT_CLEF_8VA, IDC_SELECT_CLEF_NONE,
                        IDC_SELECT_CLEF_8VB, IDC_SELECT_CLEF_15MB].iter()
                    {
                        if SendMessageW(GetDlgItem(dialog_handle, *id), WM_COMMAND, 0, 0) ==
                            BST_CHECKED as isize
                        {
                            project.selected_clef_octave_transposition = *id;
                            break;
                        }
                    }
                    EndDialog(dialog_handle, 0);
                },
                _ =>
                {
                    EnableWindow(GetDlgItem(dialog_handle, IDC_SELECT_CLEF_15MA), TRUE);
                    EnableWindow(GetDlgItem(dialog_handle, IDC_SELECT_CLEF_8VA), TRUE);
                    EnableWindow(GetDlgItem(dialog_handle, IDC_SELECT_CLEF_NONE), TRUE);
                    EnableWindow(GetDlgItem(dialog_handle, IDC_SELECT_CLEF_8VB), TRUE);
                    EnableWindow(GetDlgItem(dialog_handle, IDC_SELECT_CLEF_15MB), TRUE);                    
                }                
            }
            TRUE as isize
        },
        WM_INITDIALOG =>
        {
            size_dialog(dialog_handle);
            SetWindowLongPtrW(dialog_handle, DWLP_USER, l_param);
            let project = &mut *(l_param as *mut Project);
            let shape_handle = GetDlgItem(dialog_handle, project.selected_clef_shape);
            SendMessageW(dialog_handle, WM_COMMAND, project.selected_clef_shape as usize,
                shape_handle as isize);
            SendMessageW(shape_handle, BM_SETCHECK, BST_CHECKED, 0);
            SendMessageW(GetDlgItem(dialog_handle, project.selected_clef_octave_transposition),
                BM_SETCHECK, BST_CHECKED, 0);
            TRUE as isize
        },
        _ => FALSE as isize
    }
}

fn size_dialog(dialog_handle: HWND)
{
    unsafe
    {
        let mut window_rect: RECT = std::mem::uninitialized();
        GetWindowRect(dialog_handle, &mut window_rect);
        AdjustWindowRect(&mut window_rect, GetWindowLongW(dialog_handle, GWL_STYLE) as u32, 0);
        MoveWindow(dialog_handle, window_rect.left, window_rect.top,
            window_rect.right - window_rect.left, window_rect.bottom - window_rect.top, TRUE);
    }
}

fn space_between_objects(right_object: &ObjectType) -> fn(&ObjectType, f32, usize) -> i32
{
    match right_object
    {
        ObjectType::Clef{..} =>
        {
            |_left_object: &ObjectType, staff_space_height: f32, _slice_index: usize|
            {
                staff_space_height.round() as i32
            }
        },
        ObjectType::Duration{pitch,..} =>
        {
            if let Some(pitch) = pitch
            {
                if pitch.show_accidental
                {
                    return |left_object: &ObjectType, staff_space_height: f32, slice_index: usize|
                        {
                            let multiplier =
                            match left_object
                            {
                                ObjectType::Clef{..} =>
                                {
                                    if slice_index == 0
                                    {
                                        1.5
                                    }
                                    else
                                    {
                                        1.0
                                    }
                                },
                                ObjectType::Duration{..} => 0.0,
                                ObjectType::KeySignature{..} => 1.5,
                                ObjectType::None => 0.0
                            };
                            (multiplier * staff_space_height).round() as i32
                        };
                }
            }
            |left_object: &ObjectType, staff_space_height: f32, slice_index: usize|
            {
                let multiplier =
                match left_object
                {
                    ObjectType::Clef{..} =>
                    {
                        if slice_index == 0
                        {
                            2.5
                        }
                        else
                        {
                            1.0
                        }
                    },
                    ObjectType::Duration{..} => 0.0,
                    ObjectType::KeySignature{..} =>
                    {
                        if slice_index == 1
                        {
                            2.5
                        }
                        else
                        {
                            2.0
                        }
                    },
                    ObjectType::None => 0.0
                };
                (multiplier * staff_space_height).round() as i32
            }
        },
        ObjectType::KeySignature{..} => 
        {
            |_left_object: &ObjectType, staff_space_height: f32, _slice_index: usize|
            {
                staff_space_height.round() as i32
            }
        },
        ObjectType::None => |_left_object: &ObjectType, _staff_space_height: f32,
            _slice_index: usize|{0}
    }
}

fn staff_font(staff_space_height: f32, staff_height_multiple: f32) -> HFONT
{
    unsafe
    {
        CreateFontW(-((4.0 * staff_height_multiple * staff_space_height).round() as i32),
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, wide_char_string("Bravura").as_ptr())
    }
}

fn staff_font_set(staff_space_height: f32) -> FontSet
{
    FontSet{full_size: staff_font(staff_space_height, 1.0),
        two_thirds_size: staff_font(staff_space_height, 2.0 / 3.0)}
}

fn staff_middle_pitch(clef_codepoint: u16, baseline_offset: i8) -> i8
{
    let baseline_pitch =
    match clef_codepoint
    {
        0xe050 => 4,
        0xe051 => -10,
        0xe052 => -3,
        0xe053 => 11,
        0xe054 => 18,
        0xe05c => 0,
        0xe05d => -7,
        0xe062 => -4,
        0xe063 => -18,
        0xe064 => -11,
        0xe065 => 3,
        0xe066 => 10,
        0xe069 => 4,
        _ => panic!("unknown clef codepoint")
    };
    baseline_pitch - baseline_offset
}

fn staff_middle_pitch_at_address(staff: &Staff, address: &StaffObjectAddress) -> i8
{
    let mut previous_address = previous_address(staff, address);
    loop
    {
        if let Some(address) = previous_address
        {
            let object_type =
            if let Some(object_index) = address.object_index
            {
                &staff.object_ranges[address.range_index].other_objects[object_index].object.
                    object_type
            }
            else
            {
                &staff.object_ranges[address.range_index].slice_object.object_type
            };
            if let ObjectType::Clef{codepoint, baseline_offset,..} = object_type
            {
                return staff_middle_pitch(*codepoint, *baseline_offset);
            }
            previous_address = self::previous_address(staff, &address);
        }
        else
        {
            return DEFAULT_STAFF_MIDDLE_PITCH;
        }
    }
}

fn staff_space_height(staff: &Staff, staff_scales: &Vec<StaffScale>,
    default_staff_space_height: f32) -> f32
{
    default_staff_space_height * staff_scales[staff.scale_index].value
}

fn staff_space_heights(staves: &Vec<Staff>, staff_scales: &Vec<StaffScale>,
    default_staff_space_height: f32) -> Vec<f32>
{
    let mut staff_space_heights = vec![];
    for staff in staves
    {
        staff_space_heights.push(
            staff_space_height(staff, staff_scales, default_staff_space_height));
    }
    staff_space_heights
}

unsafe extern "system" fn staff_tab_proc(window_handle: HWND, u_msg: UINT, w_param: WPARAM,
    l_param: LPARAM, _id_subclass: UINT_PTR, _ref_data: DWORD_PTR) -> LRESULT
{
    match u_msg
    {
        WM_COMMAND =>
        {
            if HIWORD(w_param as u32) == BN_CLICKED
            {
                let main_window_handle = GetParent(GetParent(window_handle));
                SetFocus(main_window_handle);
                let project = project_memory(main_window_handle);
                if l_param == project.add_staff_button_handle as isize
                {
                    if DialogBoxIndirectParamW(null_mut(), ADD_STAFF_DIALOG_TEMPLATE.data.as_ptr()
                        as *const DLGTEMPLATE, main_window_handle, Some(add_staff_dialog_proc),
                        project as *mut _ as isize) == 0
                    {
                        return 0;
                    }
                    let space_heights = staff_space_heights(&project.staves,
                        &project.staff_scales, project.default_staff_space_height);
                    let device_context = GetDC(main_window_handle);
                    reset_distance_from_previous_slice(device_context, &mut project.slices,
                        &mut project.staves, &space_heights, 2);     
                    ReleaseDC(main_window_handle, device_context);              
                    invalidate_work_region(main_window_handle);
                    return 0;
                }
            }
        },
        _ => ()
    }
    DefWindowProcW(window_handle, u_msg, w_param, l_param)
}

fn staff_vertical_bounds(staff: &Staff, space_height: f32, zoom_factor: f32) -> VerticalInterval
{
    let line_thickness = space_height * BRAVURA_METADATA.staff_line_thickness;
    VerticalInterval{top: horizontal_line_vertical_bounds(y_of_steps_above_bottom_line(
        staff, space_height, 2 * (staff.line_count as i8 - 1)),
        line_thickness, zoom_factor).top, bottom: horizontal_line_vertical_bounds(
        y_of_steps_above_bottom_line(staff, space_height, 0),
        line_thickness, zoom_factor).bottom}
}

fn to_screen_coordinate(logical_coordinate: f32, zoom_factor: f32) -> i32
{
    (zoom_factor * logical_coordinate).round() as i32
}

fn to_string(scale: &StaffScale) -> Vec<u16>
{
    let mut string = scale.name.clone();
    string.append(&mut unterminated_wide_char_string(": "));
    string.append(&mut unterminated_wide_char_string(&scale.value.to_string()));
    string.append(&mut wide_char_string(" X default"));
    string
}

fn whole_notes_long(log2_duration: i8, augmentation_dots: u8) ->
    num_rational::Ratio<num_bigint::BigUint>
{
    let mut whole_notes_long =
    if log2_duration >= 0
    {
        num_rational::Ratio::new(num_bigint::BigUint::from(2u32.pow(log2_duration as u32)),
            num_bigint::BigUint::new(vec![1]))
    }
    else
    {
        num_rational::Ratio::new(num_bigint::BigUint::new(vec![1]),
            num_bigint::BigUint::from(2u32.pow(-log2_duration as u32)))
    };
    let mut dot_whole_notes_long = whole_notes_long.clone();
    let two = num_bigint::BigUint::new(vec![2]);
    for _ in 0..augmentation_dots
    {
        dot_whole_notes_long /= &two;
        whole_notes_long += &dot_whole_notes_long;
    }
    whole_notes_long
}

fn y_of_steps_above_bottom_line(staff: &Staff, space_height: f32, step_count: i8) -> f32
{
    staff.vertical_center as f32 +
        (staff.line_count as f32 - 1.0 - step_count as f32) * space_height / 2.0
}

fn zoom_factor(zoom_trackbar_handle: HWND) -> f32
{
    unsafe
    {
        10.0f32.powf(((SendMessageW(zoom_trackbar_handle, TBM_GETPOS, 0, 0) -
            TRACKBAR_MIDDLE) as f32) / TRACKBAR_MIDDLE as f32)
    }
}