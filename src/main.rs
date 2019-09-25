#![allow(dead_code)]

extern crate num_bigint;
extern crate num_integer;
extern crate num_rational;
extern crate winapi;

mod shared;
mod safe_windows;

use safe_windows::*;
use shared::*;
use num_integer::Integer;
use winapi::shared::basetsd::*;
use winapi::shared::minwindef::*;
use winapi::shared::windef::*;
use winapi::shared::windowsx::*;
use winapi::um::commctrl::*;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

include!("constants.rs");

const CONTROL_TABS_HEIGHT: i32 = 65;
const DEFAULT_TOP_STAFF_MIDDLE_Y: i32 = 135;
const DISTANCE_BETWEEN_ACCIDENTAL_AND_NOTE: f32 = 0.12;
const DISTANCE_BETWEEN_AUGMENTATION_DOTS: f32 = 0.12;
const DWLP_USER: i32 = (std::mem::size_of::<LRESULT>() + std::mem::size_of::<DLGPROC>()) as i32;
const MAX_LOG2_DURATION: i32 = 1;
const MIN_LOG2_DURATION: i32 = -10;
const WHEEL_DELTA_SCALE: f32 = 8.0;

const HEADER_CLEF_SLICE_INDEX: usize = 1;
const HEADER_KEY_SIG_SLICE_INDEX: usize = 2;
const HEADER_TIME_SIG_SLICE_INDEX: usize = 3;

const STAFF_TAB_INDEX: isize = 0;
const CLEF_TAB_INDEX: isize = 1;
const KEY_SIG_TAB_INDEX: isize = 2;
const TIME_SIG_TAB_INDEX: isize = 3;
const NOTE_TAB_INDEX: isize = 4;

const LETTER_NAME_B: u8 = 6;
const LETTER_NAME_F: u8 = 3;

static mut GRAY_PEN: Option<HPEN> = None;
static mut GRAY_BRUSH: Option<HBRUSH> = None;
static mut RED_PEN: Option<HPEN> = None;
static mut RED_BRUSH: Option<HBRUSH> = None;

#[derive(Clone, Copy, PartialEq)]
enum Accidental
{
    DoubleFlat,
    Flat,
    Natural,
    Sharp,
    DoubleSharp
}

struct Clef
{
    codepoint: u16,
    steps_of_baseline_above_staff_middle: i8
}

struct DisplayedAccidental
{
    accidental: Accidental,
    is_visible: bool
}

struct Duration
{
    pitch: Option<NotePitch>,
    log2: i8,
    augmentation_dot_count: u8
}

struct FontSet
{
    full_size: HFONT,
    two_thirds_size: HFONT
}

struct KeySig
{
    accidentals: Vec<KeySigAccidental>,
    floors: [i8; 7]
}

struct KeySigAccidental
{
    accidental: Accidental,
    letter_name: u8
}

struct NotePitch
{
    accidental_address: Option<usize>,
    pitch: Pitch
}

struct Object
{
    object_type: ObjectType,
    address: usize,
    slice_address: Option<usize>,
    distance_to_next_slice: i32,
    is_selected: bool,
    is_valid_cursor_position: bool
}

enum ObjectType
{
    Accidental
    {
        note_address: usize
    },
    Barline,
    Clef(Clef),
    Duration(Duration),
    KeySig(KeySig),
    None,
    TimeSig
    {
        numerator: u16,
        denominator: u16
    }
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
    staves: Vec<Staff>,
    slices: Vec<Slice>,
    slice_indices: Vec<usize>,
    slice_address_free_list: Vec<usize>,
    viewport_offset: POINT,
    leftmost_visible_slice_address: usize,
    highest_visible_staff_index: usize,
    x_of_slice_beyond_leftmost_visible: i32,
    y_of_staff_above_highest_visible: i32,
    last_slice_x: i32,
    bottom_staff_y: i32,
    ghost_cursor: Option<SystemAddress>,
    selection: Selection,
    zoom_exponent: i8,
    main_window_back_buffer: HBITMAP,
    control_tabs_handle: HWND,
    staff_tab_handle: HWND,
    add_staff_button_handle: HWND,
    clef_tab_handle: HWND,
    c_clef_handle: HWND,
    f_clef_handle: HWND,
    g_clef_handle: HWND,
    clef_15ma_handle: HWND,
    clef_8va_handle: HWND,
    clef_none_handle: HWND,
    clef_8vb_handle: HWND,
    clef_15mb_handle: HWND,
    add_clef_button_handle: HWND,
    key_sig_tab_handle: HWND,
    accidental_count_spin_handle: HWND,
    sharps_handle: HWND,
    flats_handle: HWND,
    add_key_sig_button_handle: HWND,
    time_sig_tab_handle: HWND,
    numerator_spin_handle: HWND,
    denominator_display_handle: HWND,
    denominator_spin_handle: HWND,
    add_time_sig_button_handle: HWND,
    note_tab_handle: HWND,
    duration_display_handle: HWND,
    duration_spin_handle: HWND,
    augmentation_dot_spin_handle: HWND
}

enum Selection
{
    ActiveCursor
    {
        address: SystemAddress,
        range_floor: i8
    },
    Object(SystemAddress),
    None
}

struct Slice
{
    address: usize,
    object_addresses: Vec<SystemAddress>,
    whole_notes_long: Option<num_rational::Ratio<num_bigint::BigUint>>,
    distance_from_previous_slice: i32
}

struct Staff
{
    scale_index: usize,
    objects: Vec<Object>,
    object_indices: Vec<usize>,
    object_address_free_list: Vec<usize>,

    //Set to the header clef's address when the header clef is onscreen since there is no clef
    //beyond the leftmost visible slice in that case.
    address_of_clef_beyond_leftmost_visible_slice: usize,
    distance_from_staff_above: i32,//From vertical center to vertical center.
    line_count: u8
}

struct StaffScale
{
    name: Vec<u16>,
    value: f32
}

#[derive(PartialEq)]
struct SystemAddress
{
    staff_index: usize,
    object_address: usize
}

struct VerticalInterval
{
    top: i32,
    bottom: i32
}

fn accidental_codepoint(accidental: &Accidental) -> u16
{
    match accidental
    {
        Accidental::DoubleFlat => 0xe264,
        Accidental::Flat => 0xe260,
        Accidental::Natural => 0xe261,
        Accidental::Sharp => 0xe262,
        Accidental::DoubleSharp => 0xe263
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
                    DialogBoxIndirectParamW(std::ptr::null_mut(),
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
                        remapped_index = DialogBoxIndirectParamW(std::ptr::null_mut(),
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
                    let distance_from_staff_above = 
                    if project.staves.len() == 0
                    {
                        DEFAULT_TOP_STAFF_MIDDLE_Y
                    }
                    else
                    {
                        project.bottom_staff_y += 80;
                        80
                    };
                    let scale_index = SendMessageW(GetDlgItem(dialog_handle,
                        IDC_ADD_STAFF_SCALE_LIST), CB_GETCURSEL, 0, 0) as usize;
                    let clef = selected_clef(project);
                    let staff_index = project.staves.len();
                    project.staves.push(Staff{scale_index: scale_index, objects: vec![],
                        object_indices: vec![], object_address_free_list: vec![],
                        address_of_clef_beyond_leftmost_visible_slice: 1,
                        distance_from_staff_above: distance_from_staff_above,
                        line_count: SendMessageW(GetDlgItem(dialog_handle,
                            IDC_ADD_STAFF_LINE_COUNT_SPIN), UDM_GETPOS32, 0, 0) as u8});
                    let mut slice_addresses_to_respace = vec![];
                    insert_slice_object(project, &mut slice_addresses_to_respace, staff_index,
                        Object{object_type: ObjectType::None, address: 0, slice_address: Some(0),
                            distance_to_next_slice: 0, is_selected: false,
                            is_valid_cursor_position: false},
                        0, 0);
                    insert_slice_object(project, &mut slice_addresses_to_respace, staff_index,
                        Object{object_type: ObjectType::Clef(clef), address: 0,
                            distance_to_next_slice: 0, is_selected: false,
                            is_valid_cursor_position: false, slice_address: Some(1)},
                        1, HEADER_CLEF_SLICE_INDEX);
                    let time_sig = selected_time_sig(project);
                    insert_slice_object(project, &mut slice_addresses_to_respace, staff_index,
                        Object{object_type: time_sig, address: 0, slice_address: Some(3),
                            distance_to_next_slice: 0, is_selected: false,
                            is_valid_cursor_position: false},
                        2, HEADER_TIME_SIG_SLICE_INDEX);
                    insert_slice_object(project, &mut slice_addresses_to_respace, staff_index,
                        Object{object_type: ObjectType::None, address: 0, slice_address: Some(4),
                            distance_to_next_slice: 0, is_selected: false,
                            is_valid_cursor_position: true},
                        3, 4);
                    if let Some(key_sig) = new_key_sig(project.accidental_count_spin_handle,
                        project.flats_handle, &project.staves[staff_index], 0)
                    {
                        insert_slice_object(project, &mut slice_addresses_to_respace, staff_index,
                            Object{object_type: ObjectType::KeySig(key_sig), address: 0,
                                slice_address: Some(2), distance_to_next_slice: 0,
                                is_selected: false, is_valid_cursor_position: false},
                            2, HEADER_KEY_SIG_SLICE_INDEX);
                    }
                    let main_window_handle = GetWindow(dialog_handle, GW_OWNER);
                    respace_slices(main_window_handle, project, &slice_addresses_to_respace);
                    EndDialog(dialog_handle, 0);
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

fn address_of_clicked_staff_object(project: &Project, back_buffer_device_context: HDC,
    zoom_factor: f32, staff_index: usize, staff_middle_y: i32, mouse_x: i32,
    mouse_y: i32) -> Option<SystemAddress>
{
    let staff = &project.staves[staff_index];
    let space_height =
        project.default_staff_space_height * project.staff_scales[staff.scale_index].value;
    let zoomed_font_set = staff_font_set(zoom_factor * space_height);
    let mut staff_middle_pitch = staff_middle_pitch_at_viewport_left_edge(staff);
    let mut first_slice_index = project.slice_indices[project.leftmost_visible_slice_address];
    let mut slice_x = project.x_of_slice_beyond_leftmost_visible +
        project.slices[first_slice_index].distance_from_previous_slice - project.viewport_offset.x;
    let mut object_index = index_of_nearest_staff_slice_object_to_left_of_slice(project,
        staff_index, &mut first_slice_index, &mut slice_x);
    let object = &staff.objects[object_index];
    draw_object(back_buffer_device_context, &zoomed_font_set, zoom_factor, &project.staves,
        staff_index, staff_middle_y, &mut staff_middle_pitch, space_height,
        project.default_staff_space_height, slice_x - object.distance_to_next_slice, &object);
    if get_pixel(back_buffer_device_context, mouse_x, mouse_y) == WHITE
    {
        release_font_set(&zoomed_font_set);
        return Some(SystemAddress{staff_index: staff_index, object_address: object.address});
    }
    object_index += 1;
    for slice_index in first_slice_index + 1..project.slices.len()
    {
        let slice = &project.slices[slice_index];
        slice_x += slice.distance_from_previous_slice;
        for address in &slice.object_addresses
        {
            if address.staff_index == staff_index
            {
                while object_index <= staff.object_indices[address.object_address]
                {
                    let object = &staff.objects[object_index];
                    let object_x = slice_x - object.distance_to_next_slice;
                    if mouse_x < to_screen_coordinate(object_x as f32, zoom_factor)
                    {
                        release_font_set(&zoomed_font_set);
                        return None;
                    }
                    draw_object(back_buffer_device_context, &zoomed_font_set, zoom_factor,
                        &project.staves, staff_index, staff_middle_y, &mut staff_middle_pitch,
                        space_height, project.default_staff_space_height, object_x, &object);
                    if get_pixel(back_buffer_device_context, mouse_x, mouse_y) == WHITE
                    {
                        release_font_set(&zoomed_font_set);
                        return Some(SystemAddress{staff_index: staff_index,
                            object_address: object.address});
                    }
                    object_index += 1;
                }
                break;
            }
        }
        if mouse_x < to_screen_coordinate(slice_x as f32, zoom_factor)
        {
            release_font_set(&zoomed_font_set);
            return None;
        }
    }
    release_font_set(&zoomed_font_set);
    None
}

fn basic_remove_object(slice_addresses_to_respace: &mut Vec<usize>, staff: &mut Staff,
    object_index: usize)
{
    push_if_not_present(slice_addresses_to_respace, next_slice_address(staff, object_index));
    staff.object_address_free_list.push(staff.objects.remove(object_index).address);
    increment_object_indices(staff, object_index, decrement);
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
        Selection::ActiveCursor{..} => (),
        Selection::Object(address) =>
        {
            let staff = &mut project.staves[address.staff_index];
            staff.objects[staff.object_indices[address.object_address]].is_selected = false;
        },
        Selection::None => return
    }   
    invalidate_work_region(main_window_handle);
    enable_add_header_object_buttons(project, FALSE);
    project.selection = Selection::None;
}

fn character_width(device_context: HDC, font: HFONT, codepoint: u32) -> i32
{
    let old_font = select_object(device_context, font);
    let mut abc_array = [ABC{abcA: 0, abcB: 0, abcC: 0}];
    unsafe
    {
        GetCharABCWidthsW(device_context, codepoint, codepoint + 1, abc_array.as_mut_ptr());
    }
    select_object(device_context, old_font);
    abc_array[0].abcB as i32
}

fn clamped_add(augend: i8, addend: u8) -> i8
{
    if augend > i8::max_value() - addend as i8
    {
        return i8::max_value();
    }
    augend + addend as i8
}

fn clamped_subtract(minuend: i8, subtrahend: u8) -> i8
{
    if minuend < i8::min_value() + subtrahend as i8
    {
        return i8::min_value();
    }
    minuend - subtrahend as i8
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
                    let clef = selected_clef(project);
                    let mut object_addresses_to_respace = vec![];
                    match &project.selection
                    {
                        Selection::ActiveCursor{address, range_floor} =>
                        {
                            let staff = &mut project.staves[address.staff_index];
                            let object_index = staff.object_indices[address.object_address];
                            insert_object(&mut object_addresses_to_respace, staff, object_index,
                                Object{object_type: ObjectType::Clef(clef), address: 0,
                                    slice_address: None, distance_to_next_slice: 0,
                                    is_selected: false, is_valid_cursor_position: true});
                            let staff_index = address.staff_index;
                            let floor = *range_floor;
                            set_cursor_to_next_state(project, staff_index, object_index, floor);
                        },
                        Selection::Object(address) =>
                        {
                            let staff_index = address.staff_index;
                            let staff = &mut project.staves[staff_index];
                            let mut object_index = staff.object_indices[address.object_address];
                            loop
                            {
                                if let ObjectType::Clef(old_clef) =
                                    &mut staff.objects[object_index].object_type
                                {
                                    old_clef.codepoint = clef.codepoint;
                                    old_clef.steps_of_baseline_above_staff_middle =
                                        clef.steps_of_baseline_above_staff_middle;
                                    cancel_selection(main_window_handle);
                                    set_cursor_to_next_state(project, staff_index, object_index, 0);
                                    let next_slice_index = project.slice_indices[
                                        next_slice_address(&project.staves[staff_index],
                                            object_index + 1)];
                                    object_addresses_to_respace.push(next_slice_index);
                                    break;
                                }
                                object_index -= 1;
                            }
                        },
                        Selection::None => panic!("Attempted to insert clef without selection.")
                    }
                    respace_slices(main_window_handle, project, &object_addresses_to_respace);
                    return 0;
                }
                if l_param == project.c_clef_handle as isize
                {
                    enable_window(project.clef_15ma_handle, FALSE);
                    enable_window(project.clef_8va_handle, FALSE);
                    enable_window(project.clef_none_handle, TRUE);
                    enable_window(project.clef_8vb_handle, TRUE);
                    enable_window(project.clef_15mb_handle, FALSE);                    
                    if !button_is_checked(project.clef_none_handle) &&
                        !button_is_checked(project.clef_8vb_handle)
                    {
                        SendMessageW(project.clef_15ma_handle, BM_SETCHECK, BST_UNCHECKED, 0);
                        SendMessageW(project.clef_8va_handle, BM_SETCHECK, BST_UNCHECKED, 0);
                        SendMessageW(project.clef_none_handle, BM_SETCHECK, BST_CHECKED, 0);
                        SendMessageW(project.clef_8vb_handle, BM_SETCHECK, BST_UNCHECKED, 0);
                        SendMessageW(project.clef_15mb_handle, BM_SETCHECK, BST_UNCHECKED, 0);
                    }
                    return 0;
                }
                if l_param == project.clef_none_handle as isize
                {
                    enable_window(project.clef_15ma_handle, FALSE);
                    enable_window(project.clef_8va_handle, FALSE);
                    enable_window(project.clef_none_handle, FALSE);
                    enable_window(project.clef_8vb_handle, FALSE);
                    enable_window(project.clef_15mb_handle, FALSE);
                    return 0;
                }
                enable_window(project.clef_15ma_handle, TRUE);
                enable_window(project.clef_8va_handle, TRUE);
                enable_window(project.clef_none_handle, TRUE);
                enable_window(project.clef_8vb_handle, TRUE);
                enable_window(project.clef_15mb_handle, TRUE);
                return 0;
            }
        },
        _ => ()
    }
    DefWindowProcW(window_handle, u_msg, w_param, l_param)
}

fn cursor_x(project: &Project, staff: &Staff, mut object_index: usize) -> i32
{
    let mut x = project.x_of_slice_beyond_leftmost_visible - project.viewport_offset.x -
        staff.objects[object_index].distance_to_next_slice;
    loop
    {
        if let Some(slice_address) = staff.objects[object_index].slice_address
        {
            for slice_index in project.slice_indices[project.leftmost_visible_slice_address]..=
                project.slice_indices[slice_address]
            {
                x += project.slices[slice_index].distance_from_previous_slice;
            }
            return x;
        }
        object_index += 1;
    }
}

fn decrement(index: &mut usize)
{
    *index -= 1;
}

fn default_accidental_of_steps_above_c4(previous_objects: &[Object], steps_above_c4: i8) ->
    DisplayedAccidental
{
    let mut accidental = Accidental::Natural;
    let mut pitch_in_other_octaves = vec![];
    for object in previous_objects.iter().rev()
    {
        match &object.object_type
        {
            ObjectType::Duration(duration) =>
            {
                if let Some(pitch) = &duration.pitch
                {
                    if pitch.pitch.steps_above_c4 == steps_above_c4
                    {
                        accidental = pitch.pitch.accidental;
                        break;
                    }
                    else if pitch.pitch.steps_above_c4 % 7 == steps_above_c4 % 7
                    {
                        let mut pitch_index = 0;
                        loop
                        {
                            if pitch_index == pitch_in_other_octaves.len()
                            {
                                pitch_in_other_octaves.push(&pitch.pitch);
                                break;
                            }
                            let other_octave_pitch = &mut pitch_in_other_octaves[pitch_index];
                            if pitch.pitch.steps_above_c4 == other_octave_pitch.steps_above_c4
                            {
                                break;
                            }
                            pitch_index += 1;
                        }
                    }
                }
            },
            ObjectType::KeySig(key_sig) =>
            {
                accidental = Accidental::Natural;
                let letter_name = (steps_above_c4 % 7) as u8;
                for key_sig_accidental in &key_sig.accidentals
                {
                    if key_sig_accidental.letter_name == letter_name
                    {
                        accidental = key_sig_accidental.accidental;
                        break;
                    }
                }
                break;
            },
            _ => ()
        }
    }
    let mut is_visible = false;
    for pitch in pitch_in_other_octaves
    {
        if pitch.accidental != accidental
        {
            is_visible = true;
            break;
        }
    }
    DisplayedAccidental{accidental: accidental, is_visible: is_visible}
}

fn default_object_origin_to_slice_distance(staff_space_height: f32, object: &Object) -> i32
{
    if let ObjectType::Duration(duration) = &object.object_type
    {
        if let Some(_) = duration.pitch
        {
            if duration.log2 == 1
            {
                return (staff_space_height *
                    BRAVURA_METADATA.double_whole_notehead_x_offset).round() as i32;
            }
        }
    }
    0
}

fn delete_object(main_window_handle: HWND, project: &mut Project, staff_index: usize,
    mut object_index: usize)
{
    let staff = &mut project.staves[staff_index];
    let object = &mut staff.objects[object_index];
    let mut slice_addresses_to_respace = vec![];
    match &mut object.object_type
    {
        ObjectType::Accidental{..} => (),
        ObjectType::Barline{..} => (),
        ObjectType::Duration(duration) =>
        {
            if let Some(note_pitch) = &duration.pitch
            {
                slice_addresses_to_respace.push(object.slice_address.unwrap());
                if let Some(address) = note_pitch.accidental_address
                {
                    object.is_valid_cursor_position = true;
                    let object_to_remove_index = staff.object_indices[address];
                    remove_object(project, &mut slice_addresses_to_respace, staff_index,
                        object_to_remove_index);
                    object_index -= 1;
                }
                *object_as_maybe_pitch(&mut project.staves[staff_index], object_index) = None;
                reset_accidental_displays_from_previous_key_sig(project,
                    &mut slice_addresses_to_respace, staff_index, object_index);
            }
        },
        _ =>
        {
            remove_object(project, &mut slice_addresses_to_respace, staff_index, object_index);
        }
    }
    respace_slices(main_window_handle, project, &slice_addresses_to_respace);
    let staff = &project.staves[staff_index];
    next_valid_cursor_index(staff, &mut object_index);
    set_active_cursor(SystemAddress{staff_index: staff_index,
        object_address: staff.objects[object_index].address},
        range_floor_at_index(staff, object_index), project);
}

fn draw_character(device_context: HDC, zoomed_font: HFONT, codepoint: u16, x: f32, y: f32,
    zoom_factor: f32)
{
    let old_font = select_object(device_context, zoomed_font);
    unsafe
    {
        TextOutW(device_context, to_screen_coordinate(x, zoom_factor),
            to_screen_coordinate(y, zoom_factor), vec![codepoint].as_ptr(), 1);
    }
    select_object(device_context, old_font);
}

fn draw_horizontal_line(device_context: HDC, left_end: f32, right_end: f32, vertical_center: f32,
    thickness: f32, zoom_factor: f32)
{
    let vertical_bounds = horizontal_line_vertical_bounds(vertical_center, thickness, zoom_factor);
    draw_rectangle(device_context, to_screen_coordinate(left_end, zoom_factor), vertical_bounds.top,
        to_screen_coordinate(right_end, zoom_factor), vertical_bounds.bottom);
}

fn draw_object(device_context: HDC, zoomed_font_set: &FontSet, zoom_factor: f32,
    staves: &Vec<Staff>, staff_index: usize, staff_middle_y: i32, staff_middle_pitch: &mut i8,
    staff_space_height: f32, default_staff_space_height: f32, x: i32, object: &Object)
{
    let staff = &staves[staff_index]; 
    match &object.object_type
    {
        ObjectType::Accidental{note_address} =>
        {
            let note_pitch = note_pitch(staff, *note_address);
            draw_character(device_context, zoomed_font_set.full_size,
                accidental_codepoint(&note_pitch.accidental), x as f32,
                y_of_steps_above_bottom_line(staff_middle_y, staff_space_height, staff.line_count,
                    note_pitch.steps_above_c4 - *staff_middle_pitch + staff.line_count as i8 - 1),
                zoom_factor);
        },
        ObjectType::Barline =>
        {
            let vertical_bounds = staff_vertical_bounds(staff_middle_y, staff_space_height,
                staff.line_count, zoom_factor);
            draw_rectangle(device_context, to_screen_coordinate(x as f32, zoom_factor),
                vertical_bounds.top,
                to_screen_coordinate(x as f32 +
                    default_staff_space_height * BRAVURA_METADATA.thin_barline_thickness,
                    zoom_factor),
                vertical_bounds.bottom);
        },
        ObjectType::Clef(clef) =>
        {
            let mut font = zoomed_font_set.two_thirds_size;
            if let Some(slice_address) = object.slice_address
            {
                if slice_address == HEADER_CLEF_SLICE_INDEX
                {
                    font = zoomed_font_set.full_size;
                }
            }
            *staff_middle_pitch = self::staff_middle_pitch(clef);
            draw_character(device_context, font, clef.codepoint, x as f32,
                y_of_steps_above_bottom_line(staff_middle_y, staff_space_height, staff.line_count,
                    staff.line_count as i8 - 1 + clef.steps_of_baseline_above_staff_middle),
                zoom_factor);
        },
        ObjectType::Duration(duration) =>
        {
            let duration_codepoint = duration_codepoint(duration);
            let unzoomed_font = staff_font(staff_space_height, 1.0);
            let duration_right_edge;
            let duration_y;
            let augmentation_dot_y;
            if let Some(pitch) = &duration.pitch
            {        
                let steps_above_bottom_line = pitch.pitch.steps_above_c4 -
                    bottom_line_pitch(staff.line_count, *staff_middle_pitch);
                duration_y = y_of_steps_above_bottom_line(staff_middle_y, staff_space_height,
                    staff.line_count, steps_above_bottom_line);
                augmentation_dot_y =
                if steps_above_bottom_line % 2 == 0
                {
                    y_of_steps_above_bottom_line(staff_middle_y, staff_space_height,
                        staff.line_count, steps_above_bottom_line + 1)
                }
                else
                {
                    duration_y
                };
                if duration.log2 < 0
                {             
                    let stem_left_edge;
                    let stem_right_edge;
                    let mut stem_bottom;
                    let mut stem_top;
                    let space_count = staff.line_count as i8 - 1;
                    if space_count > steps_above_bottom_line
                    {
                        stem_top = y_of_steps_above_bottom_line(staff_middle_y, staff_space_height,
                            staff.line_count, std::cmp::max(steps_above_bottom_line + 7,
                            space_count));
                        if duration.log2 == -1
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
                            if duration.log2 == -3
                            {
                                draw_character(device_context, zoomed_font_set.full_size, 0xe240,
                                    stem_left_edge, stem_top, zoom_factor);
                            }
                            else if duration.log2 < -3
                            {
                                draw_character(device_context, zoomed_font_set.full_size, 0xe242,
                                    stem_left_edge, stem_top, zoom_factor);
                                let flag_spacing = staff_space_height *
                                    (BRAVURA_METADATA.beam_spacing +
                                    BRAVURA_METADATA.beam_thickness);
                                for _ in 0..-duration.log2 - 4
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
                        stem_bottom = y_of_steps_above_bottom_line(staff_middle_y,
                            staff_space_height, staff.line_count,
                            std::cmp::min(steps_above_bottom_line - 7, space_count));
                        if duration.log2 == -1
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
                            if duration.log2 == -3
                            {
                                draw_character(device_context, zoomed_font_set.full_size, 0xe241,
                                    stem_left_edge, stem_bottom, zoom_factor);
                            }
                            else if duration.log2 < -3
                            {
                                draw_character(device_context, zoomed_font_set.full_size, 0xe243,
                                    stem_left_edge, stem_bottom, zoom_factor);
                                let flag_spacing = staff_space_height * 
                                    (BRAVURA_METADATA.beam_spacing +
                                    BRAVURA_METADATA.beam_thickness);
                                for _ in 0..-duration.log2 - 4
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
                    draw_rectangle(device_context,
                        to_screen_coordinate(stem_left_edge, zoom_factor),
                        to_screen_coordinate(stem_top, zoom_factor),
                        to_screen_coordinate(stem_right_edge, zoom_factor),
                        to_screen_coordinate(stem_bottom, zoom_factor));
                }
                duration_right_edge =
                    x + character_width(device_context, unzoomed_font, duration_codepoint as u32);
                let leger_extension = staff_space_height * BRAVURA_METADATA.leger_line_extension;
                let leger_thickness = staff_space_height * BRAVURA_METADATA.leger_line_thickness;
                let leger_left_edge = x as f32 - leger_extension;
                let leger_right_edge = duration_right_edge as f32 + leger_extension;
                if steps_above_bottom_line < -1
                {
                    for line_index in steps_above_bottom_line / 2..0
                    {
                        draw_horizontal_line(device_context, leger_left_edge, leger_right_edge,
                            y_of_steps_above_bottom_line(staff_middle_y, staff_space_height,
                                staff.line_count, 2 * line_index as i8),
                            leger_thickness, zoom_factor);
                    }
                }
                else if steps_above_bottom_line >= 2 * staff.line_count as i8
                {
                    for line_index in staff.line_count as i8..=steps_above_bottom_line / 2
                    {
                        draw_horizontal_line(device_context, leger_left_edge, leger_right_edge,
                            y_of_steps_above_bottom_line(staff_middle_y, staff_space_height,
                                staff.line_count, 2 * line_index),
                            leger_thickness, zoom_factor);
                    }
                }
            }
            else
            {
                let spaces_above_bottom_line =
                if duration.log2 == 0
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
                duration_right_edge =
                    x + character_width(device_context, unzoomed_font, duration_codepoint as u32);          
                duration_y = y_of_steps_above_bottom_line(staff_middle_y, staff_space_height,
                    staff.line_count, 2 * spaces_above_bottom_line as i8);
                augmentation_dot_y = y_of_steps_above_bottom_line(staff_middle_y,
                    staff_space_height, staff.line_count, 2 * spaces_above_bottom_line as i8 + 1);
            }
            let dot_separation = staff_space_height * DISTANCE_BETWEEN_AUGMENTATION_DOTS;
            let mut next_dot_left_edge = duration_right_edge as f32 + dot_separation;
            let dot_offset =
                dot_separation + character_width(device_context, unzoomed_font, 0xe1e7) as f32;
            draw_character(device_context, zoomed_font_set.full_size, duration_codepoint, x as f32,
                duration_y, zoom_factor);        
            for _ in 0..duration.augmentation_dot_count
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
        ObjectType::KeySig(key_sig) =>
        {
            let middle_line_letter_name = *staff_middle_pitch % 7;
            let floor = key_sig.floors[middle_line_letter_name as usize];
            let mut accidental_x = x;
            for accidental in &key_sig.accidentals
            {
                let mut steps_above_middle_line =
                    accidental.letter_name as i8 - middle_line_letter_name;
                if steps_above_middle_line < floor
                {
                    steps_above_middle_line += 7;
                }
                else if steps_above_middle_line > floor + 7
                {
                    steps_above_middle_line -= 7;
                }
                let codepoint = accidental_codepoint(&accidental.accidental);
                draw_character(device_context, zoomed_font_set.full_size, codepoint,
                    accidental_x as f32,
                    y_of_steps_above_bottom_line(staff_middle_y, staff_space_height,
                        staff.line_count, steps_above_middle_line + staff.line_count as i8 - 1),
                    zoom_factor);
                accidental_x += character_width(device_context, zoomed_font_set.full_size,
                    accidental_codepoint(&accidental.accidental) as u32);
            }
        },
        ObjectType::TimeSig{numerator, denominator,..} =>
        {
            let numerator_string = time_sig_component_string(*numerator);
            let denominator_string = time_sig_component_string(*denominator);
            let numerator_width =
                string_width(device_context, zoomed_font_set.full_size, &numerator_string);
            let denominator_width =
                string_width(device_context, zoomed_font_set.full_size, &denominator_string);
            let mut numerator_x = x;
            let mut denominator_x = x;
            if numerator_width > denominator_width
            {
                denominator_x += (numerator_width - denominator_width) / 2;
            }
            else
            {
                numerator_x += (denominator_width - numerator_width) / 2;
            }
            let old_font = select_object(device_context, zoomed_font_set.full_size);
            draw_string(device_context, to_screen_coordinate(numerator_x as f32, zoom_factor),
                to_screen_coordinate(staff_middle_y as f32 - staff_space_height, zoom_factor),
                numerator_string);
            draw_string(device_context, to_screen_coordinate(denominator_x as f32, zoom_factor),
                to_screen_coordinate(staff_middle_y as f32 + staff_space_height, zoom_factor),
                denominator_string);
            select_object(device_context, old_font);
        },
        ObjectType::None => ()
    }
}

fn draw_object_with_highlight(device_context: HDC, zoomed_font_set: &FontSet, zoom_factor: f32,
    staves: &Vec<Staff>, staff_index: usize, staff_middle_y: i32, staff_middle_pitch: &mut i8,
    staff_space_height: f32, default_staff_space_height: f32, x: i32, object: &Object)
{
    if object.is_selected
    {
        set_text_color(device_context, RED);
        draw_object(device_context, zoomed_font_set, zoom_factor, staves, staff_index,
            staff_middle_y, staff_middle_pitch, staff_space_height, default_staff_space_height, x,
            object);
        set_text_color(device_context, BLACK);
    }
    else
    {
        draw_object(device_context, &zoomed_font_set, zoom_factor, staves, staff_index,
            staff_middle_y, staff_middle_pitch, staff_space_height, default_staff_space_height, x,
            object);
    }
}

fn draw_staff(device_context: HDC, project: &Project, staff_index: usize, staff_middle_y: i32,
    update_region_right_edge: i32)
{
    let staff = &project.staves[staff_index];
    let space_height =
        project.default_staff_space_height * project.staff_scales[staff.scale_index].value;
    let zoom_factor = zoom_factor(project.zoom_exponent);
    let staff_line_thickness = space_height * BRAVURA_METADATA.staff_line_thickness;
    let staff_left_edge = to_screen_coordinate(
        (project.slices[0].distance_from_previous_slice - project.viewport_offset.x) as f32,
        zoom_factor);
    for line_index in 0..staff.line_count
    {
        let staff_line_vertical_bounds =
            horizontal_line_vertical_bounds(y_of_steps_above_bottom_line(staff_middle_y,
                space_height, staff.line_count, 2 * line_index as i8),
            staff_line_thickness, zoom_factor);
        draw_rectangle(device_context, staff_left_edge, staff_line_vertical_bounds.top,
            update_region_right_edge, staff_line_vertical_bounds.bottom);
    }
    if let Some(address) = &project.ghost_cursor
    {
        if address.staff_index == staff_index
        {
            save_dc(device_context);
            unsafe
            {
                select_object(device_context, GRAY_PEN.unwrap());
                select_object(device_context, GRAY_BRUSH.unwrap());
            }
            let cursor_x = cursor_x(project, staff, staff.object_indices[address.object_address]);
            let vertical_bounds = staff_vertical_bounds(staff_middle_y,
               project.default_staff_space_height * project.staff_scales[staff.scale_index].value,
                staff.line_count, zoom_factor);
            let left_edge = to_screen_coordinate(cursor_x as f32, zoom_factor);
            draw_rectangle(device_context, left_edge, vertical_bounds.top, left_edge + 1,
                vertical_bounds.bottom);   
            restore_dc(device_context);
        }            
    }
    if let Selection::ActiveCursor{address, range_floor,..} = &project.selection
    {
        if address.staff_index == staff_index
        {
            save_dc(device_context);
            unsafe
            {
                select_object(device_context, RED_PEN.unwrap());
                select_object(device_context, RED_BRUSH.unwrap());
            }
            let object_index = staff.object_indices[address.object_address];
            let cursor_x = cursor_x(project, staff, object_index) as f32;
            let mut previous_object_index = object_index;
            let staff_middle_pitch;
            loop
            {
                previous_object_index -= 1;
                if let ObjectType::Clef(clef) = &staff.objects[previous_object_index].object_type
                {
                    staff_middle_pitch = self::staff_middle_pitch(clef);
                    break;
                }
            }
            let steps_of_floor_above_bottom_line =
                range_floor - bottom_line_pitch(staff.line_count, staff_middle_pitch);                    
            let range_indicator_bottom = y_of_steps_above_bottom_line(staff_middle_y, space_height,
               staff.line_count, steps_of_floor_above_bottom_line);
            let range_indicator_top = y_of_steps_above_bottom_line(staff_middle_y, space_height,
               staff.line_count, steps_of_floor_above_bottom_line + 6);
            let range_indicator_right_edge = cursor_x + space_height;
            let line_thickness = space_height * BRAVURA_METADATA.staff_line_thickness;
            draw_horizontal_line(device_context, cursor_x, range_indicator_right_edge,
               range_indicator_bottom, line_thickness, zoom_factor);
            draw_horizontal_line(device_context, cursor_x, range_indicator_right_edge,
               range_indicator_top, line_thickness, zoom_factor);
            let leger_left_edge = cursor_x - space_height;
            let cursor_bottom =
            if steps_of_floor_above_bottom_line < 0
            {
                for line_index in steps_of_floor_above_bottom_line / 2..0
                {
                    draw_horizontal_line(device_context, leger_left_edge, cursor_x,
                        y_of_steps_above_bottom_line(staff_middle_y, space_height, staff.line_count,
                            2 * line_index),
                        line_thickness, zoom_factor);
                }
                range_indicator_bottom
            }
            else
            {
                y_of_steps_above_bottom_line(staff_middle_y, space_height, staff.line_count, 0)
            };
            let steps_of_ceiling_above_bottom_line = steps_of_floor_above_bottom_line + 6;
            let cursor_top =
            if steps_of_ceiling_above_bottom_line > 2 * (staff.line_count - 1) as i8
            {
                for line_index in staff.line_count as i8..=steps_of_ceiling_above_bottom_line / 2
                {
                    draw_horizontal_line(device_context, leger_left_edge, cursor_x,
                        y_of_steps_above_bottom_line(staff_middle_y, space_height, staff.line_count,
                            2 * line_index),
                        line_thickness, zoom_factor);
                }
                range_indicator_top
            }
            else
            {
                y_of_steps_above_bottom_line(staff_middle_y, space_height, staff.line_count,
                    2 * (staff.line_count as i8 - 1))
            };
            let cursor_left_edge = to_screen_coordinate(cursor_x, zoom_factor);
            draw_rectangle(device_context, cursor_left_edge,
                to_screen_coordinate(cursor_top, zoom_factor), cursor_left_edge + 1,
                to_screen_coordinate(cursor_bottom, zoom_factor));
            restore_dc(device_context);
        }
    }
    let zoomed_font_set = staff_font_set(zoom_factor * space_height);
    let mut staff_middle_pitch = staff_middle_pitch_at_viewport_left_edge(staff);
    let mut first_slice_index = project.slice_indices[project.leftmost_visible_slice_address];
    let mut slice_x = project.x_of_slice_beyond_leftmost_visible +
        project.slices[first_slice_index].distance_from_previous_slice - project.viewport_offset.x;
    let mut object_index = index_of_nearest_staff_slice_object_to_left_of_slice(project,
        staff_index, &mut first_slice_index, &mut slice_x);
    let object = &staff.objects[object_index];
    draw_object_with_highlight(device_context, &zoomed_font_set, zoom_factor, &project.staves,
        staff_index, staff_middle_y, &mut staff_middle_pitch, space_height,
        project.default_staff_space_height, slice_x - object.distance_to_next_slice, &object);
    object_index += 1;
    for slice_index in first_slice_index + 1..project.slices.len()
    {
        let slice = &project.slices[slice_index];
        slice_x += slice.distance_from_previous_slice;
        for address in &slice.object_addresses
        {
            if address.staff_index == staff_index
            {
                while object_index <= staff.object_indices[address.object_address]
                {
                    let object = &staff.objects[object_index];
                    let object_x = slice_x - object.distance_to_next_slice;
                    if to_screen_coordinate(object_x as f32, zoom_factor) >=
                        update_region_right_edge
                    {
                        release_font_set(&zoomed_font_set);
                        return;
                    }
                    draw_object_with_highlight(device_context, &zoomed_font_set, zoom_factor,
                        &project.staves, staff_index, staff_middle_y, &mut staff_middle_pitch,
                        space_height, project.default_staff_space_height, object_x, &object);
                    object_index += 1;
                }
                break;
            }
        }
        if update_region_right_edge < to_screen_coordinate(slice_x as f32, zoom_factor)
        {
            release_font_set(&zoomed_font_set);
            return;
        }
    }
    release_font_set(&zoomed_font_set);
}

fn duration_codepoint(duration: &Duration) -> u16
{
    match duration.pitch
    {
        Some(_) =>
        {
            match duration.log2
            {
                1 => 0xe0a0,
                0 => 0xe0a2,
                -1 => 0xe0a3,
                _ => 0xe0a4
            }
        },
        None =>
        {
            (0xe4e3 - duration.log2 as i32) as u16
        }
    }
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
                                    std::ptr::null(), MB_OK);
                                return TRUE as isize;
                            }
                            let scale = edit_staff_scale_dialog_memory(dialog_handle);
                            scale.value = value;
                            let name_edit = GetDlgItem(dialog_handle, IDC_EDIT_STAFF_SCALE_NAME);
                            let name_length =
                                SendMessageW(name_edit, WM_GETTEXTLENGTH, 0, 0) as usize + 1;
                            let mut name: Vec<u16> = vec![0; name_length];
                            SendMessageW(name_edit, WM_GETTEXT, name_length,
                                name.as_ptr() as isize);
                            name.pop();
                            scale.name = name;
                            EndDialog(dialog_handle, 0);
                            return TRUE as isize;
                        }
                    }
                    MessageBoxW(dialog_handle, wide_char_string(
                        "The value must be a non-negative decimal number.").as_ptr(),
                        std::ptr::null(), MB_OK);
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

fn enable_add_header_object_buttons(project: &Project, enable: BOOL)
{
    enable_window(project.add_clef_button_handle, enable);
    enable_window(project.add_key_sig_button_handle, enable);
    enable_window(project.add_time_sig_button_handle, enable);
}

fn ghost_cursor_address(project: &Project, mouse_x: i32, mouse_y: i32) -> Option<SystemAddress>
{
    let zoom_factor = zoom_factor(project.zoom_exponent);
    let x_of_slice_beyond_leftmost_visible =
        project.x_of_slice_beyond_leftmost_visible - project.viewport_offset.x;
    let mut staff_middle_y = project.y_of_staff_above_highest_visible - project.viewport_offset.y;
    for staff_index in project.highest_visible_staff_index..project.staves.len()
    {       
        let staff = &project.staves[staff_index];
        staff_middle_y += staff.distance_from_staff_above;
        let vertical_bounds = staff_vertical_bounds(staff_middle_y,
            project.default_staff_space_height * project.staff_scales[staff.scale_index].value,
            staff.line_count, zoom_factor);
        if vertical_bounds.top > mouse_y
        {
            return None;
        }
        if mouse_y <= vertical_bounds.bottom
        {
            let mut cursor_index = 0;
            next_valid_cursor_index(staff, &mut cursor_index);
            let mut object_index = cursor_index;
            let mut slice_x = x_of_slice_beyond_leftmost_visible;
            for slice_index in
                project.slice_indices[project.leftmost_visible_slice_address]..project.slices.len()
            {
                let slice = &project.slices[slice_index];
                slice_x += slice.distance_from_previous_slice;
                for address in &slice.object_addresses
                {
                    if address.staff_index == staff_index
                    {
                        while object_index <= staff.object_indices[address.object_address]
                        {
                            let object = &staff.objects[object_index];
                            let object_x = slice_x - object.distance_to_next_slice;
                            if mouse_x < to_screen_coordinate(object_x as f32, zoom_factor)
                            {
                                return Some(SystemAddress{staff_index: staff_index,
                                    object_address: staff.objects[cursor_index].address});
                            }
                            if object.is_valid_cursor_position
                            {
                                cursor_index = object_index;
                            }
                            object_index += 1;
                        }
                        break;
                    }
                }
            }
            return Some(SystemAddress{staff_index: staff_index,
                object_address: staff.objects[cursor_index].address});
        }
    }
    None
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

fn increment_object_indices(staff: &mut Staff, starting_object_index: usize,
    increment_operation: fn(&mut usize))
{
    for object_index in starting_object_index..staff.objects.len()
    {
        increment_operation(&mut staff.object_indices[staff.objects[object_index].address]);
    }
}

fn increment_slice_indices(slices: &Vec<Slice>, slice_indices: &mut Vec<usize>,
    starting_slice_index: usize, increment_operation: fn(&mut usize))
{
    for slice_index in starting_slice_index..slices.len()
    {
        increment_operation(&mut slice_indices[slices[slice_index].address]);
    }
}

fn index_of_nearest_staff_slice_object_to_left_of_slice(project: &Project, staff_index: usize,
    slice_index: &mut usize, slice_x: &mut i32) -> usize
{
    loop
    {
        let slice = &project.slices[*slice_index];
        for object_address in &slice.object_addresses
        {
            if object_address.staff_index == staff_index
            {
                return project.staves[staff_index].object_indices[object_address.object_address];
            }
        }
        *slice_x -= slice.distance_from_previous_slice;
        *slice_index -= 1;
    }
}

fn insert_object(slice_addresses_to_respace: &mut Vec<usize>, staff: &mut Staff,
    object_index: usize, mut object: Object) -> usize
{
    increment_object_indices(staff, object_index, increment);
    let address =
        new_address(&mut staff.object_indices, &mut staff.object_address_free_list, object_index);
    object.address = address;
    staff.objects.insert(object_index, object);
    push_if_not_present(slice_addresses_to_respace, next_slice_address(staff, object_index));
    address
}

fn insert_rhythmic_slice_object(project: &mut Project, slice_addresses_to_respace: &mut Vec<usize>,
    staff_index: usize, object: ObjectType, object_index: usize, slice_index: &mut usize,
    mut whole_notes_after_slice: num_rational::Ratio<num_bigint::BigUint>) -> usize
{
    let slice_address;
    loop
    {
        if *slice_index == project.slices.len() - 1
        {
            project.slices[*slice_index].whole_notes_long = Some(whole_notes_after_slice);
            *slice_index += 1;
            slice_address = insert_slice(project, *slice_index,
                Some(num_rational::Ratio::from_integer(num_bigint::BigUint::new(vec![]))));
            break;
        }
        if let Some(whole_notes_long) = &mut project.slices[*slice_index].whole_notes_long
        {
            if whole_notes_after_slice < *whole_notes_long
            {
                let new_slice_whole_notes_long =
                    whole_notes_long.clone() - whole_notes_after_slice.clone();
                *whole_notes_long = whole_notes_after_slice;
                *slice_index += 1;             
                slice_address =
                    insert_slice(project, *slice_index, Some(new_slice_whole_notes_long));
                break;
            }
            if whole_notes_after_slice == *whole_notes_long
            {
                *slice_index += 1;
                slice_address = project.slices[*slice_index].address;
                break;
            }
            whole_notes_after_slice = whole_notes_after_slice - whole_notes_long.clone();
        }
        *slice_index += 1;
    }
    insert_slice_object(project, slice_addresses_to_respace, staff_index,
        Object{object_type: object, address: 0, slice_address: Some(slice_address),
            distance_to_next_slice: 0, is_selected: false, is_valid_cursor_position: true},
        object_index, *slice_index)
}

fn insert_slice(project: &mut Project, slice_index: usize,
    whole_notes_long: Option<num_rational::Ratio<num_bigint::BigUint>>) -> usize
{
    increment_slice_indices(&project.slices, &mut project.slice_indices, slice_index, increment);
    let new_slice_address =
        new_address(&mut project.slice_indices, &mut project.slice_address_free_list, slice_index);
    project.slices.insert(slice_index,
        Slice{address: new_slice_address, object_addresses: vec![],
            whole_notes_long: whole_notes_long, distance_from_previous_slice: 0});
    new_slice_address
}

fn insert_slice_object(project: &mut Project, slice_addresses_to_respace: &mut Vec<usize>,
    staff_index: usize, object: Object, object_index: usize, slice_index: usize) -> usize
{
    let next_slice_index = slice_index + 1;
    if next_slice_index < project.slices.len()
    {
        push_if_not_present(slice_addresses_to_respace, project.slices[next_slice_index].address);
    }
    let address = insert_object(slice_addresses_to_respace, &mut project.staves[staff_index],
        object_index, object);
    project.slices[slice_index].object_addresses.push(
        SystemAddress{staff_index: staff_index, object_address: address});
    address
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
        client_rect.top = CONTROL_TABS_HEIGHT + 4;
        InvalidateRect(window_handle, &client_rect, FALSE);
    }
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
                    let staff_index;
                    let key_sig_index;
                    let key_sig_accidentals;
                    let mut slice_addresses_to_respace = vec![];
                    match &project.selection
                    {
                        Selection::ActiveCursor{address,..} =>
                        {
                            staff_index = address.staff_index;
                            let staff = &mut project.staves[staff_index];
                            key_sig_index = staff.object_indices[address.object_address];
                            let new_key_sig = new_key_sig(project.accidental_count_spin_handle,
                                project.flats_handle, staff, key_sig_index);
                            if let Some(new_key_sig) = new_key_sig
                            {
                                key_sig_accidentals =
                                    letter_name_accidentals_from_key_sig(&new_key_sig);
                                insert_object(&mut slice_addresses_to_respace, staff, key_sig_index,
                                    Object{object_type: ObjectType::KeySig(new_key_sig), address: 0,
                                        slice_address: None, distance_to_next_slice: 0,
                                        is_selected: false, is_valid_cursor_position: true});
                            }
                            else
                            {
                                return 0;
                            }
                        },
                        Selection::Object(address) =>
                        {
                            staff_index = address.staff_index;
                            let staff = &mut project.staves[staff_index];
                            let new_key_sig = new_key_sig(project.accidental_count_spin_handle,
                                project.flats_handle, staff,
                                staff.object_indices[address.object_address]);
                            if let Some(new_key_sig) = new_key_sig
                            {
                                key_sig_accidentals =
                                    letter_name_accidentals_from_key_sig(&new_key_sig);
                                key_sig_index = 2;
                                let object = &mut staff.objects[key_sig_index];
                                if let Some(slice_address) = object.slice_address
                                {
                                    if slice_address == HEADER_KEY_SIG_SLICE_INDEX
                                    {
                                        object.object_type = ObjectType::KeySig(new_key_sig);
                                    }
                                    else
                                    {
                                        let key_sig_address = new_address(&mut staff.object_indices,
                                            &mut staff.object_address_free_list, 2);
                                        insert_slice_object(project,
                                            &mut slice_addresses_to_respace, staff_index,
                                            Object{object_type: ObjectType::KeySig(new_key_sig),
                                                address: key_sig_address,
                                                slice_address: Some(HEADER_KEY_SIG_SLICE_INDEX),
                                                distance_to_next_slice: 0, is_selected: false,
                                                is_valid_cursor_position: false},
                                            2, HEADER_KEY_SIG_SLICE_INDEX);
                                    }
                                }
                                else
                                {
                                    let key_sig_address = new_address(&mut staff.object_indices,
                                        &mut staff.object_address_free_list, 2);
                                    insert_slice_object(project, &mut slice_addresses_to_respace,
                                        staff_index,
                                        Object{object_type: ObjectType::KeySig(new_key_sig),
                                            address: key_sig_address,
                                            slice_address: Some(HEADER_KEY_SIG_SLICE_INDEX),
                                            distance_to_next_slice: 0, is_selected: false,
                                            is_valid_cursor_position: false},
                                        2, HEADER_KEY_SIG_SLICE_INDEX);
                                }
                                cancel_selection(main_window_handle);
                            }
                            else
                            {
                                return 0;
                            }
                        },
                        Selection::None => panic!("Key sig insertion attempted without selection.")
                    }
                    let mut next_key_sig_index = key_sig_index + 1;
                    if reset_accidental_displays(project, &mut slice_addresses_to_respace,
                        staff_index, &mut next_key_sig_index, &key_sig_accidentals)
                    {                        
                        let new_key_sig =
                            object_as_key_sig(&mut project.staves[staff_index], key_sig_index);
                        if let Accidental::Natural = &new_key_sig.accidentals[0].accidental
                        {
                            if object_as_key_sig(&mut project.staves[staff_index],
                                next_key_sig_index).accidentals[0].accidental == Accidental::Natural
                            {
                                if let Some(slice_address) = project.staves[staff_index].
                                    objects[next_key_sig_index].slice_address
                                {
                                    let slice_index = project.slice_indices[slice_address];
                                    if project.slices[slice_index].object_addresses.len() == 1
                                    {
                                        project.slices.remove(slice_index);
                                        increment_slice_indices(&project.slices,
                                            &mut project.slice_indices, slice_index, decrement);
                                    }
                                    else
                                    {
                                        for object_address_index in
                                            0..project.slices[slice_index].object_addresses.len()
                                        {
                                            if project.slices[slice_index].
                                                object_addresses[object_address_index].
                                                staff_index == staff_index
                                            {
                                                project.slices[slice_index].
                                                    object_addresses.remove(object_address_index);
                                                break;
                                            }
                                        }
                                    }
                                }
                                remove_object(project, &mut slice_addresses_to_respace,
                                    staff_index, next_key_sig_index);
                            }
                        }
                        else
                        {
                            let mut next_key_sig_accidentals = vec![];
                            for accidental in &new_key_sig.accidentals
                            {
                                next_key_sig_accidentals.push(KeySigAccidental{accidental:
                                    Accidental::Natural, letter_name: accidental.letter_name});
                            }
                            let next_key_sig = object_as_key_sig(&mut project.staves[staff_index],
                                next_key_sig_index);
                            if next_key_sig.accidentals[0].accidental == Accidental::Natural
                            {
                                next_key_sig.accidentals = next_key_sig_accidentals;
                            }
                        }
                    }
                    set_cursor_to_next_state(project, staff_index, key_sig_index, 0);
                    respace_slices(main_window_handle, project, &slice_addresses_to_respace);
                    return 0;
                }
            }
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
                let project = project_memory(GetParent(GetParent(window_handle)));
                EnableWindow(project.flats_handle, enable);
                EnableWindow(project.sharps_handle, enable);
                return 0;
            }
        },
        _ => ()
    }
    DefWindowProcW(window_handle, u_msg, w_param, l_param)
}

fn letter_name_accidentals_from_key_sig(key_sig: &KeySig) -> [Accidental; 7]
{
    let mut key_sig_accidentals = [Accidental::Natural; 7];
    for accidental in &key_sig.accidentals
    {
        key_sig_accidentals[accidental.letter_name as usize] = accidental.accidental;
    }
    key_sig_accidentals
}

fn main()
{
    let mut main_window_memory;
    let main_window_handle;
    let mut message: MSG;
    unsafe
    {        
        let gray = RGB(127, 127, 127);
        GRAY_PEN = Some(CreatePen(PS_SOLID as i32, 1, gray));
        GRAY_BRUSH = Some(CreateSolidBrush(gray));
        RED_PEN = Some(CreatePen(PS_SOLID as i32, 1, RED));
        RED_BRUSH = Some(CreateSolidBrush(RED));
        let button_string = wide_char_string("button");
        let static_string = wide_char_string("static");    
        let main_window_name = wide_char_string("main");
        let cursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
        let instance = winapi::um::libloaderapi::GetModuleHandleW(std::ptr::null());
        let common_controls =
            INITCOMMONCONTROLSEX{dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                dwICC: ICC_BAR_CLASSES | ICC_STANDARD_CLASSES | ICC_TAB_CLASSES | ICC_UPDOWN_CLASS};
        InitCommonControlsEx(&common_controls as *const _);
        RegisterClassW(&WNDCLASSW{style: CS_HREDRAW | CS_OWNDC,
            lpfnWndProc: Some(main_window_proc as unsafe extern "system" fn(HWND, UINT, WPARAM,
                LPARAM) -> LRESULT),
            cbClsExtra: 0, cbWndExtra: std::mem::size_of::<usize>() as i32, hInstance: instance,
            hIcon: std::ptr::null_mut(), hCursor: cursor,
            hbrBackground: (COLOR_WINDOW + 1) as HBRUSH, lpszMenuName: std::ptr::null(),
            lpszClassName: main_window_name.as_ptr()});
        main_window_handle = CreateWindowExW(0, main_window_name.as_ptr(),
            wide_char_string("Music Notation").as_ptr(), WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, std::ptr::null_mut(),
            std::ptr::null_mut(), instance, std::ptr::null_mut());
        let device_context = get_dc(main_window_handle);
        let mut client_rect: RECT = std::mem::uninitialized();
        GetClientRect(main_window_handle, &mut client_rect);
        let back_buffer = CreateCompatibleBitmap(device_context,
            client_rect.right - client_rect.left, client_rect.bottom - client_rect.top);
        release_dc(main_window_handle, device_context);
        let mut metrics: NONCLIENTMETRICSA = std::mem::uninitialized();
        metrics.cbSize = std::mem::size_of::<NONCLIENTMETRICSA>() as u32;
        SystemParametersInfoA(SPI_GETNONCLIENTMETRICS, metrics.cbSize,
            &mut metrics as *mut _ as *mut winapi::ctypes::c_void, 0);
        let text_font = CreateFontIndirectA(&metrics.lfMessageFont as *const _);
        let control_tabs_handle = CreateWindowExW(0, wide_char_string("SysTabControl32").as_ptr(),
            std::ptr::null(), WS_CHILD | WS_VISIBLE, 0, 0, 0, 0, main_window_handle,
            std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(control_tabs_handle, WM_SETFONT, text_font as usize, 0);
        let tab_top = 25;
        let tab_height = CONTROL_TABS_HEIGHT - tab_top;
        let mut staff_tab_label = wide_char_string("Staves");
        let staff_tab = TCITEMW{mask: TCIF_TEXT, dwState: 0, dwStateMask: 0,
            pszText: staff_tab_label.as_mut_ptr(), cchTextMax: 0, iImage: -1, lParam: 0};
        SendMessageW(control_tabs_handle, TCM_INSERTITEMW, STAFF_TAB_INDEX as usize,
            &staff_tab as *const _ as isize);
        let staff_tab_handle = CreateWindowExW(0, static_string.as_ptr(), std::ptr::null(),
            WS_CHILD | WS_VISIBLE, 0, tab_top, 500, tab_height, control_tabs_handle,
            std::ptr::null_mut(), instance, std::ptr::null_mut());
        SetWindowSubclass(staff_tab_handle, Some(staff_tab_proc), 0, 0);
        let add_staff_button_handle =
            CreateWindowExW(0, button_string.as_ptr(), wide_char_string("Add staff").as_ptr(),
                BS_PUSHBUTTON | BS_VCENTER | WS_CHILD | WS_VISIBLE, 10, 10, 55, 20,
                staff_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(add_staff_button_handle, WM_SETFONT, text_font as usize, 0);
        let mut clef_tab_label = wide_char_string("Clefs");
        let clef_tab = TCITEMW{mask: TCIF_TEXT, dwState: 0, dwStateMask: 0,
            pszText: clef_tab_label.as_mut_ptr(), cchTextMax: 0, iImage: -1, lParam: 0};
        SendMessageW(control_tabs_handle, TCM_INSERTITEMW, CLEF_TAB_INDEX as usize,
            &clef_tab as *const _ as isize);
        let clef_tab_handle = CreateWindowExW(0, static_string.as_ptr(), std::ptr::null(), WS_CHILD,
            0, tab_top, 500, tab_height, control_tabs_handle, std::ptr::null_mut(), instance,
            std::ptr::null_mut());
        SetWindowSubclass(clef_tab_handle, Some(clef_tab_proc), 0, 0);
        let clef_shape_label_handle = CreateWindowExW(0, static_string.as_ptr(),
            wide_char_string("Shape:").as_ptr(), SS_LEFT | WS_CHILD | WS_VISIBLE, 5, 0, 50, 20,
            clef_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(clef_shape_label_handle, WM_SETFONT, text_font as usize, 0);
        let c_clef_handle = CreateWindowExW(0, button_string.as_ptr(),
            wide_char_string("C").as_ptr(), BS_AUTORADIOBUTTON | WS_CHILD | WS_GROUP | WS_VISIBLE,
            60, 0, 35, 20, clef_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(c_clef_handle, WM_SETFONT, text_font as usize, 0);
        let f_clef_handle = CreateWindowExW(0, button_string.as_ptr(),
            wide_char_string("F").as_ptr(), BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 115, 0, 35,
            20, clef_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(f_clef_handle, WM_SETFONT, text_font as usize, 0);
        let g_clef_handle = CreateWindowExW(0, button_string.as_ptr(),
            wide_char_string("G").as_ptr(), BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 170, 0, 35,
            20, clef_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(g_clef_handle, WM_SETFONT, text_font as usize, 0);
        SendMessageW(g_clef_handle, BM_SETCHECK, BST_CHECKED, 0);
        let unpitched_clef_handle = CreateWindowExW(0, button_string.as_ptr(),
            wide_char_string("Unpitched").as_ptr(), BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 225,
            0, 75, 20, clef_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(unpitched_clef_handle, WM_SETFONT, text_font as usize, 0);
        let clef_octave_label_handle = CreateWindowExW(0, static_string.as_ptr(),
            wide_char_string("Octave:").as_ptr(), SS_LEFT | WS_CHILD | WS_VISIBLE, 5, 20, 50, 20,
            clef_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(clef_octave_label_handle, WM_SETFONT, text_font as usize, 0);
        let clef_15ma_handle =
            CreateWindowExW(0, button_string.as_ptr(), wide_char_string("15ma").as_ptr(),
                BS_AUTORADIOBUTTON | WS_CHILD | WS_GROUP | WS_VISIBLE, 60, 20, 50, 20,
                clef_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(clef_15ma_handle, WM_SETFONT, text_font as usize, 0);
        let clef_8va_handle = CreateWindowExW(0, button_string.as_ptr(),
            wide_char_string("8va").as_ptr(), BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 115, 20,
            50, 20, clef_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(clef_8va_handle, WM_SETFONT, text_font as usize, 0);
        let clef_none_handle = CreateWindowExW(0, button_string.as_ptr(),
            wide_char_string("None").as_ptr(), BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 170, 20,
            50, 20, clef_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(clef_none_handle, WM_SETFONT, text_font as usize, 0);
        SendMessageW(clef_none_handle, BM_SETCHECK, BST_CHECKED, 0);
        let clef_8vb_handle = CreateWindowExW(0, button_string.as_ptr(),
            wide_char_string("8vb").as_ptr(), BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 225, 20,
            50, 20, clef_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(clef_8vb_handle, WM_SETFONT, text_font as usize, 0);
        let clef_15mb_handle = CreateWindowExW(0, button_string.as_ptr(),
            wide_char_string("15ma").as_ptr(), BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 280, 20,
            50, 20, clef_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(clef_15mb_handle, WM_SETFONT, text_font as usize, 0);
        let add_clef_button_handle =
            CreateWindowExW(0, button_string.as_ptr(), wide_char_string("Add clef").as_ptr(),
                BS_PUSHBUTTON | WS_DISABLED | WS_CHILD | WS_VISIBLE | BS_VCENTER, 335, 10, 55, 20,
                clef_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(add_clef_button_handle, WM_SETFONT, text_font as usize, 0);
        let mut key_sig_tab_label = wide_char_string("Key Sigs");
        let key_sig_tab = TCITEMW{mask: TCIF_TEXT, dwState: 0, dwStateMask: 0,
            pszText: key_sig_tab_label.as_mut_ptr(), cchTextMax: 0, iImage: -1, lParam: 0};
        SendMessageW(control_tabs_handle, TCM_INSERTITEMW, KEY_SIG_TAB_INDEX as usize,
            &key_sig_tab as *const _ as isize);
        let key_sig_tab_handle = CreateWindowExW(0, static_string.as_ptr(), std::ptr::null(),
            WS_CHILD, 0, tab_top, 500, tab_height, control_tabs_handle, std::ptr::null_mut(),
            instance, std::ptr::null_mut());
        SetWindowSubclass(key_sig_tab_handle, Some(key_sig_tab_proc), 0, 0);
        let accidental_count_label_handle = CreateWindowExW(0, static_string.as_ptr(),
            wide_char_string("Accidental count:").as_ptr(), SS_LEFT | WS_CHILD | WS_VISIBLE, 5, 10,
            95, 20, key_sig_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(accidental_count_label_handle, WM_SETFONT, text_font as usize, 0);
        let accidental_count_display_handle = CreateWindowExW(0, static_string.as_ptr(),
            std::ptr::null(), WS_BORDER | WS_CHILD | WS_VISIBLE, 105, 10, 30, 20,
            key_sig_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(accidental_count_display_handle, WM_SETFONT, text_font as usize, 0);
        let accidental_count_spin_handle = CreateWindowExW(0,
            wide_char_string(UPDOWN_CLASS).as_ptr(), std::ptr::null(),
            UDS_ALIGNRIGHT | UDS_AUTOBUDDY | UDS_SETBUDDYINT | WS_CHILD | WS_VISIBLE, 0, 0, 0, 0,
            key_sig_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(accidental_count_spin_handle, UDM_SETRANGE32, 0, 7);
        let sharps_handle =
            CreateWindowExW(0, button_string.as_ptr(), wide_char_string("Sharps").as_ptr(),
                BS_AUTORADIOBUTTON | WS_CHILD | WS_DISABLED | WS_GROUP | WS_VISIBLE, 150, 0, 55, 20,
                key_sig_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(sharps_handle, BM_SETCHECK, BST_CHECKED, 0);
        SendMessageW(sharps_handle, WM_SETFONT, text_font as usize, 0);
        let flats_handle =
            CreateWindowExW(0, button_string.as_ptr(), wide_char_string("Flats").as_ptr(),
                BS_AUTORADIOBUTTON | WS_CHILD | WS_DISABLED | WS_VISIBLE, 150, 20, 55, 20,
                key_sig_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(flats_handle, WM_SETFONT, text_font as usize, 0);
        let add_key_sig_button_handle = CreateWindowExW(0, button_string.as_ptr(),
            wide_char_string("Add key signature").as_ptr(),
            BS_PUSHBUTTON | BS_VCENTER | WS_DISABLED | WS_CHILD | WS_VISIBLE, 215, 10, 105, 20,
            key_sig_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(add_key_sig_button_handle, WM_SETFONT, text_font as usize, 0);
        let mut time_sig_tab_label = wide_char_string("Time sigs");
        let time_sig_tab = TCITEMW{mask: TCIF_TEXT, dwState: 0, dwStateMask: 0,
            pszText: time_sig_tab_label.as_mut_ptr(), cchTextMax: 0, iImage: -1, lParam: 0};
        SendMessageW(control_tabs_handle, TCM_INSERTITEMW, TIME_SIG_TAB_INDEX as usize,
            &time_sig_tab as *const _ as isize);
        let time_sig_tab_handle = CreateWindowExW(0, static_string.as_ptr(), std::ptr::null(),
            WS_CHILD, 0, tab_top, 500, tab_height, control_tabs_handle, std::ptr::null_mut(),
            instance, std::ptr::null_mut());
        SetWindowSubclass(time_sig_tab_handle, Some(time_sig_tab_proc), 0, 0);
        let numerator_label_handle = CreateWindowExW(0, static_string.as_ptr(),
            wide_char_string("Numerator:").as_ptr(), SS_LEFT | WS_CHILD | WS_VISIBLE, 5, 0, 90, 20,
            time_sig_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(numerator_label_handle, WM_SETFONT, text_font as usize, 0);
        let numerator_display_handle = CreateWindowExW(0, static_string.as_ptr(),
            std::ptr::null_mut(), WS_BORDER | WS_CHILD | WS_VISIBLE, 90, 0, 45, 20,
            time_sig_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(numerator_display_handle, WM_SETFONT, text_font as usize, 0);
        let numerator_spin_handle =
            CreateWindowExW(0, wide_char_string(UPDOWN_CLASS).as_ptr(), std::ptr::null(),
                UDS_ALIGNRIGHT | UDS_AUTOBUDDY | UDS_SETBUDDYINT | WS_CHILD | WS_VISIBLE, 0, 0, 0,
                0, time_sig_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(numerator_spin_handle, UDM_SETRANGE32, 0, 100);
        SendMessageW(numerator_spin_handle, UDM_SETPOS32, 0, 4);
        let denominator_label_handle = CreateWindowExW(0, static_string.as_ptr(),
            wide_char_string("Denominator:").as_ptr(), SS_LEFT | WS_CHILD | WS_VISIBLE, 5, 20, 90,
            20, time_sig_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(denominator_label_handle, WM_SETFONT, text_font as usize, 0);
        let denominator_display_handle = CreateWindowExW(0, static_string.as_ptr(),
            wide_char_string("4").as_ptr(), WS_BORDER | WS_CHILD | WS_VISIBLE, 90, 20, 45, 20,
            time_sig_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(denominator_display_handle, WM_SETFONT, text_font as usize, 0);
        let denominator_spin_handle = CreateWindowExW(0, wide_char_string(UPDOWN_CLASS).as_ptr(),
            std::ptr::null(), UDS_ALIGNRIGHT | UDS_AUTOBUDDY | WS_CHILD | WS_VISIBLE, 0, 0, 0, 0,
            time_sig_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(denominator_spin_handle, UDM_SETRANGE32, MIN_LOG2_DURATION as usize, 0);
        SendMessageW(denominator_spin_handle, UDM_SETPOS32, 0, -2);
        let add_time_sig_button_handle = CreateWindowExW(0, button_string.as_ptr(),
            wide_char_string("Add time signature").as_ptr(),
            BS_PUSHBUTTON | BS_VCENTER | WS_DISABLED | WS_CHILD | WS_VISIBLE, 145, 10, 115, 20,
            time_sig_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(add_time_sig_button_handle, WM_SETFONT, text_font as usize, 0);
        let mut note_tab_label = wide_char_string("Notes");
        let note_tab = TCITEMW{mask: TCIF_TEXT, dwState: 0, dwStateMask: 0,
            pszText: note_tab_label.as_mut_ptr(), cchTextMax: 0, iImage: -1, lParam: 0};
        SendMessageW(control_tabs_handle, TCM_INSERTITEMW, NOTE_TAB_INDEX as usize,
        &note_tab as *const _ as isize);
        let note_tab_handle = CreateWindowExW(0, static_string.as_ptr(), std::ptr::null(), WS_CHILD,
            0, tab_top, 500, tab_height, control_tabs_handle, std::ptr::null_mut(), instance,
            std::ptr::null_mut());
        SetWindowSubclass(note_tab_handle, Some(note_tab_proc), 0, 0);
        let mut x = 0;
        let label_height = 20;
        let duration_label_handle = CreateWindowExW(0, static_string.as_ptr(),
            wide_char_string("Duration:").as_ptr(), SS_CENTER | WS_CHILD | WS_VISIBLE, 0, 0, 110,
            label_height, note_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(duration_label_handle, WM_SETFONT, text_font as usize, 0);
        let duration_display_handle = CreateWindowExW(0, static_string.as_ptr(),
            wide_char_string("quarter").as_ptr(), WS_BORDER | WS_CHILD | WS_VISIBLE, x,
            label_height, 110, label_height, note_tab_handle, std::ptr::null_mut(), instance,
            std::ptr::null_mut());
        SendMessageW(duration_display_handle, WM_SETFONT, text_font as usize, 0);
        let duration_spin_handle = CreateWindowExW(0, wide_char_string(UPDOWN_CLASS).as_ptr(),
            std::ptr::null(), UDS_ALIGNRIGHT | UDS_AUTOBUDDY | WS_CHILD | WS_VISIBLE, 0, 0, 0, 0,
            note_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(duration_spin_handle, UDM_SETRANGE32, MIN_LOG2_DURATION as usize,
            MAX_LOG2_DURATION as isize);
        SendMessageW(duration_spin_handle, UDM_SETPOS32, 0, -2);
        x += 110;
        let augmentation_dot_label_handle = CreateWindowExW(0, static_string.as_ptr(),
            wide_char_string("Augmentation dots:").as_ptr(), SS_CENTER | WS_CHILD | WS_VISIBLE, x,
            0, 110, 20, note_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(augmentation_dot_label_handle, WM_SETFONT, text_font as usize, 0);
        let augmentation_dot_display_handle = CreateWindowExW(0, static_string.as_ptr(),
            std::ptr::null(), WS_BORDER | WS_VISIBLE | WS_CHILD, x, label_height, 110, 20,
            note_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(augmentation_dot_display_handle, WM_SETFONT, text_font as usize, 0);
        let augmentation_dot_spin_handle = CreateWindowExW(0,
            wide_char_string(UPDOWN_CLASS).as_ptr(), std::ptr::null(),
            UDS_ALIGNRIGHT | UDS_AUTOBUDDY | UDS_SETBUDDYINT | WS_CHILD | WS_VISIBLE, 0, 0, 0, 0,
            note_tab_handle, std::ptr::null_mut(), instance, std::ptr::null_mut());
        SendMessageW(augmentation_dot_spin_handle, UDM_SETRANGE32, 0,
            (-2 - MIN_LOG2_DURATION) as isize);
        main_window_memory = Project{default_staff_space_height: 10.0,
            staff_scales: vec![StaffScale{name: unterminated_wide_char_string("Default"),
                value: 1.0},
            StaffScale{name: unterminated_wide_char_string("Cue"), value: 0.75}],
            slices: vec![Slice{address: 0, object_addresses: vec![], whole_notes_long: None,
                distance_from_previous_slice: 20},
                Slice{address: 1, object_addresses: vec![], whole_notes_long: None,
                    distance_from_previous_slice: 0},
                Slice{address: 2, object_addresses: vec![], whole_notes_long: None,
                    distance_from_previous_slice: 0},
                Slice{address: 3, object_addresses: vec![], whole_notes_long: None,
                    distance_from_previous_slice: 0},
                Slice{address: 4, object_addresses: vec![],
                    whole_notes_long:
                        Some(num_rational::Ratio::from_integer(num_bigint::BigUint::new(vec![]))),
                    distance_from_previous_slice: 0}],
            slice_indices: vec![0, 1, 2, 3, 4], slice_address_free_list: vec![], staves: vec![],
            viewport_offset: POINT{x: 0, y: 0}, leftmost_visible_slice_address: 0,
            highest_visible_staff_index: 0, x_of_slice_beyond_leftmost_visible: 0,
            y_of_staff_above_highest_visible: 0, last_slice_x: 20,
            bottom_staff_y: DEFAULT_TOP_STAFF_MIDDLE_Y, ghost_cursor: None,
            selection: Selection::None, zoom_exponent: 0, control_tabs_handle: control_tabs_handle,
            staff_tab_handle: staff_tab_handle, add_staff_button_handle: add_staff_button_handle,
            main_window_back_buffer: back_buffer, clef_tab_handle: clef_tab_handle,
            c_clef_handle: c_clef_handle, f_clef_handle: f_clef_handle,
            g_clef_handle: g_clef_handle, clef_15ma_handle: clef_15ma_handle,
            clef_8va_handle: clef_8va_handle, clef_none_handle: clef_none_handle,
            clef_8vb_handle: clef_8vb_handle, clef_15mb_handle: clef_15mb_handle,
            add_clef_button_handle: add_clef_button_handle, key_sig_tab_handle: key_sig_tab_handle,
            accidental_count_spin_handle: accidental_count_spin_handle,
            sharps_handle: sharps_handle, flats_handle: flats_handle,
            add_key_sig_button_handle: add_key_sig_button_handle,
            time_sig_tab_handle: time_sig_tab_handle, numerator_spin_handle: numerator_spin_handle,
            denominator_display_handle: denominator_display_handle,
            denominator_spin_handle: denominator_spin_handle,
            add_time_sig_button_handle: add_time_sig_button_handle,
            note_tab_handle: note_tab_handle, duration_display_handle: duration_display_handle,
            duration_spin_handle: duration_spin_handle,
            augmentation_dot_spin_handle: augmentation_dot_spin_handle};
        if SetWindowLongPtrW(main_window_handle, GWLP_USERDATA,
            &mut main_window_memory as *mut _ as isize) == 0xe050
        {
            panic!("Failed to set main window extra memory; error code {}", GetLastError());
        }
        ShowWindow(main_window_handle, SW_MAXIMIZE);
        message = std::mem::uninitialized(); 
    }
    while get_message(&mut message, main_window_handle) > 0
    {
        translate_message(&message);
        dispatch_message(&message);
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
                    let staff_index;
                    let object_address;
                    let mut octave4_cursor_range_floor;
                    let mut octaves_of_range_floor_above_octave4;
                    if let Selection::ActiveCursor{address, range_floor} = &project.selection
                    {
                        staff_index = address.staff_index;
                        object_address = address.object_address;
                        octave4_cursor_range_floor = *range_floor % 7;
                        octaves_of_range_floor_above_octave4 = *range_floor / 7;
                    }
                    else
                    {
                        return 0;
                    }
                    let scale_degree = (w_param as i8 - 60) % 7;
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
                    let duration_log2 =
                        SendMessageW(project.duration_spin_handle, UDM_GETPOS32, 0, 0) as i8;
                    let augmentation_dot_count = SendMessageW(project.augmentation_dot_spin_handle,
                        UDM_GETPOS32, 0, 0) as u8;
                    let mut slice_addresses_to_respace = vec![];
                    let mut cursor_index =
                        project.staves[staff_index].object_indices[object_address];
                    let slice_index = remove_objects_between_cursor_and_next_duration_slice(project,
                        &mut slice_addresses_to_respace, staff_index, &mut cursor_index);
                    let accidental =
                        default_accidental_of_steps_above_c4(project.staves[staff_index].objects.
                            split_at(cursor_index).0, steps_above_c4);
                    let accidental_address;
                    if accidental.is_visible
                    {
                        project.staves[staff_index].objects[cursor_index].is_valid_cursor_position =
                            false;
                        let note_address =
                            project.staves[staff_index].objects[cursor_index].address;
                        accidental_address = Some(insert_object(&mut slice_addresses_to_respace,
                            &mut project.staves[staff_index], cursor_index,
                            Object{object_type:
                                ObjectType::Accidental{note_address: note_address},
                                address: 0, slice_address: None, distance_to_next_slice: 0,
                                is_selected: false, is_valid_cursor_position: true}));
                        cursor_index += 1;
                    }
                    else
                    {
                        accidental_address = None;
                    }
                    let new_object_address = overwrite_with_duration(project,
                        &mut slice_addresses_to_respace, staff_index, cursor_index, slice_index,
                        Duration{pitch:
                            Some(NotePitch{accidental_address: accidental_address,
                                pitch: Pitch{steps_above_c4: steps_above_c4,
                                    accidental: accidental.accidental}}),
                            log2: duration_log2, augmentation_dot_count: augmentation_dot_count});
                    if let Selection::ActiveCursor{address, range_floor} =
                        &mut (*project).selection
                    {           
                        address.object_address = new_object_address;
                        *range_floor = clamped_subtract(steps_above_c4, 3);
                    }
                    respace_slices(window_handle, project, &slice_addresses_to_respace);
                    return 0;
                },
                VK_BACK =>
                {
                    let project = project_memory(window_handle);
                    match &project.selection
                    {
                        Selection::ActiveCursor{address,..} =>
                        {
                            let selection_object_index = project.staves[address.staff_index].
                                object_indices[address.object_address];
                            if selection_object_index > 0
                            {
                                let staff_index = address.staff_index;
                                delete_object(window_handle, project, staff_index,
                                    selection_object_index - 1);
                            }
                        },
                        Selection::None => (),
                        Selection::Object(address) =>
                        {
                            let staff_index = address.staff_index;
                            let object_index =
                                project.staves[staff_index].object_indices[address.object_address];
                            delete_object(window_handle, project, staff_index, object_index);
                        }
                    }
                    return 0;
                },
                VK_DELETE =>
                {
                    let project = project_memory(window_handle);
                    if let Selection::Object(address) = &mut project.selection
                    {
                        let staff_index = address.staff_index;
                        let staff = &mut project.staves[staff_index];
                        let object_index = staff.object_indices[address.object_address];
                        staff.objects[object_index].is_selected = false;
                        delete_object(window_handle, project, staff_index, object_index);
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
                            *range_floor = clamped_subtract(*range_floor, 7);
                            invalidate_work_region(window_handle);
                        },
                        Selection::Object(address) =>
                        {
                            let staff_line_count =
                                project.staves[address.staff_index].line_count as i8;
                            let staff_index = address.staff_index;
                            let staff = &mut project.staves[staff_index];
                            let object_index = staff.object_indices[address.object_address];
                            let (previous_objects, remaining_objects) =
                                staff.objects.split_at_mut(object_index);
                            match &mut remaining_objects[0].object_type
                            {
                                ObjectType::Clef(clef) =>
                                {
                                    let new_baseline =
                                        clef.steps_of_baseline_above_staff_middle - 1;
                                    if new_baseline > -staff_line_count
                                    {
                                        clef.steps_of_baseline_above_staff_middle = new_baseline;
                                    }
                                    invalidate_work_region(window_handle);
                                },
                                ObjectType::Duration(duration) =>
                                {
                                    if let Some(pitch) = &mut duration.pitch
                                    {
                                        if HIBYTE(GetKeyState(VK_SHIFT) as u16) == 0        
                                        {
                                            if pitch.pitch.steps_above_c4 > i8::min_value()
                                            {
                                                pitch.pitch.steps_above_c4 -= 1;
                                            }
                                            let steps_above_c4 = pitch.pitch.steps_above_c4;
                                            pitch.pitch.accidental =
                                                default_accidental_of_steps_above_c4(
                                                previous_objects, steps_above_c4).accidental;
                                        }
                                        else
                                        {
                                            pitch.pitch.accidental =
                                            match pitch.pitch.accidental
                                            {
                                                Accidental::DoubleSharp => Accidental::Sharp,
                                                Accidental::Sharp => Accidental::Natural,
                                                Accidental::Natural => Accidental::Flat,
                                                Accidental::Flat => Accidental::DoubleFlat,
                                                Accidental::DoubleFlat => return 0
                                            };
                                        }
                                        let mut slice_addresses_to_respace = vec![];
                                        reset_accidental_displays_from_previous_key_sig(project,
                                            &mut slice_addresses_to_respace, staff_index,
                                            object_index);
                                        respace_slices(window_handle, project,
                                            &slice_addresses_to_respace);
                                    }
                                },
                                _ => ()
                            }
                        },
                        Selection::None => ()
                    }
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
                        let staff = &project.staves[address.staff_index];
                        let mut previous_object_index =
                            staff.object_indices[address.object_address];
                        let mut new_range_floor = *range_floor;
                        while previous_object_index > 0
                        {
                            previous_object_index -= 1;
                            let object = &staff.objects[previous_object_index];
                            if let ObjectType::Duration(duration) = &object.object_type
                            {
                                if let Some(pitch) = &duration.pitch
                                {
                                    new_range_floor =
                                        clamped_subtract(pitch.pitch.steps_above_c4, 3);
                                }
                            }
                            if object.is_valid_cursor_position
                            {
                                address.object_address = object.address;
                                *range_floor = new_range_floor;
                                invalidate_work_region(window_handle);
                                return 0;
                            }
                        }
                    }                   
                    return 0;
                },
                VK_RIGHT =>
                {
                    let project = project_memory(window_handle);
                    if let Selection::ActiveCursor{address, range_floor} = &mut project.selection
                    {
                        let range_floor = *range_floor;
                        let staff_index = address.staff_index;
                        let staff = &project.staves[staff_index];
                        let object_index = staff.object_indices[address.object_address];
                        set_cursor_to_next_state(project, staff_index, object_index, range_floor);
                        invalidate_work_region(window_handle);
                    }
                    return 0;
                },
                VK_SPACE =>
                {
                    let project = project_memory(window_handle);
                    let staff_index;
                    let object_address;
                    if let Selection::ActiveCursor{address,..} = &project.selection
                    {
                        staff_index = address.staff_index;
                        object_address = address.object_address;
                    }
                    else
                    {
                        return 0;
                    }
                    let duration_log2 =
                        SendMessageW(project.duration_spin_handle, UDM_GETPOS32, 0, 0) as i8;
                    let augmentation_dot_count = SendMessageW(project.augmentation_dot_spin_handle,
                        UDM_GETPOS32, 0, 0) as u8;
                    let mut slice_addresses_to_respace = vec![];
                    let mut cursor_index =
                        project.staves[staff_index].object_indices[object_address];
                    let slice_index = remove_objects_between_cursor_and_next_duration_slice(project,
                        &mut slice_addresses_to_respace, staff_index, &mut cursor_index);
                    let new_object_address = overwrite_with_duration(project,
                        &mut slice_addresses_to_respace, staff_index, cursor_index, slice_index,
                        Duration{pitch: None, log2: duration_log2,
                            augmentation_dot_count: augmentation_dot_count});
                    if let Selection::ActiveCursor{address,..} = &mut project.selection
                    {
                        address.object_address = new_object_address;
                    }
                    respace_slices(window_handle, project, &slice_addresses_to_respace);
                    return 0;
                },
                VK_UP =>
                {
                    let project = project_memory(window_handle);
                    match &mut project.selection
                    {
                        Selection::ActiveCursor{range_floor,..} =>
                        {
                            *range_floor = clamped_add(*range_floor, 7);
                            invalidate_work_region(window_handle);
                        },
                        Selection::Object(address) =>
                        {
                            let staff_line_count =
                                project.staves[address.staff_index].line_count as i8;
                            let staff_index = address.staff_index;
                            let staff = &mut project.staves[staff_index];
                            let object_index = staff.object_indices[address.object_address];
                            let (previous_objects, remaining_objects) =
                                staff.objects.split_at_mut(object_index);
                            match &mut remaining_objects[0].object_type
                            {
                                ObjectType::Clef(clef) =>
                                {
                                    let new_baseline =
                                        clef.steps_of_baseline_above_staff_middle + 1;
                                    if new_baseline < staff_line_count
                                    {
                                        clef.steps_of_baseline_above_staff_middle = new_baseline;
                                    }
                                    invalidate_work_region(window_handle);
                                },
                                ObjectType::Duration(duration) =>
                                {
                                    if let Some(pitch) = &mut duration.pitch
                                    {
                                        if HIBYTE(GetKeyState(VK_SHIFT) as u16) == 0        
                                        {
                                            if pitch.pitch.steps_above_c4 < i8::max_value()
                                            {
                                                pitch.pitch.steps_above_c4 += 1;
                                            }
                                            pitch.pitch.accidental =
                                                default_accidental_of_steps_above_c4(
                                                    previous_objects, pitch.pitch.steps_above_c4).
                                                accidental;
                                        }
                                        else
                                        {
                                            pitch.pitch.accidental =
                                            match pitch.pitch.accidental
                                            {
                                                Accidental::DoubleSharp => return 0,
                                                Accidental::Sharp => Accidental::DoubleSharp,
                                                Accidental::Natural => Accidental::Sharp,
                                                Accidental::Flat => Accidental::Natural,
                                                Accidental::DoubleFlat => Accidental::Flat
                                            };
                                        }
                                        let mut slice_addresses_to_respace = vec![];
                                        reset_accidental_displays_from_previous_key_sig(project,
                                            &mut slice_addresses_to_respace, staff_index,
                                            object_index);
                                        respace_slices(window_handle, project,
                                            &slice_addresses_to_respace);
                                    }
                                },
                                _ => ()
                            }
                        },
                        Selection::None => ()
                    }
                    return 0;
                },
                _ => ()
            }            
        },
        WM_LBUTTONDOWN =>
        {
            let project = project_memory(window_handle);
            let zoom_factor = zoom_factor(project.zoom_exponent);
            let click_x = GET_X_LPARAM(l_param);
            let click_y = GET_Y_LPARAM(l_param);
            let device_context = get_dc(window_handle);
            let back_buffer_device_context = CreateCompatibleDC(device_context);
            release_dc(window_handle, device_context);
            save_dc(back_buffer_device_context);
            SelectObject(back_buffer_device_context,
                project.main_window_back_buffer as *mut winapi::ctypes::c_void);
            SetBkMode(back_buffer_device_context, TRANSPARENT as i32);            
            SetTextAlign(back_buffer_device_context, TA_BASELINE);
            set_text_color(back_buffer_device_context, WHITE);
            select_object(back_buffer_device_context, GetStockObject(WHITE_PEN as i32));
            select_object(back_buffer_device_context, GetStockObject(WHITE_BRUSH as i32));
            let mut client_rect: RECT = std::mem::uninitialized();
            GetClientRect(window_handle, &mut client_rect);
            FillRect(back_buffer_device_context, &client_rect,
                GetStockObject(BLACK_BRUSH as i32) as HBRUSH);
            let mut staff_middle_y =
                project.y_of_staff_above_highest_visible - project.viewport_offset.y;
            for staff_index in 0..project.staves.len()
            {        
                staff_middle_y += project.staves[staff_index].distance_from_staff_above;       
                if let Some(address) =
                    address_of_clicked_staff_object(project, back_buffer_device_context,
                        zoom_factor, staff_index, staff_middle_y, click_x, click_y)
                {
                    cancel_selection(window_handle);
                    let staff = &mut project.staves[staff_index];
                    let object = &mut staff.objects[staff.object_indices[address.object_address]];
                    object.is_selected = true;
                    if object_is_header(object)
                    {
                        enable_add_header_object_buttons(project, TRUE);
                    }
                    project.selection = Selection::Object(address);
                    restore_dc(back_buffer_device_context);
                    release_dc(window_handle, back_buffer_device_context);
                    invalidate_work_region(window_handle);
                    return 0;
                }
            }
            match project.ghost_cursor
            {
                Some(_) =>
                {
                    cancel_selection(window_handle);
                    set_active_cursor(std::mem::replace(&mut project.ghost_cursor, None).unwrap(),
                        3, project);
                    invalidate_work_region(window_handle);
                },
                _ => ()
            }
            restore_dc(back_buffer_device_context);
            release_dc(window_handle, back_buffer_device_context);
            return 0;
        },
        WM_MOUSEMOVE =>
        {
            let project = project_memory(window_handle);
            if let Some(address) =
                ghost_cursor_address(project, GET_X_LPARAM(l_param), GET_Y_LPARAM(l_param))
            {
                if let Some(current_address) = &project.ghost_cursor
                {
                    if address == *current_address
                    {
                        return 0;
                    }
                }
                project.ghost_cursor = Some(address);
                invalidate_work_region(window_handle);               
                return 0;
            }
            if let Some(_) = &project.ghost_cursor
            {                 
                project.ghost_cursor = None;   
                invalidate_work_region(window_handle);
            }
            return 0;
        },
        WM_MOUSEWHEEL =>
        {
            let mut project = project_memory(window_handle);
            if project.staves.len() == 0
            {
                return 0;
            }
            let delta = HIWORD(w_param as u32) as i16;
            let current_zoom_factor = zoom_factor(project.zoom_exponent);
            let virtual_key = LOWORD(w_param as u32) as usize;
            if virtual_key == MK_CONTROL
            {
                if delta > 0
                {
                    project.zoom_exponent = clamped_subtract(project.zoom_exponent, 1);
                }
                else
                {
                    project.zoom_exponent = clamped_add(project.zoom_exponent, 1);
                }
                let new_zoom_factor = zoom_factor(project.zoom_exponent);
                let cursor_x = GET_X_LPARAM(l_param) as f32;
                let cursor_y = GET_Y_LPARAM(l_param) as f32;
                let new_viewport_offset_x = project.viewport_offset.x +
                    (cursor_x / current_zoom_factor - cursor_x / new_zoom_factor).round() as i32;
                let new_viewport_offset_y = project.viewport_offset.y +
                    (cursor_y / current_zoom_factor - cursor_y / new_zoom_factor).round() as i32;
                reset_viewport_offset_x(window_handle, &mut project, new_viewport_offset_x);
                reset_viewport_offset_y(window_handle, &mut project, new_viewport_offset_y);
            }
            else
            {
                let shift = to_screen_coordinate(delta as f32,
                    1.0 / (WHEEL_DELTA_SCALE * current_zoom_factor));
                if virtual_key == MK_SHIFT
                {
                    let new_viewport_offset_x = project.viewport_offset.x + shift;
                    reset_viewport_offset_x(window_handle, &mut project, new_viewport_offset_x);
                }
                else
                {
                    let new_viewport_offset_y = project.viewport_offset.y + shift;
                    reset_viewport_offset_y(window_handle, &mut project, new_viewport_offset_y);
                }
            }
            invalidate_work_region(window_handle);
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
                        TIME_SIG_TAB_INDEX =>
                        {
                            ShowWindow(project.time_sig_tab_handle, SW_SHOW);
                            SendMessageW(project.time_sig_tab_handle, WM_ENABLE, TRUE as usize, 0);
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
                        TIME_SIG_TAB_INDEX =>
                        {
                            ShowWindow(project.time_sig_tab_handle, SW_HIDE);
                            SendMessageW(project.time_sig_tab_handle, WM_ENABLE, FALSE as usize, 0);
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
            let zoom_factor = zoom_factor(project.zoom_exponent);
            let mut paint_struct: PAINTSTRUCT = std::mem::uninitialized();
            let device_context = BeginPaint(window_handle, &mut paint_struct as *mut _);
            let back_buffer_device_context = CreateCompatibleDC(device_context);
            save_dc(back_buffer_device_context);
            SelectObject(back_buffer_device_context,
                project.main_window_back_buffer as *mut winapi::ctypes::c_void);
            SetBkMode(back_buffer_device_context, TRANSPARENT as i32);
            SetTextAlign(back_buffer_device_context, TA_BASELINE);
            select_object(back_buffer_device_context, GetStockObject(BLACK_PEN as i32));
            select_object(back_buffer_device_context, GetStockObject(BLACK_BRUSH as i32)); 
            set_text_color(back_buffer_device_context, BLACK);
            FillRect(back_buffer_device_context, &paint_struct.rcPaint,
                GetStockObject(WHITE_BRUSH as i32) as HBRUSH);
            let mut staff_middle_y =
                project.y_of_staff_above_highest_visible - project.viewport_offset.y;
            for staff_index in project.highest_visible_staff_index..project.staves.len()
            {
                let staff = &project.staves[staff_index];
                staff_middle_y += staff.distance_from_staff_above;
                if to_screen_coordinate(y_of_steps_above_bottom_line(staff_middle_y, 
                    project.default_staff_space_height *
                        project.staff_scales[staff.scale_index].value,
                    staff.line_count, 2 * (staff.line_count as i8 - 1)) as f32, zoom_factor) >=
                    paint_struct.rcPaint.bottom
                {
                    break;
                }
                draw_staff(back_buffer_device_context, project, staff_index, staff_middle_y,
                    paint_struct.rcPaint.right);
            }      
            BitBlt(device_context, paint_struct.rcPaint.left, paint_struct.rcPaint.top,
                paint_struct.rcPaint.right - paint_struct.rcPaint.left,
                paint_struct.rcPaint.bottom - paint_struct.rcPaint.top,                
                back_buffer_device_context, paint_struct.rcPaint.left, paint_struct.rcPaint.top,
                SRCCOPY);
            restore_dc(back_buffer_device_context);
            EndPaint(window_handle, &mut paint_struct as *mut _);
        },
        WM_SIZE =>
        {
            let project = GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut Project;
            if project != std::ptr::null_mut()
            {
                let project = &mut *project;
                let mut client_rect = std::mem::uninitialized();
                GetClientRect(window_handle, &mut client_rect);
                let width = client_rect.right - client_rect.left;
                let device_context = get_dc(window_handle);
                DeleteObject(project.main_window_back_buffer as *mut winapi::ctypes::c_void);
                project.main_window_back_buffer = CreateCompatibleBitmap(device_context, width,
                    client_rect.bottom - client_rect.top);
                release_dc(window_handle, device_context);
                SetWindowPos(project.control_tabs_handle, std::ptr::null_mut(), client_rect.left, 0,
                    width, 70, 0);
                InvalidateRect(window_handle, &client_rect, FALSE);
            }
            return 0;
        }, 
        _ => ()
    }
    DefWindowProcW(window_handle, u_msg, w_param, l_param)
}

fn new_address(indices: &mut Vec<usize>, address_free_list: &mut Vec<usize>, index: usize) ->
    usize
{
    if let Some(address) = address_free_list.pop()
    {
        indices[address] = index;
        address
    }
    else
    {
        let address = indices.len();
        indices.push(index);
        address
    }
}

fn new_key_sig(accidental_count_spin_handle: HWND, flats_handle: HWND, staff: &Staff,
    mut object_index: usize) -> Option<KeySig>
{
    let accidental_count =
    unsafe
    {
        SendMessageW(accidental_count_spin_handle, UDM_GETPOS32, 0, 0)
    };
    if accidental_count == 0
    {
        loop
        {
            if object_index == 0
            {
                return None;
            }
            object_index -= 1;
            if let ObjectType::KeySig(previous_key_sig) = &staff.objects[object_index].object_type
            {
                if previous_key_sig.accidentals.len() == 0
                {
                    return None;
                }
                let mut new_key_sig = KeySig{accidentals: vec![], floors: previous_key_sig.floors};
                for accidental in &previous_key_sig.accidentals
                {
                    new_key_sig.accidentals.push(KeySigAccidental{accidental: Accidental::Natural,
                        letter_name: accidental.letter_name});
                }
                return Some(new_key_sig);
            }
        }
    }
    let floors;
    let accidental_type;
    let stride;
    let mut next_letter_name;
    if button_is_checked(flats_handle)
    {
        floors = [-4, -5, -4, -5, -1, -2, -3];
        accidental_type = Accidental::Flat;
        stride = 3;
        next_letter_name = LETTER_NAME_B;
    }
    else
    {
        floors = [-2, -3, -4, -5, -1, -2, -1];
        accidental_type = Accidental::Sharp;
        stride = 4;
        next_letter_name = LETTER_NAME_F;
    }
    let mut new_key_sig = KeySig{accidentals: vec![], floors: floors};
    for _ in 0..accidental_count
    {
        new_key_sig.accidentals.push(KeySigAccidental{accidental: accidental_type,
            letter_name: next_letter_name});
        next_letter_name = (next_letter_name + stride) % 7;
    }
    Some(new_key_sig)
}

fn next_slice_address(staff: &Staff, mut object_index: usize) -> usize
{
    loop
    {
        if let Some(slice_address) = staff.objects[object_index].slice_address
        {
            return slice_address
        }
        object_index += 1;
    }
}

fn next_valid_cursor_index(staff: &Staff, object_index: &mut usize)
{
    loop
    {
        if staff.objects[*object_index].is_valid_cursor_position
        {
            return;
        }
        *object_index += 1;
    }
}

fn note_pitch(staff: &Staff, note_address: usize) -> &Pitch
{
    if let ObjectType::Duration(duration) =
        &staff.objects[staff.object_indices[note_address]].object_type
    {
        if let Some(pitch) = &duration.pitch
        {
            &pitch.pitch
        }
        else
        {
            panic!("Note address index identified rest.");
        }
    }
    else
    {
        panic!("Note address index identified non-duration.");
    }
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

fn object_as_key_sig<'a>(staff: &'a mut Staff, object_index: usize) -> &'a mut KeySig
{
    if let ObjectType::KeySig(key_sig) = &mut staff.objects[object_index].object_type
    {
        key_sig
    }
    else
    {
        panic!("Key sig index didn't point to key sig.");
    }
}

fn object_as_maybe_pitch<'a>(staff: &'a mut Staff, object_index: usize) -> &'a mut Option<NotePitch>
{
    if let ObjectType::Duration(duration) = &mut staff.objects[object_index].object_type
    {
        &mut duration.pitch
    }
    else
    {
        panic!("Note index didn't point to duration.");
    }
}

fn object_as_pitch<'a>(staff: &'a mut Staff, object_index: usize) -> &'a mut NotePitch
{
    if let Some(pitch) = object_as_maybe_pitch(staff, object_index)
    {
        pitch
    }
    else
    {
        panic!("Note index pointed to rest.");
    }
}

fn object_is_header(object: &Object) -> bool
{
    if let Some(address) = object.slice_address
    {
        if address >= HEADER_CLEF_SLICE_INDEX && address <= HEADER_TIME_SIG_SLICE_INDEX
        {
            return true;
        }
    }
    false
}

fn overwrite_with_duration(project: &mut Project, slice_addresses_to_respace: &mut Vec<usize>,
    staff_index: usize, mut object_index: usize, mut slice_index: usize,
    duration: Duration) -> usize
{
    let mut remaining_whole_notes_long = whole_notes_long(&duration);
    project.staves[staff_index].objects[object_index].object_type = ObjectType::Duration(duration);
    object_index += 1;
    let mut new_cursor_address;
    loop 
    {
        if object_index == project.staves[staff_index].objects.len()
        {
            return insert_rhythmic_slice_object(project, slice_addresses_to_respace, staff_index,
                ObjectType::None, object_index, &mut slice_index, remaining_whole_notes_long);
        }
        let object = &project.staves[staff_index].objects[object_index];
        if let Some(slice_address) = object.slice_address
        {
            if let Some(whole_notes_long) =
                &project.slices[project.slice_indices[slice_address]].whole_notes_long
            {
                if *whole_notes_long >= remaining_whole_notes_long
                {
                    new_cursor_address = object.address;
                    if let ObjectType::Duration(duration) = &object.object_type
                    {
                        if let Some(pitch) = &duration.pitch
                        {
                            if let Some(address) = pitch.accidental_address
                            {
                                new_cursor_address = address;
                            }
                        }
                    }
                    remaining_whole_notes_long = whole_notes_long - remaining_whole_notes_long;
                    break;
                }
            }
        }
        match &object.object_type
        {
            ObjectType::Accidental{..} => object_index += 1,
            ObjectType::Barline{..} => object_index += 1,
            _ => object_index = object_index + 1 - remove_object(project,
                slice_addresses_to_respace, staff_index, object_index)
        }
    }
    let mut denominator = remaining_whole_notes_long.denom().clone();
    let mut numerator = remaining_whole_notes_long.numer().clone();
    let mut rest_duration_log2 = 0;
    let zero = num_bigint::BigUint::new(vec![]);
    let two = num_bigint::BigUint::new(vec![2]);
    while denominator != zero
    {
        let division = numerator.div_rem(&denominator);
        denominator /= &two;
        if division.0 != zero
        {
            let old_remaining_whole_notes_long = remaining_whole_notes_long;
            let rest = Duration{log2: rest_duration_log2, pitch: None, augmentation_dot_count: 0};
            remaining_whole_notes_long =
                &old_remaining_whole_notes_long - whole_notes_long(&rest);
            insert_rhythmic_slice_object(project, slice_addresses_to_respace, staff_index,
                ObjectType::Duration(rest), object_index, &mut slice_index,
                old_remaining_whole_notes_long);
            numerator = division.1;            
            object_index += 1;
        }
        rest_duration_log2 -= 1;
    }
    new_cursor_address
}

fn project_memory<'a>(main_window_handle: HWND) -> &'a mut Project
{
    unsafe
    {
        &mut *(GetWindowLongPtrW(main_window_handle, GWLP_USERDATA) as *mut Project)
    }
}

fn push_if_not_present(vec: &mut Vec<usize>, new_element: usize)
{
    if !vec.contains(&new_element)
    {
        vec.push(new_element);
    }
}

fn range_floor_at_index(staff: &Staff, mut object_index: usize) -> i8
{
    loop
    {
        object_index -= 1;
        match &staff.objects[object_index].object_type
        {
            ObjectType::Clef(clef) => return staff_middle_pitch(clef) - 3,
            ObjectType::Duration(duration) =>
            {
                if let Some(pitch) = &duration.pitch
                {
                    return clamped_subtract(pitch.pitch.steps_above_c4, 3);
                }
            },
            _ => ()
        }
    }
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

fn remove_object(project: &mut Project, slice_addresses_to_respace: &mut Vec<usize>,
    staff_index: usize, mut object_index: usize) -> usize
{
    if let Some(address) = &mut project.ghost_cursor
    {
        if address.staff_index == staff_index &&
            project.staves[staff_index].object_indices[address.object_address] == object_index
        {
            project.ghost_cursor = None;
        }
    }
    let mut removal_count = 1;
    let object = &mut project.staves[staff_index].objects[object_index];
    match &mut object.object_type
    {
        ObjectType::Accidental{note_address} =>
        {
            let address = *note_address;
            object.is_valid_cursor_position = true;
            let object_index = project.staves[staff_index].object_indices[address];
            object_as_pitch(&mut project.staves[staff_index], object_index).
                accidental_address = None;
        },
        ObjectType::Clef(_) =>
        {
            if object_is_header(object)
            {
                object.is_selected = false;
                return 0;
            }
        },
        ObjectType::Duration(duration) =>
        {
            if let Some(pitch) = &duration.pitch
            {
                if let Some(address) = pitch.accidental_address
                {
                    removal_count += remove_object(project, slice_addresses_to_respace, staff_index,
                        project.staves[staff_index].object_indices[address]);
                    object_index -= 1;
                }
                reset_accidental_displays_from_previous_key_sig(project, slice_addresses_to_respace,
                    staff_index, object_index);
            }
        },
        ObjectType::KeySig(_) =>
        {
            remove_object_from_slice(&mut project.slices, &mut project.slice_indices,
                slice_addresses_to_respace, staff_index,
                &project.staves[staff_index].objects[object_index]);
            basic_remove_object(slice_addresses_to_respace, &mut project.staves[staff_index],
                object_index);
            reset_accidental_displays_from_previous_key_sig(project, slice_addresses_to_respace,
                staff_index, object_index);
            return 1;
        },
        _ => ()
    }
    remove_object_from_slice(&mut project.slices, &mut project.slice_indices,
        slice_addresses_to_respace, staff_index,
        &project.staves[staff_index].objects[object_index]);
    basic_remove_object(slice_addresses_to_respace, &mut project.staves[staff_index],
        object_index);
    return removal_count;
}

fn remove_object_from_slice(slices: &mut Vec<Slice>, slice_indices: &mut Vec<usize>,
    slice_addresses_to_respace: &mut Vec<usize>, staff_index: usize, object: &Object)
{
    if let Some(slice_address) = object.slice_address
    {
        let slice_index = slice_indices[slice_address];
        let next_slice_index = slice_index + 1;
        if next_slice_index < slices.len()
        {
            push_if_not_present(slice_addresses_to_respace, slices[next_slice_index].address);
        }
        let objects_in_slice_count = slices[slice_index].object_addresses.len();
        if objects_in_slice_count == 1 && slice_index > HEADER_TIME_SIG_SLICE_INDEX
        {
            for address_index in 0..slice_addresses_to_respace.len()
            {
                if slice_addresses_to_respace[address_index] == slice_address
                {
                    slice_addresses_to_respace.remove(address_index);
                    break;
                }
            }
            for index in 0..slice_addresses_to_respace.len()
            {
                if slice_addresses_to_respace[index] == slice_address
                {
                    slice_addresses_to_respace.swap_remove(index);
                    break;
                }
            }
            slices.remove(slice_index);
            increment_slice_indices(slices, slice_indices, slice_index, decrement);                
        }
        else
        {
            for object_address_index in 0..objects_in_slice_count
            {
                if slices[slice_index].object_addresses[object_address_index].staff_index ==
                    staff_index
                {
                    slices[slice_index].object_addresses.remove(object_address_index);
                    break;
                }
            }
        }
    }
}

fn remove_objects_between_cursor_and_next_duration_slice(project: &mut Project,
    slice_addresses_to_respace: &mut Vec<usize>, staff_index: usize,
    cursor_index: &mut usize) -> usize
{
    loop
    {
        let staff = &mut project.staves[staff_index];
        if let Some(slice_address) = staff.objects[*cursor_index].slice_address
        {
            let slice_index = project.slice_indices[slice_address];
            if let Some(_) = &project.slices[slice_index].whole_notes_long
            {
                push_if_not_present(slice_addresses_to_respace,
                    next_slice_address(staff, *cursor_index));
                let duration_object = &mut staff.objects[*cursor_index];
                if let ObjectType::Duration(duration) = &duration_object.object_type
                {
                    if let Some(pitch) = &duration.pitch
                    {
                        if let Some(address) = pitch.accidental_address
                        {
                            duration_object.is_valid_cursor_position = true;
                            let object_index = project.staves[staff_index].object_indices[address];
                            remove_object(project, slice_addresses_to_respace, staff_index,
                                object_index);
                            *cursor_index -= 1;
                        }
                    }
                }
                return slice_index;
            }
        }
        *cursor_index += 1;
        match &mut project.staves[staff_index].objects[*cursor_index].object_type
        {
            ObjectType::Accidental{..} => (),
            ObjectType::Barline{..} => (),
            _ => *cursor_index -=
                remove_object(project, slice_addresses_to_respace, staff_index, *cursor_index)
        }
    }
}

fn reset_accidental_displays(project: &mut Project, slice_addresses_to_respace: &mut Vec<usize>,
    staff_index: usize, object_index: &mut usize, key_sig_accidentals: &[Accidental; 7]) -> bool
{
    let mut note_pitches = vec![vec![], vec![], vec![], vec![], vec![], vec![], vec![]];
    while *object_index < project.staves[staff_index].objects.len()
    {
        let object = &project.staves[staff_index].objects[*object_index];
        let address = object.address;
        match &object.object_type
        {
            ObjectType::Duration(duration) =>
            {
                if let Some(pitch) = &duration.pitch
                {
                    let scale_degree = pitch.pitch.steps_above_c4 as usize % 7;
                    let scale_degree_pitches = &mut note_pitches[scale_degree];
                    let show_accidental;
                    let mut pitch_index = scale_degree_pitches.len();
                    loop
                    {
                        if pitch_index == 0
                        {
                            show_accidental =
                                key_sig_accidentals[scale_degree] != pitch.pitch.accidental;
                            scale_degree_pitches.push(pitch.pitch);
                            break;
                        }
                        pitch_index -= 1;
                        let scale_degree_pitch = &mut scale_degree_pitches[pitch_index];
                        if scale_degree_pitch.steps_above_c4 == pitch.pitch.steps_above_c4
                        {
                            show_accidental =
                                scale_degree_pitch.accidental != pitch.pitch.accidental;
                            *scale_degree_pitch = pitch.pitch;
                            break;
                        }
                        if scale_degree_pitches[pitch_index].accidental != pitch.pitch.accidental
                        {
                            show_accidental = true;
                            scale_degree_pitches.push(pitch.pitch);
                            break;
                        }
                    }
                    if let Some(accidental_address) = pitch.accidental_address
                    {
                        if !show_accidental
                        {
                            project.staves[staff_index].objects[*object_index].
                                is_valid_cursor_position = true;
                            remove_object(project, slice_addresses_to_respace, staff_index,
                                project.staves[staff_index].object_indices[accidental_address]);
                        }
                        else
                        {
                            push_if_not_present(slice_addresses_to_respace,
                                next_slice_address(&project.staves[staff_index], *object_index));
                        }
                    }
                    else if show_accidental
                    {
                        project.staves[staff_index].objects[*object_index].
                            is_valid_cursor_position = false;
                        let note_address =
                            project.staves[staff_index].objects[*object_index].address;
                        let new_accidental_address =
                            insert_object(slice_addresses_to_respace,
                                &mut project.staves[staff_index], *object_index,
                                Object{object_type:
                                    ObjectType::Accidental{note_address: note_address},
                                    address: 0, slice_address: None, distance_to_next_slice: 0,
                                    is_selected: false, is_valid_cursor_position: true});
                        *object_index += 1;
                        object_as_pitch(&mut project.staves[staff_index], *object_index).
                            accidental_address = Some(new_accidental_address);
                    }
                }
            },
            ObjectType::KeySig(_) => return true,
            _ => ()
        }
        *object_index = project.staves[staff_index].object_indices[address] + 1;
    } 
    false
}

fn reset_accidental_displays_from_previous_key_sig(project: &mut Project,
    slice_addresses_to_respace: &mut Vec<usize>, staff_index: usize, mut object_index: usize)
{
    let staff_objects = &project.staves[staff_index].objects;
    let key_sig_accidentals;
    loop
    {
        if object_index == 0
        {
            key_sig_accidentals = [Accidental::Natural; 7];
            break;
        }
        if let ObjectType::KeySig(key_sig) = &staff_objects[object_index - 1].object_type
        {
            key_sig_accidentals = letter_name_accidentals_from_key_sig(key_sig);
            break;
        }
        object_index -= 1;
    }
    reset_accidental_displays(project, slice_addresses_to_respace, staff_index, &mut object_index,
        &key_sig_accidentals);
}

fn reset_distance_from_previous_slice(device_context: HDC, project: &mut Project,
    slice_index: usize)
{
    if slice_index == 0
    {
        return;
    }
    let mut distance_from_previous_slice = 0;
    let slice = &project.slices[slice_index];
    if let Some(_) = &slice.whole_notes_long
    {
        for previous_slice_index in (0..slice_index).rev()
        {
            if let Some(whole_notes_long) = &project.slices[previous_slice_index].whole_notes_long
            {
                let mut division = whole_notes_long.numer().div_rem(whole_notes_long.denom());
                let mut whole_notes_long_float = division.0.to_bytes_le()[0] as f32;
                let zero = num_bigint::BigUint::new(vec![]);
                let two = num_bigint::BigUint::new(vec![2]);
                let mut place_value = 2.0;
                while place_value > 0.0
                {
                    division = (&two * division.1).div_rem(whole_notes_long.denom());
                    whole_notes_long_float += division.0.to_bytes_le()[0] as f32 / place_value;
                    if division.1 == zero
                    {
                        break;
                    }
                    place_value *= 2.0;
                }
                distance_from_previous_slice =
                    (WHOLE_NOTE_WIDTH * project.default_staff_space_height *
                        DURATION_RATIO.powf(whole_notes_long_float.log2())).round() as i32;
                break;
            }
        }
    }
    for system_address in &slice.object_addresses
    {
        let staff = &mut project.staves[system_address.staff_index];
        let space_height =
            project.default_staff_space_height * project.staff_scales[staff.scale_index].value;
        let font_set = staff_font_set(space_height);
        let mut range_width = 0;
        let mut object_index = staff.object_indices[system_address.object_address];
        let object = &mut staff.objects[object_index];
        range_width += default_object_origin_to_slice_distance(space_height, object);
        object.distance_to_next_slice = range_width;
        loop
        {
            if object_index == 1
            {
                range_width += space_height.round() as i32;
                break;
            }
            let previous_object = &staff.objects[object_index - 1];
            match &previous_object.object_type
            {
                ObjectType::Accidental{note_address} =>
                {
                    range_width += character_width(device_context, font_set.full_size,
                        accidental_codepoint(&note_pitch(staff, *note_address).accidental) as u32) +
                        (space_height * DISTANCE_BETWEEN_ACCIDENTAL_AND_NOTE).round() as i32;
                },
                ObjectType::Barline{..} =>
                {
                    range_width += (project.default_staff_space_height *
                       (BRAVURA_METADATA.thin_barline_thickness + 1.0)).round() as i32;
                },
                ObjectType::Clef(clef) =>
                {
                    let mut spacer = 1.0;
                    let font =
                    if object_is_header(previous_object)
                    {
                        match &staff.objects[object_index].object_type
                        {
                            ObjectType::Accidental{..} =>
                            {
                                if let ObjectType::Accidental{..} =
                                    &staff.objects[object_index + 1].object_type
                                {}
                                else
                                {
                                    spacer = 1.5;
                                }
                            },
                            ObjectType::Duration{..} =>
                            {
                                spacer = 2.5;
                            },
                            _ => ()
                        };
                        font_set.full_size
                    }
                    else
                    {
                        font_set.two_thirds_size
                    };
                    range_width += (space_height * spacer).round() as i32 +
                        character_width(device_context, font, clef.codepoint as u32);
                },
                ObjectType::Duration(duration) =>
                {
                    let spacer = 
                    if let ObjectType::Duration{..} = &staff.objects[object_index].object_type
                    {
                        0.0
                    }
                    else
                    {
                        1.0
                    };
                    range_width += (space_height *
                        (spacer + duration.augmentation_dot_count as f32 *
                            DISTANCE_BETWEEN_AUGMENTATION_DOTS)).round() as i32 +
                        duration.augmentation_dot_count as i32 *
                        character_width(device_context, font_set.full_size, 0xe1e7) +
                        character_width(device_context, font_set.full_size,
                            duration_codepoint(duration) as u32);
                },
                ObjectType::KeySig(key_sig) =>
                {
                    let spacer =
                    match &staff.objects[object_index].object_type
                    {
                        ObjectType::Accidental{..} =>
                        {
                            if let ObjectType::Accidental{..} =
                                &staff.objects[object_index + 1].object_type
                            {
                                1.0
                            }
                            else
                            {
                                1.5
                            }
                        },
                        ObjectType::Clef(_) => 2.0,
                        ObjectType::Duration{..} =>
                        {
                            if object_is_header(previous_object)
                            {
                                2.5
                            }
                            else
                            {
                                2.0
                            }
                        }
                        ObjectType::KeySig(_) => 2.0,
                        _ => 1.0
                    };
                    range_width += (space_height * spacer).round() as i32;
                    for accidental in &key_sig.accidentals
                    {
                        range_width += character_width(device_context, font_set.full_size,
                            accidental_codepoint(&accidental.accidental) as u32);
                    }
                },
                ObjectType::None => panic!("ObjectType::None found in staff interior."),
                ObjectType::TimeSig{numerator, denominator,..} =>
                {
                    let spacer = 
                    match &staff.objects[object_index].object_type
                    {
                        ObjectType::Accidental{..} => 1.0,
                        ObjectType::Barline{..} => 1.0,
                        ObjectType::None{..} => 1.0,
                        _ => 2.0
                    };
                    range_width += (space_height * spacer).round() as i32 +
                        std::cmp::max(string_width(device_context, font_set.full_size,
                            &time_sig_component_string(*numerator)),
                            string_width(device_context, font_set.full_size,
                                &time_sig_component_string(*denominator)));
                }
            };
            object_index -= 1;
            let object = &mut staff.objects[object_index];
            if let Some(slice_address) = object.slice_address
            {
                range_width -= default_object_origin_to_slice_distance(space_height, object);
                for index in project.slice_indices[slice_address] + 1..slice_index
                {
                    range_width -= project.slices[index].distance_from_previous_slice;
                }
                break;
            }
            object.distance_to_next_slice = range_width;
        }
        distance_from_previous_slice = std::cmp::max(distance_from_previous_slice, range_width);
        release_font_set(&font_set);
    }
    project.last_slice_x +=
        distance_from_previous_slice - project.slices[slice_index].distance_from_previous_slice;
    project.slices[slice_index].distance_from_previous_slice = distance_from_previous_slice;
}

fn reset_viewport_offset_x(main_window_handle: HWND, project: &mut Project, mut new_offset_x: i32)
{
    if new_offset_x > project.last_slice_x
    {
        new_offset_x = project.last_slice_x;
    }
    else
    {
        let mut client_rect;
        unsafe
        {
            client_rect = std::mem::uninitialized();
            GetClientRect(main_window_handle, &mut client_rect);
        }
        let minimum_allowed_offset = project.slices[0].distance_from_previous_slice -
            to_screen_coordinate(client_rect.right as f32,
                1.0 / zoom_factor(project.zoom_exponent));
        if new_offset_x < minimum_allowed_offset
        {
            new_offset_x = minimum_allowed_offset;
        }
    }
    let mut leftmost_visible_slice_index =
        project.slice_indices[project.leftmost_visible_slice_address];
    let mut slice = &project.slices[leftmost_visible_slice_index];
    let mut leftmost_visible_slice_x =
        project.x_of_slice_beyond_leftmost_visible + slice.distance_from_previous_slice;
    if new_offset_x < project.viewport_offset.x
    {
        while leftmost_visible_slice_index > 0
        {
            leftmost_visible_slice_x -= slice.distance_from_previous_slice;
            leftmost_visible_slice_index -= 1;
            slice = &project.slices[leftmost_visible_slice_index];
            if leftmost_visible_slice_x <= new_offset_x
            {
                break;
            }
        }
        project.x_of_slice_beyond_leftmost_visible =
            leftmost_visible_slice_x - slice.distance_from_previous_slice;
        for staff_index in 0..project.staves.len()
        {
            let staff = &project.staves[staff_index];
            if staff.address_of_clef_beyond_leftmost_visible_slice == 1
            {
                continue;
            }
            let mut clef_index =
                staff.object_indices[staff.address_of_clef_beyond_leftmost_visible_slice];
            let mut index_of_slice_object_right_of_clef = clef_index;
            let index_of_staff_slice_right_of_clef;
            loop
            {
                if let Some(address) =
                    staff.objects[index_of_slice_object_right_of_clef].slice_address
                {
                    index_of_staff_slice_right_of_clef = project.slice_indices[address];
                    break;
                }
                index_of_slice_object_right_of_clef += 1;
            }
            if index_of_staff_slice_right_of_clef <= leftmost_visible_slice_index
            {
                continue;
            }
            let mut clef_x =
                leftmost_visible_slice_x - staff.objects[clef_index].distance_to_next_slice;
            for slice_index in leftmost_visible_slice_index + 1..=index_of_staff_slice_right_of_clef
            {
                clef_x += project.slices[slice_index].distance_from_previous_slice;
            }
            if clef_x >= leftmost_visible_slice_x
            {
                continue;
            }
            loop
            {
                clef_index -= 1;
                let object = &staff.objects[clef_index];
                if let ObjectType::Clef(_) = &object.object_type
                {
                    project.staves[staff_index].address_of_clef_beyond_leftmost_visible_slice =
                        object.address;
                    break;
                }
            }
        }
    }
    else
    {
        for slice_index in leftmost_visible_slice_index + 1..project.slices.len()
        {
            let next_slice = &project.slices[slice_index];
            let next_slice_x = leftmost_visible_slice_x + next_slice.distance_from_previous_slice;
            if next_slice_x <= new_offset_x
            {
                slice = next_slice;
                project.x_of_slice_beyond_leftmost_visible = leftmost_visible_slice_x;
                leftmost_visible_slice_x = next_slice_x;
            }
            else
            {
                break;
            }
        }
        for staff_index in 0..project.staves.len()
        {
            let mut staff_slice_index = leftmost_visible_slice_index;
            let mut staff_slice_x = leftmost_visible_slice_x;
            let object_index = index_of_nearest_staff_slice_object_to_left_of_slice(project,
                staff_index, &mut staff_slice_index, &mut staff_slice_x);
            let staff = &mut project.staves[staff_index];
            for index in object_index..staff.objects.len()
            {
                let object = &staff.objects[index];
                if staff_slice_x - object.distance_to_next_slice > new_offset_x
                {
                    break;
                }
                if let ObjectType::Clef(_) = &object.object_type
                {
                    staff.address_of_clef_beyond_leftmost_visible_slice = object.address;
                }
                if let Some(slice_address) = object.slice_address
                {
                    staff_slice_x += project.slices
                        [project.slice_indices[slice_address]].distance_from_previous_slice;
                }
            }
        }
    }
    project.leftmost_visible_slice_address = slice.address;
    project.viewport_offset.x = new_offset_x;
}

fn reset_viewport_offset_y(main_window_handle: HWND, project: &mut Project, mut new_offset_y: i32)
{
    let inverse_zoom_factor = 1.0 / zoom_factor(project.zoom_exponent);
    let maximum_allowed_offset = project.bottom_staff_y -
        to_screen_coordinate(CONTROL_TABS_HEIGHT as f32, inverse_zoom_factor);
    if new_offset_y > maximum_allowed_offset
    {
        new_offset_y = maximum_allowed_offset;
    }
    else
    {
        let top_staff_y =
        if project.staves.len() > 0
        {
            project.staves[0].distance_from_staff_above
        }
        else
        {
            project.bottom_staff_y
        };
        let mut client_rect;
        unsafe
        {
            client_rect = std::mem::uninitialized();
            GetClientRect(main_window_handle, &mut client_rect);
        }
        let minimum_allowed_offset =
            top_staff_y - to_screen_coordinate(client_rect.bottom as f32, inverse_zoom_factor);
        if new_offset_y < minimum_allowed_offset
        {
            new_offset_y = minimum_allowed_offset;
        }
    }
    let mut highest_visible_staff_y = project.y_of_staff_above_highest_visible +
        project.staves[project.highest_visible_staff_index].distance_from_staff_above;
    if new_offset_y < highest_visible_staff_y
    {
        while project.highest_visible_staff_index > 0
        {
            highest_visible_staff_y -=
                project.staves[project.highest_visible_staff_index].distance_from_staff_above;
            project.highest_visible_staff_index -= 1;
            if highest_visible_staff_y <= new_offset_y
            {
                break;
            }
        }
        project.y_of_staff_above_highest_visible = highest_visible_staff_y - 
            project.staves[project.highest_visible_staff_index].distance_from_staff_above;
    }
    else
    {
        for staff_index in project.highest_visible_staff_index + 1..project.staves.len()
        {
            let next_staff_y =
                highest_visible_staff_y + project.staves[staff_index].distance_from_staff_above;
            if next_staff_y <= new_offset_y
            {
                project.y_of_staff_above_highest_visible = highest_visible_staff_y;
                project.highest_visible_staff_index = staff_index;
                highest_visible_staff_y = next_staff_y;
            }
            else
            {
                break;
            }
        }
    }
    project.viewport_offset.y = new_offset_y;
}

fn respace_slices(main_window_handle: HWND, project: &mut Project,
    slice_addresses_to_respace: &Vec<usize>)
{
    let mut slice_indices_to_respace = vec![];
    for address in slice_addresses_to_respace
    {
        slice_indices_to_respace.push(project.slice_indices[*address]);
    }
    slice_indices_to_respace.sort_unstable();
    let device_context = get_dc(main_window_handle);
    for slice_index in slice_indices_to_respace
    {
        reset_distance_from_previous_slice(device_context, project, slice_index);
    }
    release_dc(main_window_handle, device_context);
    invalidate_work_region(main_window_handle);
}

fn selected_clef(project: &Project) -> Clef
{
    let steps_of_baseline_above_staff_middle;
    let codepoint;
    if button_is_checked(project.c_clef_handle)
    {
        steps_of_baseline_above_staff_middle = 0;
        if button_is_checked(project.clef_none_handle)
        {
            codepoint = 0xe05c;
        }
        else
        {
            codepoint = 0xe05d;
        }
    }
    else if button_is_checked(project.f_clef_handle)
    {
        steps_of_baseline_above_staff_middle = 2;
        if button_is_checked(project.clef_15ma_handle)
        {
            codepoint = 0xe066;
        }
        else if button_is_checked(project.clef_8va_handle)
        {
            codepoint = 0xe065;
        }
        else if button_is_checked(project.clef_none_handle)
        {
            codepoint = 0xe062;
        }
        else if button_is_checked(project.clef_8vb_handle)
        {
            codepoint = 0xe064;
        }
        else
        {
            codepoint = 0xe063;
        }
    }
    else if button_is_checked(project.g_clef_handle)
    {
        steps_of_baseline_above_staff_middle = -2;
        if button_is_checked(project.clef_15ma_handle)
        {
            codepoint = 0xe054;
        }
        else if button_is_checked(project.clef_8va_handle)
        {
            codepoint = 0xe053;
        }
        else if button_is_checked(project.clef_none_handle)
        {
            codepoint = 0xe050;
        }
        else if button_is_checked(project.clef_8vb_handle)
        {
            codepoint = 0xe052;
        }
        else
        {
            codepoint = 0xe051;
        }
    }
    else
    {
        steps_of_baseline_above_staff_middle = 0;
        codepoint = 0xe069;
    }
    Clef{codepoint: codepoint,
        steps_of_baseline_above_staff_middle: steps_of_baseline_above_staff_middle}
}

fn selected_time_sig(project: &Project) -> ObjectType
{
    unsafe
    {
        ObjectType::TimeSig{numerator: SendMessageW(project.numerator_spin_handle, UDM_GETPOS32, 0,
            0) as u16,
            denominator: 2u32.pow(-SendMessageW(project.denominator_spin_handle, UDM_GETPOS32, 0, 0)
                as u32) as u16}
    }
}

fn set_active_cursor(address: SystemAddress, range_floor: i8, project: &mut Project)
{
    project.selection = Selection::ActiveCursor{address: address, range_floor: range_floor};
    enable_add_header_object_buttons(project, TRUE);
}

fn set_cursor_to_next_state(project: &mut Project, staff_index: usize, current_object_index: usize,
    current_range_floor: i8)
{
    let staff = &project.staves[staff_index];
    let mut new_range_floor = current_range_floor;
    let mut next_object_index = current_object_index;
    loop
    {
        match &staff.objects[next_object_index].object_type
        {
            ObjectType::Clef(clef) => new_range_floor = staff_middle_pitch(clef) - 3,
            ObjectType::Duration(duration) =>
            {
                if let Some(pitch) = &duration.pitch
                {
                    new_range_floor = clamped_subtract(pitch.pitch.steps_above_c4, 3);
                }
            },
            _ => ()
        }
        next_object_index += 1;
        if next_object_index == staff.objects.len()
        {
            set_active_cursor(SystemAddress{staff_index: staff_index,
                object_address: staff.objects[current_object_index].address},
                current_range_floor, project);
            return;
        }
        let object = &staff.objects[next_object_index];
        if object.is_valid_cursor_position
        {
            set_active_cursor(SystemAddress{staff_index: staff_index,
                object_address: object.address}, new_range_floor, project);
            return;
        }
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

fn staff_middle_pitch(clef: &Clef) -> i8
{
    let baseline_pitch =
    match clef.codepoint
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
    baseline_pitch - clef.steps_of_baseline_above_staff_middle
}

fn staff_middle_pitch_at_viewport_left_edge(staff: &Staff) -> i8
{
    if let ObjectType::Clef(clef) = &staff.objects
        [staff.object_indices[staff.address_of_clef_beyond_leftmost_visible_slice]].object_type
    {
        staff_middle_pitch(clef)
    }
    else
    {
        panic!("Clef address didn't point to clef.");
    }
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
                    DialogBoxIndirectParamW(std::ptr::null_mut(),
                        ADD_STAFF_DIALOG_TEMPLATE.data.as_ptr() as *const DLGTEMPLATE,
                        main_window_handle, Some(add_staff_dialog_proc),
                        project as *mut _ as isize);
                    return 0;
                }
            }
        },
        _ => ()
    }
    DefWindowProcW(window_handle, u_msg, w_param, l_param)
}

fn staff_vertical_bounds(staff_middle_y: i32, staff_space_height: f32, staff_line_count: u8,
    zoom_factor: f32) -> VerticalInterval
{
    let line_thickness = staff_space_height * BRAVURA_METADATA.staff_line_thickness;
    VerticalInterval{
        top: horizontal_line_vertical_bounds(
            y_of_steps_above_bottom_line(staff_middle_y,
                staff_space_height, staff_line_count, 2 * (staff_line_count as i8 - 1)),
            line_thickness, zoom_factor).top,
        bottom: horizontal_line_vertical_bounds(
            y_of_steps_above_bottom_line(staff_middle_y,
                staff_space_height, staff_line_count, 0),
            line_thickness, zoom_factor).bottom}
}

fn string_width(device_context: HDC, zoomed_font: HFONT, string: &Vec<u16>) -> i32
{
    let old_font = select_object(device_context, zoomed_font);
    let mut size: winapi::shared::windef::SIZE;
    unsafe
    {
        size = std::mem::uninitialized();
        GetTextExtentPoint32W(device_context, string.as_ptr(), string.len() as i32,
            &mut size as *mut _);
    }
    select_object(device_context, old_font);
    size.cx
}

fn time_sig_component_string(mut component: u16) -> Vec<u16>
{
    let mut place_values = vec![];
    while component != 0
    {
        place_values.push(component % 10);
        component /= 10;
    }
    let mut codepoints = vec![];
    for value in place_values.iter().rev()
    {
        match value
        {
            0 => codepoints.push(0xe080),
            1 => codepoints.push(0xe081),
            2 => codepoints.push(0xe082),
            3 => codepoints.push(0xe083),
            4 => codepoints.push(0xe084),
            5 => codepoints.push(0xe085),
            6 => codepoints.push(0xe086),
            7 => codepoints.push(0xe087),
            8 => codepoints.push(0xe088),
            9 => codepoints.push(0xe089),
            _ => panic!("Key sig place value had multiple digits.")
        }
    }
    codepoints
}

unsafe extern "system" fn time_sig_tab_proc(window_handle: HWND, u_msg: UINT, w_param: WPARAM,
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
                if l_param == project.add_time_sig_button_handle as isize
                {
                    let new_time_sig = selected_time_sig(project);
                    let staff_index;
                    let mut time_sig_index;
                    let current_range_floor;
                    let mut slice_addresses_to_respace = vec![];
                    match &project.selection
                    {
                        Selection::ActiveCursor{address, range_floor} =>
                        {
                            current_range_floor = *range_floor;
                            staff_index = address.staff_index;
                            let staff = &mut project.staves[staff_index];
                            time_sig_index = staff.object_indices[address.object_address];
                            insert_object(&mut slice_addresses_to_respace, staff, time_sig_index,
                                Object{object_type: new_time_sig, address: 0, slice_address: None,
                                    distance_to_next_slice: 0, is_selected: false,
                                    is_valid_cursor_position: true});
                        }
                        Selection::Object(address) =>
                        {
                            staff_index = address.staff_index;
                            let staff = &mut project.staves[staff_index];
                            current_range_floor =
                            if let ObjectType::Clef(clef) = &staff.objects[1].object_type
                            {
                                staff_middle_pitch(clef) - 3
                            }
                            else
                            {
                                panic!("Object at index 1 wasn't clef.")
                            };
                            time_sig_index = 2;
                            loop
                            {
                                let object = &mut staff.objects[time_sig_index];
                                if let Some(slice_address) = object.slice_address
                                {
                                    if slice_address == HEADER_TIME_SIG_SLICE_INDEX
                                    {
                                        object.object_type = new_time_sig;
                                        break;
                                    }
                                    else if slice_address < HEADER_TIME_SIG_SLICE_INDEX
                                    {
                                        time_sig_index += 1;
                                        continue;
                                    }
                                }
                                let time_sig_address = new_address(&mut staff.object_indices,
                                    &mut staff.object_address_free_list, time_sig_index);
                                insert_slice_object(project, &mut slice_addresses_to_respace,
                                    staff_index,
                                    Object{object_type: new_time_sig, address: time_sig_address,
                                        slice_address: Some(HEADER_TIME_SIG_SLICE_INDEX),
                                        distance_to_next_slice: 0, is_selected: false,
                                        is_valid_cursor_position: false},
                                    time_sig_index, HEADER_TIME_SIG_SLICE_INDEX);
                                break;
                            }
                            cancel_selection(main_window_handle);
                        },
                        Selection::None => panic!("Time sig insertion attempted without selection.")
                    }
                    set_cursor_to_next_state(project, staff_index, time_sig_index,
                        current_range_floor);
                    respace_slices(main_window_handle, project, &slice_addresses_to_respace);
                    return 0;
                }
            }
        },
        WM_NOTIFY =>
        {
            let lpmhdr = l_param as LPNMHDR;
            if (*lpmhdr).code == UDN_DELTAPOS
            {
                let project = project_memory(GetParent(GetParent(window_handle)));
                let lpnmud = l_param as LPNMUPDOWN;
                let new_position = (*lpnmud).iPos + (*lpnmud).iDelta;
                if (*lpmhdr).hwndFrom == project.denominator_spin_handle
                {
                    let new_text =                
                    if new_position > 0
                    {                           
                        wide_char_string("1")
                    }
                    else if new_position < MIN_LOG2_DURATION
                    {
                        wide_char_string("1024")                        
                    }
                    else
                    {
                        wide_char_string(&2u32.pow(-new_position as u32).to_string())
                    };
                    SendMessageW(project.denominator_display_handle, WM_SETTEXT, 0,
                        new_text.as_ptr() as isize); 
                    return 0;               
                }
            }
        },
        _ => ()
    }
    DefWindowProcW(window_handle, u_msg, w_param, l_param)
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

fn whole_notes_long(duration: &Duration) -> num_rational::Ratio<num_bigint::BigUint>
{
    let mut whole_notes_long =
    if duration.log2 >= 0
    {
        num_rational::Ratio::new(num_bigint::BigUint::from(2u32.pow(duration.log2 as u32)),
            num_bigint::BigUint::new(vec![1]))
    }
    else
    {
        num_rational::Ratio::new(num_bigint::BigUint::new(vec![1]),
            num_bigint::BigUint::from(2u32.pow(-duration.log2 as u32)))
    };
    let mut dot_whole_notes_long = whole_notes_long.clone();
    let two = num_bigint::BigUint::new(vec![2]);
    for _ in 0..duration.augmentation_dot_count
    {
        dot_whole_notes_long /= &two;
        whole_notes_long += &dot_whole_notes_long;
    }
    whole_notes_long
}

fn y_of_steps_above_bottom_line(staff_middle_y: i32, staff_space_height: f32, staff_line_count: u8,
    step_count: i8) -> f32
{
    staff_middle_y as f32 +
        (staff_line_count as i16 - 1 - step_count as i16) as f32 * staff_space_height / 2.0
}

fn zoom_factor(zoom_exponent: i8) -> f32
{
    1.1f32.powf(zoom_exponent as f32)
}