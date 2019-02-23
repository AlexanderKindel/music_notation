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
const WHOLE_NOTE_WIDTH: u32 = 90;
const DURATION_RATIO: f32 = 1.618034;
const MIN_LOG2_DURATION: i32 = -10;
const MAX_LOG2_DURATION: i32 = 1;
const TRACKBAR_MIDDLE: isize = 32767;
const DWLP_USER: i32 = (std::mem::size_of::<LRESULT>() + std::mem::size_of::<DLGPROC>()) as i32;

static mut GRAY_PEN: Option<HPEN> = None;
static mut GRAY_BRUSH: Option<HBRUSH> = None;
static mut RED_PEN: Option<HPEN> = None;
static mut RED_BRUSH: Option<HBRUSH> = None;

struct Address
{
    range_address: RangeAddress,
    object_index: Option<usize>
}

struct FontSet
{
    full_size: HFONT,
    two_thirds_size: HFONT
}

struct MainWindowMemory
{
    default_staff_space_height: f32,
    staff_scales: Vec<StaffScale>,
    slices: Vec<Slice>,
    staves: Vec<Staff>,
    system_left_edge: i32,
    ghost_cursor: Option<Address>,
    selection: Selection,
    add_staff_button_handle: HWND,
    add_clef_button_handle: HWND,
    add_key_sig_button_handle: HWND,
    duration_display_handle: HWND,
    duration_spin_handle: HWND,
    augmentation_dot_spin_handle: HWND,
    zoom_trackbar_handle: HWND
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
        baseline_offset: i8,//With respect to staff middle.
        header: bool
    },
    Duration
    {
        //Denotes the power of two times the duration of a whole note of the object's duration.
        log2_duration: i8,
        pitch: Option<i8>,//In steps above c4.
        augmentation_dot_count: u8
    },
    KeySignature
    {
        accidental_count: u8,
        flats: bool,
        header: bool
    },
    None
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
    Objects(Vec<Address>),
    None
}

struct Slice
{
    objects: Vec<RangeAddress>,
    slice_type: SliceType,
    distance_from_previous_slice: i32
}

enum SliceType
{
    Duration{rhythmic_position: num_rational::Ratio<num_bigint::BigUint>},
    HeaderClef,
    HeaderKeySig
}

struct Staff
{
    scale_index: usize,
    object_ranges: Vec<ObjectRange>,
    vertical_center: i32,
    line_count: u8
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

fn add_clef(slices: &mut Vec<Slice>, address: &Address, staves: &mut Vec<Staff>, codepoint: u16,
    baseline_offset: i8) -> usize
{
    let object = resolve_address(staves, address);
    if let ObjectType::Clef{header,..} = object.object_type
    {
        *object = Object{object_type: ObjectType::Clef{codepoint: codepoint,
            baseline_offset: baseline_offset, header: header}, is_selected: false};
        return address.range_address.range_index;
    }
    if let Some(previous_address) =
        previous_address(&staves[address.range_address.staff_index], address)
    {
        let previous_object = resolve_address(staves, &previous_address);
        if let ObjectType::Clef{header,..} = previous_object.object_type
        {
            *previous_object = Object{object_type: ObjectType::Clef{codepoint: codepoint,
                baseline_offset: baseline_offset, header: header}, is_selected: false};
            return previous_address.range_address.range_index;
        }
    }
    else
    {
        insert_object_range(slices, &mut staves[address.range_address.staff_index],
            &address.range_address, 0);    
        slices[0].objects.push(
            RangeAddress{staff_index: address.range_address.staff_index, range_index: 0});    
        staves[address.range_address.staff_index].object_ranges[0].slice_object =
            Object{object_type: ObjectType::Clef{codepoint: codepoint,
            baseline_offset: baseline_offset, header: true}, is_selected: false};
        return 0;
    }
    let other_objects = &mut staves[address.range_address.staff_index].
        object_ranges[address.range_address.range_index].other_objects;
    let object_index =
    if let Some(index) = address.object_index
    {
        index
    }
    else
    {
        other_objects.len()
    };
    other_objects.insert(object_index, RangeObject{object: Object{object_type:
        ObjectType::Clef{codepoint: codepoint, baseline_offset: baseline_offset, header: false},
        is_selected: false}, distance_to_slice_object: 0});
    address.range_address.range_index
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
                    let mut octave_id = 0;
                    for id in [IDC_ADD_CLEF_15MA, IDC_ADD_CLEF_8VA, IDC_ADD_CLEF_NONE,
                        IDC_ADD_CLEF_8VB, IDC_ADD_CLEF_15MB].iter()
                    {
                        if SendMessageW(GetDlgItem(dialog_handle, *id), BM_GETCHECK, 0, 0) ==
                            BST_CHECKED as isize
                        {
                            octave_id = *id;
                            break;
                        }
                    }
                    let baseline_offset;
                    let codepoint =
                    if SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_CLEF_G),
                        BM_GETCHECK, 0, 0) == BST_CHECKED as isize
                    {
                        baseline_offset = -2;
                        match octave_id
                        {
                            IDC_ADD_CLEF_15MA => 0xe054,
                            IDC_ADD_CLEF_8VA => 0xe053,
                            IDC_ADD_CLEF_NONE => 0xe050,
                            IDC_ADD_CLEF_8VB => 0xe052,
                            IDC_ADD_CLEF_15MB => 0xe051,
                            _ => panic!("Unknown clef octave transposition.")
                        }
                    }
                    else if SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_CLEF_C),
                        BM_GETCHECK, 0, 0) == BST_CHECKED as isize
                    {
                        baseline_offset = 0;
                        match octave_id
                        {
                            IDC_ADD_CLEF_NONE => 0xe05c,
                            IDC_ADD_CLEF_8VB => 0xe05d,
                            _ => panic!("Unknown clef octave transposition.")
                        }
                    }
                    else if SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_CLEF_F),
                        BM_GETCHECK, 0, 0) == BST_CHECKED as isize
                    {
                        baseline_offset = 2;
                        match octave_id
                        {
                            IDC_ADD_CLEF_15MA => 0xe066,
                            IDC_ADD_CLEF_8VA => 0xe065,
                            IDC_ADD_CLEF_NONE => 0xe062,
                            IDC_ADD_CLEF_8VB => 0xe064,
                            IDC_ADD_CLEF_15MB => 0xe063,
                            _ => panic!("Unknown clef octave transposition.")
                        }
                    }
                    else
                    {
                        baseline_offset = 0;
                        0xe069
                    };
                    let main_window_memory = &mut *(GetWindowLongPtrW(dialog_handle, DWLP_USER)
                        as *mut MainWindowMemory);
                    let address =
                    if let Selection::ActiveCursor{address,..} =
                        &main_window_memory.selection
                    {
                        address
                    }
                    else
                    {
                        panic!("Clef insertion attempted without active cursor.");
                    };
                    let insertion_range_index = add_clef(&mut main_window_memory.slices, address,
                        &mut main_window_memory.staves, codepoint, baseline_offset);
                    space_new_object(main_window_memory, GetWindow(dialog_handle, GW_OWNER),
                        address.range_address.staff_index, insertion_range_index);
                    EndDialog(dialog_handle, 0);
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
            size_dialog(dialog_handle);
            SetWindowLongPtrW(dialog_handle, DWLP_USER, l_param);
            SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_CLEF_G), BM_SETCHECK, BST_CHECKED, 0);
            SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_CLEF_NONE), BM_SETCHECK, BST_CHECKED, 0);
            TRUE as isize
        },
        _ => FALSE as isize
    }
}

fn add_key_sig(slices: &mut Vec<Slice>, address: &Address, staves: &mut Vec<Staff>,
    accidental_count: u8, flats: bool) -> usize
{
    let object = resolve_address(staves, address);
    if let ObjectType::KeySignature{header,..} = object.object_type
    {
        *object = Object{object_type: ObjectType::KeySignature{accidental_count: accidental_count,
            flats: flats, header: header}, is_selected: false};
        return address.range_address.range_index;
    }
    if let Some(previous_address) =
        previous_address(&staves[address.range_address.staff_index], address)
    {
        let previous_object = resolve_address(staves, &previous_address);
        if let ObjectType::KeySignature{header,..} = previous_object.object_type
        {
            *previous_object = Object{object_type: ObjectType::KeySignature{accidental_count:
                accidental_count, flats: flats, header: header}, is_selected: false};
            return previous_address.range_address.range_index;
        }
    }
    else
    {
        if staves[address.range_address.staff_index].object_ranges.len() < 2 ||
            staves[address.range_address.staff_index].object_ranges[1].slice_index != 1
        {
            insert_object_range(slices, &mut staves[address.range_address.staff_index],
                &address.range_address, 1);
            slices[0].objects.push(
                RangeAddress{staff_index: address.range_address.staff_index, range_index: 1});
        }
        staves[address.range_address.staff_index].object_ranges[0].slice_object =
            Object{object_type: ObjectType::KeySignature{accidental_count: accidental_count,
            flats: flats, header: true}, is_selected: false};
        return 1;
    }
    let other_objects = &mut staves[address.range_address.staff_index].
        object_ranges[address.range_address.range_index].other_objects;
    let object_index =
    if let Some(index) = address.object_index
    {
        index
    }
    else
    {
        other_objects.len()
    };
    other_objects.insert(object_index, RangeObject{object: Object{object_type:
        ObjectType::KeySignature{accidental_count: accidental_count, flats: flats, header: false},
        is_selected: false}, distance_to_slice_object: 0});
    address.range_address.range_index
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
                    let main_window_memory = &mut *(GetWindowLongPtrW(dialog_handle, DWLP_USER)
                        as *mut MainWindowMemory);
                    let address =
                    if let Selection::ActiveCursor{address,..} =
                        &main_window_memory.selection
                    {
                        address
                    }
                    else
                    {
                        panic!("Key signature insertion attempted without active cursor.");
                    };
                    let insertion_range_index = add_key_sig(&mut main_window_memory.slices, address,
                        &mut main_window_memory.staves, SendMessageW(GetDlgItem(dialog_handle,
                        IDC_ADD_KEY_SIG_ACCIDENTAL_COUNT), UDM_GETPOS32, 0, 0) as u8,
                        SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_KEY_SIG_FLATS),
                        BM_GETCHECK, 0, 0) == BST_CHECKED as isize);
                    space_new_object(main_window_memory, GetWindow(dialog_handle, GW_OWNER),
                        address.range_address.staff_index, insertion_range_index);
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
            SendMessageW(accidental_count_spin_handle, UDM_SETRANGE32, 1, 7);
            SendMessageW(accidental_count_spin_handle, UDM_SETPOS32, 0, 1);
            SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_KEY_SIG_SHARPS),
                BM_SETCHECK, BST_CHECKED, 0);
            TRUE as isize
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
                        as *mut MainWindowMemory)).staff_scales;
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
                    let main_window_memory = &mut *(GetWindowLongPtrW(dialog_handle, DWLP_USER)
                        as *mut MainWindowMemory);
                    let scale_index = SendMessageW(GetDlgItem(dialog_handle,
                        IDC_ADD_STAFF_SCALE_LIST), CB_GETCURSEL, 0, 0) as usize;
                    DialogBoxIndirectParamW(null_mut(),
                        EDIT_STAFF_SCALE_DIALOG_TEMPLATE.data.as_ptr() as *mut DLGTEMPLATE,
                        dialog_handle, Some(edit_staff_scale_dialog_proc),
                        &mut main_window_memory.staff_scales[scale_index] as *mut _ as isize);
                    let edited_scale = main_window_memory.staff_scales.remove(scale_index);
                    let edited_scale_index =
                        insert_staff_scale(&mut main_window_memory.staff_scales, edited_scale);
                    let scale_list_handle = GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST);
                    SendMessageW(scale_list_handle, CB_DELETESTRING, scale_index, 0);
                    SendMessageW(scale_list_handle, CB_INSERTSTRING, edited_scale_index,
                        to_string(&main_window_memory.staff_scales[edited_scale_index]).
                        as_ptr() as isize);
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
                    for staff in &mut main_window_memory.staves
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
                    let main_window_memory = &mut *(GetWindowLongPtrW(dialog_handle, DWLP_USER)
                        as *mut MainWindowMemory);
                    let mut scale_is_used = false;
                    for staff_index in 0..main_window_memory.staves.len()
                    {
                        if main_window_memory.staves[staff_index].scale_index == removal_index
                        {
                            scale_is_used = true;
                            break;
                        }
                    }
                    let remapped_index;
                    if scale_is_used
                    {
                        let mut reassignment_candidates = vec![]; 
                        for scale_index in 0..main_window_memory.staff_scales.len()
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
                    main_window_memory.staff_scales.remove(removal_index);
                    for staff in &mut main_window_memory.staves
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
                    let main_window_memory = &mut *(GetWindowLongPtrW(dialog_handle, DWLP_USER)
                        as *mut MainWindowMemory);
                    let vertical_center = 
                    if main_window_memory.staves.len() == 0
                    {
                        110
                    }
                    else
                    {
                        main_window_memory.staves[main_window_memory.staves.len() - 1].
                            vertical_center + 80
                    };
                    let scale_index = SendMessageW(GetDlgItem(dialog_handle,
                        IDC_ADD_STAFF_SCALE_LIST), CB_GETCURSEL, 0, 0) as usize;
                    let staff_index = main_window_memory.staves.len();
                    main_window_memory.staves.push(Staff{scale_index: scale_index,
                        object_ranges: vec![], vertical_center: vertical_center,
                        line_count: SendMessageW(GetDlgItem(dialog_handle,
                        IDC_ADD_STAFF_LINE_COUNT_SPIN), UDM_GETPOS32, 0, 0) as u8});
                    register_rhythmic_position(&mut main_window_memory.slices,
                        &mut main_window_memory.staves, &mut 0,
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
            let staff_scales = &(*(GetWindowLongPtrW(dialog_handle, DWLP_USER)
                as *mut MainWindowMemory)).staff_scales;
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
    slices: &Vec<Slice>, staves: &mut Vec<Staff>, staff_space_height: f32, system_left_edge: i32,
    staff_index: usize, click_x: i32, click_y: i32, zoom_factor: f32) -> Option<Address>
{
    let mut x = system_left_edge;
    if click_x < to_screen_coordinate(x as f32, zoom_factor)
    {
        return None;
    }
    let staff = &staves[staff_index];
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
                return None;
            }
            draw(buffer_device_context, &zoomed_font_set, staff, staff_space_height,
                &range_object.object, object_x, &mut staff_middle_pitch, zoom_factor);
            unsafe
            {
                if GetPixel(buffer_device_context, click_x, click_y) == WHITE
                {
                    cancel_selection(window_handle);
                    staves[staff_index].object_ranges[range_index].
                        other_objects[object_index].object.is_selected = true;
                    return Some(Address{range_address: RangeAddress{staff_index: staff_index,
                        range_index: range_index}, object_index: Some(object_index)});
                }
            }
        }
        if click_x < to_screen_coordinate(x as f32, zoom_factor)
        {
            return None;
        }
        draw(buffer_device_context, &zoomed_font_set, staff, staff_space_height,
            &staff.object_ranges[range_index].slice_object, x, &mut staff_middle_pitch,
            zoom_factor);
        unsafe
        {
            if GetPixel(buffer_device_context, click_x, click_y) == WHITE
            {
                cancel_selection(window_handle);
                staves[staff_index].object_ranges[range_index].slice_object.is_selected = true;
                return Some(Address{range_address: RangeAddress{staff_index: staff_index,
                    range_index: range_index}, object_index: None});
            }
        }
    }
    None
}

fn bottom_line_pitch(staff_line_count: u8, middle_pitch: i8) -> i8
{
    middle_pitch - staff_line_count as i8 + 1
}

fn cancel_selection(window_handle: HWND)
{
    let window_memory = main_window_memory(window_handle);
    match &window_memory.selection
    {
        Selection::ActiveCursor{..} =>
        {
            invalidate_work_region(window_handle);
            unsafe
            {
                EnableWindow(window_memory.add_clef_button_handle, FALSE);
            }
        }
        Selection::Objects(addresses) =>
        {
            for address in addresses
            {
                resolve_address(&mut window_memory.staves, address).is_selected = false;
            }
            invalidate_work_region(window_handle);
            unsafe
            {
                EnableWindow(window_memory.add_clef_button_handle, FALSE);
            }
        },
        Selection::None => ()
    }        
    window_memory.selection = Selection::None;
}

fn character_width(device_context: HDC, font: HFONT, codepoint: u32) -> i32
{
    unsafe
    {
        SelectObject(device_context, font as *mut winapi::ctypes::c_void);
        let mut abc_array: [ABC; 1] = [ABC{abcA: 0, abcB: 0, abcC: 0}];
        GetCharABCWidthsW(device_context, codepoint, codepoint + 1, abc_array.as_mut_ptr());
        abc_array[0].abcB as i32
    }
}

fn clef_baseline(staff: &Staff, staff_space_height: f32,
    steps_of_clef_baseline_above_middle: i8) -> f32
{
    y_of_steps_above_bottom_line(staff, staff_space_height,
        staff.line_count as i8 - 1 + steps_of_clef_baseline_above_middle)
}

fn cursor_x(slices: &Vec<Slice>, staves: &Vec<Staff>, system_left_edge: i32,
    address: &Address) -> i32
{
    let mut x = system_left_edge;
    let staff = &staves[address.range_address.staff_index];
    for slice_index in 0..=staff.object_ranges[address.range_address.range_index].slice_index
    {
        x += slices[slice_index].distance_from_previous_slice;
    }
    if let Some(object_index) = address.object_index
    {
        x -= staff.object_ranges[address.range_address.range_index].
            other_objects[object_index].distance_to_slice_object;
    }
    x
}

fn decrement(index: &mut usize)
{
    *index -= 1;
}

fn draw(device_context: HDC, zoomed_font_set: &FontSet, staff: &Staff,
    staff_space_height: f32, object: &Object, x: i32, staff_middle_pitch: &mut i8, zoom_factor: f32)
{
    unsafe
    {
        SelectObject(device_context, zoomed_font_set.full_size as *mut winapi::ctypes::c_void);
    }
    match object.object_type
    {
        ObjectType::Clef{codepoint, baseline_offset, header} =>
        {
            if !header
            {
                unsafe
                {
                    SelectObject(device_context,
                        zoomed_font_set.two_thirds_size as *mut winapi::ctypes::c_void);
                    draw_clef(device_context, staff, staff_space_height, codepoint, baseline_offset,
                        x, staff_middle_pitch, zoom_factor);
                    SelectObject(device_context,
                        zoomed_font_set.full_size as *mut winapi::ctypes::c_void);
                }
            }
            else
            {
                draw_clef(device_context, staff, staff_space_height, codepoint, baseline_offset, x,
                    staff_middle_pitch, zoom_factor);
            }
        },
        ObjectType::Duration{log2_duration, pitch, augmentation_dot_count} =>
        {
            let duration_codepoint;
            let mut duration_left_edge = x;
            let duration_right_edge;
            let duration_y;
            let augmentation_dot_y;
            let unzoomed_font = staff_font(staff_space_height, 1.0);
            if let Some(pitch) = pitch
            {        
                let steps_above_bottom_line =
                    pitch - bottom_line_pitch(staff.line_count, *staff_middle_pitch);
                duration_y = y_of_steps_above_bottom_line(staff, staff_space_height,
                    steps_above_bottom_line);
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
                if log2_duration == 1
                {
                    duration_codepoint = 0xe0a0;
                    duration_left_edge -= (staff_space_height *
                        BRAVURA_METADATA.double_whole_notehead_x_offset).round() as i32;
                }
                else if log2_duration == 0
                {
                    duration_codepoint = 0xe0a2;
                }
                else
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
                        if log2_duration == -1
                        {
                            duration_codepoint = 0xe0a3;
                            stem_right_edge = x as f32 +
                                staff_space_height * BRAVURA_METADATA.half_notehead_stem_up_se.x;
                            stem_left_edge = stem_right_edge -
                                staff_space_height * BRAVURA_METADATA.stem_thickness;
                            stem_bottom = duration_y as f32 -
                                staff_space_height * BRAVURA_METADATA.half_notehead_stem_up_se.y;                        
                        }
                        else
                        {
                            duration_codepoint = 0xe0a4;
                            stem_right_edge = x as f32 +
                                staff_space_height * BRAVURA_METADATA.black_notehead_stem_up_se.x;
                            stem_left_edge = stem_right_edge -
                                staff_space_height * BRAVURA_METADATA.stem_thickness;
                            stem_bottom = duration_y as f32 -
                                staff_space_height * BRAVURA_METADATA.black_notehead_stem_up_se.y;
                            if log2_duration == -3
                            {
                                draw_character(device_context, 0xe240, stem_left_edge, stem_top,
                                    zoom_factor);
                            }
                            else if log2_duration < -3
                            {
                                draw_character(device_context, 0xe242, stem_left_edge, stem_top,
                                    zoom_factor);
                                let flag_spacing = staff_space_height *
                                    (BRAVURA_METADATA.beam_spacing +
                                    BRAVURA_METADATA.beam_thickness);
                                for _ in 0..-log2_duration - 4
                                {
                                    stem_top -= flag_spacing;
                                    draw_character(device_context, 0xe250, stem_left_edge, stem_top,
                                        zoom_factor);
                                }
                            }
                        }
                    }
                    else
                    {
                        stem_bottom = y_of_steps_above_bottom_line(staff, staff_space_height,
                            std::cmp::min(steps_above_bottom_line - 7, space_count));
                        if log2_duration == -1
                        {
                            duration_codepoint = 0xe0a3;
                            stem_left_edge = x as f32 +
                                staff_space_height * BRAVURA_METADATA.half_notehead_stem_down_nw.x;
                            stem_top = duration_y as f32 -
                                staff_space_height * BRAVURA_METADATA.half_notehead_stem_down_nw.y;
                        }
                        else
                        {
                            duration_codepoint = 0xe0a4;
                            stem_left_edge = x as f32 +
                                staff_space_height * BRAVURA_METADATA.black_notehead_stem_down_nw.x;
                            stem_top = duration_y as f32 -
                                staff_space_height * BRAVURA_METADATA.black_notehead_stem_down_nw.y;
                            if log2_duration == -3
                            {
                                draw_character(device_context, 0xe241, stem_left_edge, stem_bottom,
                                    zoom_factor);
                            }
                            else if log2_duration < -3
                            {
                                draw_character(device_context, 0xe243, stem_left_edge, stem_bottom,
                                    zoom_factor);
                                let flag_spacing = staff_space_height * 
                                    (BRAVURA_METADATA.beam_spacing +
                                    BRAVURA_METADATA.beam_thickness);
                                for _ in 0..-log2_duration - 4
                                {      
                                    stem_bottom += flag_spacing;
                                    draw_character(device_context, 0xe251, stem_left_edge,
                                        stem_bottom, zoom_factor);
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
                if log2_duration == 0
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
                duration_codepoint = rest_codepoint(log2_duration);  
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
            draw_character(device_context, duration_codepoint, duration_left_edge as f32,
                duration_y, zoom_factor);        
            for _ in 0..augmentation_dot_count
            {
                draw_character(device_context, 0xe1e7, next_dot_left_edge as f32,
                    augmentation_dot_y, zoom_factor);
                next_dot_left_edge += dot_offset;
            }
        },
        ObjectType::KeySignature{accidental_count, flats,..} =>
        {
            let codepoint;
            let stride;
            let mut steps_of_accidental_above_floor;
            let steps_of_floor_above_middle;
            if flats
            {         
                codepoint = 0xe260;   
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
            }
            else
            {
                codepoint = 0xe262;
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
            let steps_of_floor_above_bottom_line =
                steps_of_floor_above_middle + staff.line_count as i8 - 1;
            let accidental_width =
                character_width(device_context, staff_font(staff_space_height, 1.0), codepoint);
            let mut x = x;
            for _ in 0..accidental_count
            {
                draw_character(device_context, codepoint as u16, x as f32,
                    y_of_steps_above_bottom_line(staff, staff_space_height,
                    steps_of_accidental_above_floor + steps_of_floor_above_bottom_line),
                    zoom_factor);
                steps_of_accidental_above_floor = (steps_of_accidental_above_floor + stride) % 7;
                x += accidental_width;
            }
        },
        ObjectType::None => ()
    }
}

fn draw_character(device_context: HDC, codepoint: u16, x: f32, y: f32,
    zoom_factor: f32)
{
    unsafe
    {
        TextOutW(device_context, to_screen_coordinate(x, zoom_factor),
            to_screen_coordinate(y, zoom_factor), vec![codepoint, 0].as_ptr(), 1);
    }
}

fn draw_clef(device_context: HDC, staff: &Staff, staff_space_height: f32, codepoint: u16,
    steps_of_baseline_above_middle: i8, x: i32, staff_middle_pitch: &mut i8, zoom_factor: f32)
{
    *staff_middle_pitch = self::staff_middle_pitch(codepoint, steps_of_baseline_above_middle);
    draw_character(device_context, codepoint, x as f32,
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
    staff_space_height: f32, object: &Object, x: i32, staff_middle_pitch: &mut i8, zoom_factor: f32)
{
    if object.is_selected
    {
        unsafe
        {
            SetTextColor(device_context, RED);
            draw(device_context, zoomed_font_set, staff, staff_space_height, object, x,
                staff_middle_pitch, zoom_factor);
            SetTextColor(device_context, BLACK);
        }
    }
    else
    {
        draw(device_context, zoomed_font_set, staff, staff_space_height, object, x,
            staff_middle_pitch, zoom_factor);
    }
}

fn duration_codepoint(log2_duration: i8, pitch: Option<i8>) -> u16
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
            rest_codepoint(log2_duration)
        }
    }
}

fn duration_width(log2_duration: i8, augmentation_dot_count: u8) -> i32
{
    if augmentation_dot_count == 0
    {
        return (WHOLE_NOTE_WIDTH as f32 *
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
    (WHOLE_NOTE_WIDTH as f32 * DURATION_RATIO.powf(duration_float.log2())).round() as i32
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

unsafe fn init() -> (HWND, MainWindowMemory)
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
        dwICC: ICC_BAR_CLASSES | ICC_STANDARD_CLASSES | ICC_UPDOWN_CLASS};
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
    let device_context = GetDC(main_window_handle);
    SetBkMode(device_context, TRANSPARENT as i32);
    SetTextAlign(device_context, TA_BASELINE);   
    let mut metrics: NONCLIENTMETRICSA = std::mem::uninitialized();
    metrics.cbSize = std::mem::size_of::<NONCLIENTMETRICSA>() as u32;
    SystemParametersInfoA(SPI_GETNONCLIENTMETRICS, metrics.cbSize,
        &mut metrics as *mut _ as *mut winapi::ctypes::c_void, 0);
    let text_font = CreateFontIndirectA(&metrics.lfMessageFont as *const _);
    let add_staff_button_handle = CreateWindowExW(0, button_string.as_ptr(),
        wide_char_string("Add staff").as_ptr(), WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON | BS_VCENTER,
        0, 0, 55, 20, main_window_handle, null_mut(), instance, null_mut());
    if add_staff_button_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create add staff button; error code {}", GetLastError());
    } 
    SendMessageW(add_staff_button_handle, WM_SETFONT, text_font as usize, 0);
    let add_clef_button_handle = CreateWindowExW(0, button_string.as_ptr(),
        wide_char_string("Add clef").as_ptr(), BS_PUSHBUTTON | WS_DISABLED | WS_CHILD | WS_VISIBLE |
        BS_VCENTER, 55, 0, 55, 20, main_window_handle, null_mut(), instance, null_mut());
    if add_clef_button_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create add clef button; error code {}", GetLastError());
    }
    SendMessageW(add_clef_button_handle, WM_SETFONT, text_font as usize, 0);
    let add_key_sig_button_handle = CreateWindowExW(0, button_string.as_ptr(),
        wide_char_string("Add key signature").as_ptr(), BS_PUSHBUTTON | WS_DISABLED | WS_CHILD |
        WS_VISIBLE | BS_VCENTER, 110, 0, 105, 20, main_window_handle, null_mut(), instance,
        null_mut());
    if add_key_sig_button_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create add key signature button; error code {}", GetLastError());
    }
    SendMessageW(add_key_sig_button_handle, WM_SETFONT, text_font as usize, 0);
    let duration_label_handle = CreateWindowExW(0, static_string.as_ptr(),
        wide_char_string("Selected duration:").as_ptr(), SS_CENTER | WS_CHILD | WS_VISIBLE, 215, 0,
        110, 20, main_window_handle, null_mut(), instance, null_mut());
    if duration_label_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create selected duration label; error code {}", GetLastError());
    }
    SendMessageW(duration_label_handle, WM_SETFONT, text_font as usize, 0);
    let duration_display_handle = CreateWindowExW(0, static_string.as_ptr(),
        wide_char_string("quarter").as_ptr(), WS_BORDER | WS_CHILD | WS_VISIBLE, 215, 20, 110, 20,
        main_window_handle, null_mut(), instance, null_mut());
    if duration_display_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create selected duration display; error code {}", GetLastError());
    }
    SendMessageW(duration_display_handle, WM_SETFONT, text_font as usize, 0);
    let duration_spin_handle = CreateWindowExW(0, wide_char_string(UPDOWN_CLASS).as_ptr(),
        null_mut(), UDS_ALIGNRIGHT | UDS_AUTOBUDDY | WS_CHILD | WS_VISIBLE, 0, 0, 0, 0,
        main_window_handle, null_mut(), instance, null_mut());
    if duration_spin_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create selected duration spin; error code {}", GetLastError());
    }
    SendMessageW(duration_spin_handle, UDM_SETRANGE32, MIN_LOG2_DURATION as usize,
        MAX_LOG2_DURATION as isize);
    SendMessageW(duration_spin_handle, UDM_SETPOS32, 0, -2);
    let augmentation_dot_label_handle = CreateWindowExW(0, static_string.as_ptr(),
        wide_char_string("Augmentation dots:").as_ptr(), SS_CENTER | WS_VISIBLE | WS_CHILD, 325, 0,
        110, 20, main_window_handle, null_mut(), instance, null_mut());
    if augmentation_dot_label_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create augmentation dot label; error code {}", GetLastError());
    }
    SendMessageW(augmentation_dot_label_handle, WM_SETFONT, text_font as usize, 0);
    let augmentation_dot_display_handle =  CreateWindowExW(0, static_string.as_ptr(),
        wide_char_string("0").as_ptr(), WS_BORDER | WS_VISIBLE | WS_CHILD, 325, 20, 110, 20,
        main_window_handle, null_mut(), instance, null_mut());
    if augmentation_dot_display_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create augmentation dot display; error code {}", GetLastError());
    }
    SendMessageW(augmentation_dot_display_handle, WM_SETFONT, text_font as usize, 0);
    let augmentation_dot_spin_handle = CreateWindowExW(0, wide_char_string(UPDOWN_CLASS).as_ptr(),
        null_mut(), UDS_ALIGNRIGHT | UDS_AUTOBUDDY | UDS_SETBUDDYINT | WS_CHILD | WS_VISIBLE, 0, 0,
        0, 0, main_window_handle, null_mut(), instance, null_mut());
    if augmentation_dot_spin_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create augmentation dot spin; error code {}", GetLastError());
    } 
    SendMessageW(augmentation_dot_spin_handle, UDM_SETRANGE32, 0,
        (-2 - MIN_LOG2_DURATION) as isize);
    let mut client_rect = RECT{bottom: 0, left: 0, right: 0, top: 0};
    GetClientRect(main_window_handle, &mut client_rect);
    let zoom_trackbar_handle = CreateWindowExW(0, wide_char_string(TRACKBAR_CLASS).as_ptr(),
        null_mut(), WS_CHILD | WS_VISIBLE, 0, 0, 0, 0, main_window_handle, null_mut(), instance,
        null_mut());
    if zoom_trackbar_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create zoom trackbar; error code {}", GetLastError());
    }
    position_zoom_trackbar(main_window_handle, zoom_trackbar_handle);
    SendMessageW(zoom_trackbar_handle, TBM_SETRANGEMIN, 0, 0);
    SendMessageW(zoom_trackbar_handle, TBM_SETRANGEMAX, 0, 2 * TRACKBAR_MIDDLE);
    SendMessageW(zoom_trackbar_handle, TBM_SETTIC, 0, TRACKBAR_MIDDLE);
    SendMessageW(zoom_trackbar_handle, TBM_SETPOS, 1, TRACKBAR_MIDDLE);
    let main_window_memory = MainWindowMemory{default_staff_space_height: 10.0,
        staff_scales: vec![StaffScale{name: unterminated_wide_char_string("Default"), value: 1.0},
        StaffScale{name: unterminated_wide_char_string("Cue"), value: 0.75}],
        slices: vec![Slice{objects: vec![], slice_type: SliceType::HeaderClef,
        distance_from_previous_slice: 0}, Slice{objects: vec![],
        slice_type: SliceType::HeaderKeySig, distance_from_previous_slice: 0}], staves: vec![],
        system_left_edge: 20, ghost_cursor: None, selection: Selection::None,
        add_staff_button_handle: add_staff_button_handle,
        add_clef_button_handle: add_clef_button_handle,
        add_key_sig_button_handle: add_key_sig_button_handle,
        duration_display_handle: duration_display_handle,
        duration_spin_handle: duration_spin_handle,
        augmentation_dot_spin_handle: augmentation_dot_spin_handle,
        zoom_trackbar_handle: zoom_trackbar_handle};        
    (main_window_handle, main_window_memory)
}

fn insert_duration(device_context: HDC, slices: &mut Vec<Slice>, staves: &mut Vec<Staff>,
    staff_space_heights: &Vec<f32>, log2_duration: i8, pitch: Option<i8>,
    augmentation_dot_count: u8, insertion_address: &Address) -> Address
{
    let mut range_index = insertion_address.range_address.range_index;
    if let Some(object_index) = insertion_address.object_index
    {
        staves[insertion_address.range_address.staff_index].object_ranges[range_index].
            other_objects.split_off(object_index);
    }    
    let mut rest_rhythmic_position;
    let mut slice_index;
    loop
    {       
        let staff = &mut staves[insertion_address.range_address.staff_index];
        slice_index = staff.object_ranges[range_index].slice_index;        
        if let SliceType::Duration{rhythmic_position} = &slices[slice_index].slice_type
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
        if range_index == staves[insertion_address.range_address.staff_index].object_ranges.len()
        {
            register_rhythmic_position(slices, staves, &mut slice_index, rest_rhythmic_position,
                insertion_address.range_address.staff_index, range_index);
            reset_distance_from_previous_slice(device_context, slices, staves, staff_space_heights,
                slice_index);
            slice_index += 1;
            if slice_index < slices.len()
            {
                reset_distance_from_previous_slice(device_context, slices, staves,
                    staff_space_heights, slice_index);
            }
            return Address{range_address: RangeAddress{staff_index:
                insertion_address.range_address.staff_index, range_index: range_index},
                object_index: None};
        }
        let slice_index = staves[insertion_address.range_address.staff_index].
            object_ranges[range_index].slice_index;
        if let SliceType::Duration{rhythmic_position} = &slices[slice_index].slice_type
        {
            if *rhythmic_position < rest_rhythmic_position
            {
                remove_object_range(staves, slices, insertion_address.range_address.staff_index,
                    range_index, slice_index);
            }
            else
            {
                rest_duration = rhythmic_position - &rest_rhythmic_position;
                break;
            }
        }
        else
        {
            remove_object_range(staves, slices, insertion_address.range_address.staff_index,
                range_index, slice_index);
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
                insertion_address.range_address.staff_index, range_index);
            staves[insertion_address.range_address.staff_index].object_ranges[range_index].
                slice_object.object_type = ObjectType::Duration{log2_duration: log2_duration,
                pitch: None, augmentation_dot_count: augmentation_dot_count};
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
    Address{range_address: RangeAddress{staff_index: insertion_address.range_address.staff_index,
        range_index: range_index}, object_index: None}
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
        client_rect.top = 40;
        InvalidateRect(window_handle, &client_rect, TRUE);
    }
}

fn left_edge_to_origin_distance(staff_space_height: f32, log2_duration: i8) -> i32
{
    if log2_duration == 1
    {
        return (staff_space_height *
            BRAVURA_METADATA.double_whole_notehead_x_offset).round() as i32;
    }
    0
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

fn main_window_memory<'a>(window_handle: HWND) -> &'a mut MainWindowMemory
{
    unsafe
    {
        &mut *(GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory)
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
                let window_memory = main_window_memory(window_handle);
                if l_param == window_memory.add_clef_button_handle as isize
                {
                    DialogBoxIndirectParamW(null_mut(),
                        ADD_CLEF_DIALOG_TEMPLATE.data.as_ptr() as *const DLGTEMPLATE, window_handle,
                        Some(add_clef_dialog_proc), window_memory as *mut _ as isize);
                }
                else if l_param == window_memory.add_key_sig_button_handle as isize
                {
                    DialogBoxIndirectParamW(null_mut(), ADD_KEY_SIG_DIALOG_TEMPLATE.data.as_ptr()
                        as *const DLGTEMPLATE, window_handle, Some(add_key_sig_dialog_proc),
                        window_memory as *mut _ as isize);
                }
                else if l_param == window_memory.add_staff_button_handle as isize
                {
                    if DialogBoxIndirectParamW(null_mut(), ADD_STAFF_DIALOG_TEMPLATE.data.as_ptr()
                        as *const DLGTEMPLATE, window_handle, Some(add_staff_dialog_proc),
                        window_memory as *mut _ as isize) == 0
                    {
                        return 0;
                    }
                    let space_heights = &staff_space_heights(&window_memory.staves,
                        &window_memory.staff_scales, window_memory.default_staff_space_height);
                    reset_distance_from_previous_slice(GetDC(window_handle),
                        &mut window_memory.slices, &mut window_memory.staves, &space_heights, 2);
                    invalidate_work_region(window_handle);
                    return 0;
                }
            }
            DefWindowProcW(window_handle, u_msg, w_param, l_param)
        },  
        WM_CTLCOLORSTATIC =>
        {
            GetStockObject(WHITE_BRUSH as i32) as isize
        },
        WM_HSCROLL =>
        {
            SetFocus(window_handle);
            invalidate_work_region(window_handle);
            0        
        },
        WM_KEYDOWN =>
        {
            match w_param as i32
            {
                65..=71 =>
                {
                    let window_memory = main_window_memory(window_handle);
                    if let Selection::ActiveCursor{ref address, range_floor} =
                        (*window_memory).selection
                    {
                        let device_context = GetDC(window_handle);
                        let octave4_pitch = (w_param as i8 - 60) % 7;
                        let mut octave4_cursor_range_floor = range_floor % 7;
                        let mut octaves_of_range_floor_above_octave4 = range_floor / 7;
                        if octave4_cursor_range_floor < 0
                        {
                            octave4_cursor_range_floor += 7;
                            octaves_of_range_floor_above_octave4 -= 1;
                        }
                        let mut pitch = 7 * octaves_of_range_floor_above_octave4 + octave4_pitch;
                        if octave4_cursor_range_floor > octave4_pitch
                        {
                            pitch += 7;
                        }
                        let space_heights = staff_space_heights(&window_memory.staves,
                            &window_memory.staff_scales, window_memory.default_staff_space_height);
                        let next_duration_address = insert_duration(device_context,
                            &mut window_memory.slices, &mut window_memory.staves, &space_heights,
                            SendMessageW(window_memory.duration_spin_handle,
                            UDM_GETPOS32, 0, 0) as i8, Some(pitch),
                            SendMessageW(window_memory.augmentation_dot_spin_handle, UDM_GETPOS32,
                            0, 0) as u8, address.clone());
                        (*window_memory).selection = Selection::ActiveCursor{
                            address: next_duration_address, range_floor: pitch - 3};
                        invalidate_work_region(window_handle);
                    }
                    0
                },
                VK_DOWN =>
                {
                    let window_memory = main_window_memory(window_handle);
                    match &mut window_memory.selection
                    {
                        Selection::ActiveCursor{range_floor,..} =>
                        {
                            if *range_floor < i8::min_value() + 7
                            {
                                *range_floor = i8::min_value();
                            }
                            else
                            {
                                *range_floor -= 7;
                            }
                        },
                        Selection::Objects(addresses) =>
                        {
                            for address in addresses
                            {
                                let staff_line_count = window_memory.
                                    staves[address.range_address.staff_index].line_count as i8;
                                match resolve_address(&mut window_memory.staves, address).
                                    object_type
                                {
                                    ObjectType::Clef{ref mut baseline_offset,..} =>
                                    {
                                        let new_baseline = *baseline_offset - 1;
                                        if new_baseline > -staff_line_count
                                        {
                                            *baseline_offset = new_baseline;
                                        }
                                    },
                                    ObjectType::Duration{ref mut pitch,..} =>
                                    {
                                        if let Some(pitch) = pitch
                                        {
                                            if *pitch > i8::min_value()
                                            {
                                                *pitch -= 1;
                                            }
                                        }
                                    },
                                    _ => ()
                                }
                            }
                        },
                        Selection::None => return 0
                    }
                    invalidate_work_region(window_handle);
                    0
                },
                VK_ESCAPE =>
                {
                    cancel_selection(window_handle);
                    0
                },
                VK_LEFT =>
                {
                    let window_memory = main_window_memory(window_handle);
                    if let Selection::ActiveCursor{address, range_floor} =
                        &mut window_memory.selection
                    {
                        let staff = &window_memory.staves[address.range_address.staff_index];
                        if let Some(previous_address) = previous_address(staff, &address)
                        {
                            if let ObjectType::Duration{pitch,..} = resolve_address(
                                &mut window_memory.staves, &previous_address).object_type
                            {
                                if let Some(pitch) = pitch
                                {
                                    *range_floor = pitch - 3;
                                }
                            }
                            *address = previous_address;
                            invalidate_work_region(window_handle);
                        }
                    }                   
                    0
                },
                VK_RIGHT =>
                {
                    let window_memory = main_window_memory(window_handle);
                    if let Selection::ActiveCursor{address, range_floor} =
                        &mut window_memory.selection
                    {
                        let staff = &window_memory.staves[address.range_address.staff_index];
                        if let Some(next_address) = next_address(staff, &address)
                        {
                            if let ObjectType::Duration{pitch,..} = resolve_address(
                                &mut window_memory.staves, &next_address).object_type
                            {
                                if let Some(pitch) = pitch
                                {
                                    *range_floor = pitch - 3;
                                }
                            }
                            *address = next_address;
                            invalidate_work_region(window_handle);
                        }
                    }
                    0
                },
                VK_SPACE =>
                {
                    let window_memory = main_window_memory(window_handle);
                    if let Selection::ActiveCursor{ref address, range_floor} =
                        window_memory.selection
                    {
                        let space_heights = staff_space_heights(&window_memory.staves,
                            &window_memory.staff_scales, window_memory.default_staff_space_height);
                        let next_duration_address = insert_duration(GetDC(window_handle),
                            &mut window_memory.slices, &mut window_memory.staves, &space_heights,
                            SendMessageW(window_memory.duration_spin_handle,
                            UDM_GETPOS32, 0, 0) as i8, None,
                            SendMessageW(window_memory.augmentation_dot_spin_handle, UDM_GETPOS32,
                            0, 0) as u8, &address);
                        window_memory.selection =
                            Selection::ActiveCursor{address: next_duration_address, range_floor};
                        invalidate_work_region(window_handle);
                    }
                    0
                },
                VK_UP =>
                {
                    let window_memory = main_window_memory(window_handle);
                    match &mut window_memory.selection
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
                        Selection::Objects(addresses) =>
                        {
                            for address in addresses
                            {
                                let staff_line_count = window_memory.
                                    staves[address.range_address.staff_index].line_count as i8;
                                match resolve_address(&mut window_memory.staves, address).
                                    object_type
                                {
                                    ObjectType::Clef{ref mut baseline_offset,..} =>
                                    {
                                        let new_baseline = *baseline_offset + 1;
                                        if new_baseline < staff_line_count
                                        {
                                            *baseline_offset = new_baseline;
                                        }
                                    },
                                    ObjectType::Duration{ref mut pitch,..} =>
                                    {
                                        if let Some(pitch) = pitch
                                        {
                                            if *pitch < i8::max_value()
                                            {
                                                *pitch += 1;
                                            }
                                        }
                                    },
                                    _ => ()
                                }
                            }
                        },
                        Selection::None => return 0
                    }
                    invalidate_work_region(window_handle);
                    0
                },
                _ => DefWindowProcW(window_handle, u_msg, w_param, l_param)
            }            
        },
        WM_LBUTTONDOWN =>
        {
            let window_memory = main_window_memory(window_handle);
            let device_context = GetDC(window_handle);
            let zoom_factor = zoom_factor(window_memory.zoom_trackbar_handle);
            let click_x = GET_X_LPARAM(l_param);
            let click_y = GET_Y_LPARAM(l_param);
            let buffer_device_context = CreateCompatibleDC(device_context);
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
            for staff_index in 0..window_memory.staves.len()
            {
                let space_height = staff_space_height(&window_memory.staves[staff_index],
                    &window_memory.staff_scales, window_memory.default_staff_space_height);
                let address = address_of_clicked_staff_object(window_handle, buffer_device_context,
                    &window_memory.slices, &mut window_memory.staves, space_height,
                    window_memory.system_left_edge, staff_index, click_x, click_y, zoom_factor);                            
                if let Some(address) = address
                {
                    window_memory.selection = Selection::Objects(vec![address]);
                    invalidate_work_region(window_handle);
                    break;
                }
            }
            DeleteObject(buffer as *mut winapi::ctypes::c_void);
            match window_memory.ghost_cursor
            {
                Some(_) =>
                {
                    cancel_selection(window_handle); 
                    invalidate_work_region(window_handle);
                    window_memory.selection = Selection::ActiveCursor{address: std::mem::replace(
                        &mut window_memory.ghost_cursor, None).unwrap(), range_floor: 3}; 
                    EnableWindow(window_memory.add_clef_button_handle, TRUE);
                    EnableWindow(window_memory.add_key_sig_button_handle, TRUE);
                },
                _ => ()
            }
            0
        },
        WM_MOUSEMOVE =>
        {
            let window_memory = main_window_memory(window_handle);
            let zoom_factor = zoom_factor(window_memory.zoom_trackbar_handle);
            let mouse_x = GET_X_LPARAM(l_param);
            let mouse_y = GET_Y_LPARAM(l_param);                
            for staff_index in 0..window_memory.staves.len()
            {
                let staff = &window_memory.staves[staff_index];
                let vertical_bounds = staff_vertical_bounds(&staff, staff_space_height(
                    &staff, &window_memory.staff_scales, window_memory.default_staff_space_height),
                    zoom_factor);
                if mouse_x >= to_screen_coordinate(
                    window_memory.system_left_edge as f32, zoom_factor) &&
                    vertical_bounds.top <= mouse_y && mouse_y <= vertical_bounds.bottom
                {
                    match window_memory.selection
                    {
                        Selection::ActiveCursor{ref address,..} =>
                        {
                            if address.range_address.staff_index == staff_index
                            {
                                return 0;
                            }
                        }
                        _ => ()
                    }
                    if let Some(ref address) = window_memory.ghost_cursor
                    {
                        if address.range_address.staff_index == staff_index
                        {
                            return 0;
                        }
                        invalidate_work_region(window_handle);
                    }
                    let object_index =
                    if staff.object_ranges[0].other_objects.len() > 0
                    {
                        Some(0)
                    }   
                    else
                    {
                        None
                    };  
                    window_memory.ghost_cursor = Some(Address{range_address: RangeAddress{
                        staff_index: staff_index, range_index: 0}, object_index: object_index});          
                    invalidate_work_region(window_handle);
                    return 0;
                }
            }
            match window_memory.ghost_cursor
            {
                Some(_) =>
                {                     
                    invalidate_work_region(window_handle);
                    window_memory.ghost_cursor = None;
                }
                None => ()
            }
            0
        },
        WM_NOTIFY =>
        {
            let lpmhdr = l_param as LPNMHDR;
            if (*lpmhdr).code == UDN_DELTAPOS
            {
                let window_memory = main_window_memory(window_handle);
                let lpnmud = l_param as LPNMUPDOWN;
                let new_position = (*lpnmud).iPos + (*lpnmud).iDelta;
                if (*lpmhdr).hwndFrom == window_memory.duration_spin_handle
                {                    
                    let new_text =                
                    if new_position > MAX_LOG2_DURATION
                    {
                        SendMessageW(window_memory.augmentation_dot_spin_handle, UDM_SETRANGE32, 0,
                            11);                            
                        wide_char_string("double whole")
                    }
                    else if new_position < MIN_LOG2_DURATION
                    {
                        SendMessageW(window_memory.augmentation_dot_spin_handle, UDM_SETRANGE32, 0,
                            0);
                        SendMessageW(window_memory.augmentation_dot_spin_handle, UDM_SETPOS32, 0, 
                            0);
                        wide_char_string("1024th")                        
                    }
                    else
                    {
                        let new_max_dot_count = (new_position - MIN_LOG2_DURATION) as isize;
                        if SendMessageW(window_memory.augmentation_dot_spin_handle, UDM_GETPOS32, 0,
                            0) > new_max_dot_count
                        {
                            SendMessageW(window_memory.augmentation_dot_spin_handle, UDM_SETPOS32,
                                0, new_max_dot_count);
                        }
                        SendMessageW(window_memory.augmentation_dot_spin_handle, UDM_SETRANGE32, 0,
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
                    SendMessageW(window_memory.duration_display_handle, WM_SETTEXT, 0,
                        new_text.as_ptr() as isize);                
                }
            }
            0
        },
        WM_PAINT =>
        {
            let window_memory = main_window_memory(window_handle);
            let zoom_factor = 10.0f32.powf(((SendMessageW(window_memory.zoom_trackbar_handle,
                TBM_GETPOS, 0, 0) - TRACKBAR_MIDDLE) as f32) / TRACKBAR_MIDDLE as f32);
            let mut ps: PAINTSTRUCT = std::mem::uninitialized();
            let device_context = BeginPaint(window_handle, &mut ps);
            let original_device_context = SaveDC(device_context);
            SelectObject(device_context, GetStockObject(BLACK_PEN as i32));
            SelectObject(device_context, GetStockObject(BLACK_BRUSH as i32)); 
            SetTextColor(device_context, BLACK);           
            let mut client_rect = RECT{bottom: 0, left: 0, right: 0, top: 0};
            GetClientRect(window_handle, &mut client_rect);
            for staff in &window_memory.staves
            {
                let space_height = staff_space_height(staff, &window_memory.staff_scales,
                    window_memory.default_staff_space_height);
                let zoomed_font_set = staff_font_set(zoom_factor * space_height);
                SelectObject(device_context,
                    zoomed_font_set.full_size as *mut winapi::ctypes::c_void);
                for line_index in 0..staff.line_count
                {
                    draw_horizontal_line(device_context, window_memory.system_left_edge as f32,                        
                        client_rect.right as f32, y_of_steps_above_bottom_line(staff, space_height,
                        2 * line_index as i8), space_height * BRAVURA_METADATA.staff_line_thickness,
                        zoom_factor);
                }
                let mut x = window_memory.system_left_edge;
                let mut slice_index = 0;
                let mut staff_middle_pitch = 6;
                for index in 0..staff.object_ranges.len()
                {
                    let object_range = &staff.object_ranges[index];
                    while slice_index <= object_range.slice_index
                    {
                        x += window_memory.slices[slice_index].distance_from_previous_slice;
                        slice_index += 1;
                    }
                    for range_object in &object_range.other_objects
                    {
                        draw_with_highlight(device_context, &zoomed_font_set, staff, space_height,
                        &range_object.object, x - range_object.distance_to_slice_object,
                            &mut staff_middle_pitch, zoom_factor);
                    }
                    draw_with_highlight(device_context, &zoomed_font_set, staff, space_height,
                        &object_range.slice_object, x, &mut staff_middle_pitch, zoom_factor);
                }
            }            
            if let Some(address) = &window_memory.ghost_cursor
            {
                SelectObject(device_context, GRAY_PEN.unwrap() as *mut winapi::ctypes::c_void);
                SelectObject(device_context, GRAY_BRUSH.unwrap() as *mut winapi::ctypes::c_void);
                let cursor_x = cursor_x(&window_memory.slices, &window_memory.staves,
                    window_memory.system_left_edge, address);
                let staff = &window_memory.staves[address.range_address.staff_index];
                let vertical_bounds = staff_vertical_bounds(staff, staff_space_height(staff,
                    &window_memory.staff_scales, window_memory.default_staff_space_height),
                    zoom_factor);
                let left_edge = to_screen_coordinate(cursor_x as f32, zoom_factor);
                Rectangle(device_context, left_edge, vertical_bounds.top, left_edge + 1,
                    vertical_bounds.bottom);               
            }
            if let Selection::ActiveCursor{address, range_floor} = &window_memory.selection
            {
                SelectObject(device_context, RED_PEN.unwrap() as *mut winapi::ctypes::c_void);
                SelectObject(device_context, RED_BRUSH.unwrap() as *mut winapi::ctypes::c_void);
                let cursor_x = cursor_x(&window_memory.slices, &window_memory.staves,
                    window_memory.system_left_edge, address);
                let staff = &window_memory.staves[address.range_address.staff_index];   
                let staff_space_height = staff_space_height(staff, &window_memory.staff_scales,
                    window_memory.default_staff_space_height);           
                let steps_of_floor_above_bottom_line = range_floor - bottom_line_pitch(
                    staff.line_count, staff_middle_pitch_at_address(staff, &address));                    
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
            RestoreDC(device_context, original_device_context);
            EndPaint(window_handle, &mut ps);
            DefWindowProcW(window_handle, u_msg, w_param, l_param)
        },
        WM_SIZE =>
        {
            let window_memory =
                GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
            if window_memory != null_mut()
            {
                position_zoom_trackbar(window_handle, (*window_memory).zoom_trackbar_handle);
            }
            0
        }, 
        _ => DefWindowProcW(window_handle, u_msg, w_param, l_param)
    }
}

fn next_address(staff: &Staff, address: &Address) -> Option<Address>
{
    let mut range_index = address.range_address.range_index;
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
        return Some(Address{range_address: RangeAddress{staff_index:
            address.range_address.staff_index, range_index: range_index}, object_index: None});
    }
    Some(Address{range_address: RangeAddress{staff_index: address.range_address.staff_index,
        range_index: range_index}, object_index: Some(object_index)})
}

fn object_width(device_context: HDC, font_set: &FontSet, staff_space_height: f32,
    object: &ObjectType) -> i32
{
    match object
    {
        ObjectType::Clef{codepoint, header,..} =>
        {
            let font =
            if *header
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
            *augmentation_dot_count as i32 *
                ((staff_space_height * DISTANCE_BETWEEN_AUGMENTATION_DOTS).round() as i32 +
                character_width(device_context, font_set.full_size, 0xe1e7)) +
                character_width(device_context, font_set.full_size,
                duration_codepoint(*log2_duration, *pitch) as u32)
        },
        ObjectType::KeySignature{accidental_count, flats,..} =>
        {
            let codepoint =
            if *flats
            {
                0xe260
            }
            else
            {
                0xe262
            };
            *accidental_count as i32 *
                character_width(device_context, font_set.full_size, codepoint as u32)
        }
        ObjectType::None => 0
    }
}

fn position_zoom_trackbar(parent_window_handle: HWND, trackbar_handle: HWND)
{
    unsafe
    {
        let mut client_rect = RECT{bottom: 0, left: 0, right: 0, top: 0};
        GetClientRect(parent_window_handle, &mut client_rect);
        SetWindowPos(trackbar_handle, null_mut(), (client_rect.right - client_rect.left) / 2 - 70,
            client_rect.bottom - 20, 140, 20, 0);
    }
}

fn previous_address(staff: &Staff, address: &Address) -> Option<Address>
{
    let object_index =
    if let Some(index) = address.object_index
    {
        index
    }
    else
    {
        staff.object_ranges[address.range_address.range_index].other_objects.len()
    };
    if object_index == 0
    {
        if address.range_address.range_index == 0
        {
            return None;
        }
        return Some(Address{range_address:
            RangeAddress{staff_index: address.range_address.staff_index,
            range_index: address.range_address.range_index - 1}, object_index: None})
    }
    Some(Address{range_address: RangeAddress{staff_index: address.range_address.staff_index,
        range_index: address.range_address.range_index}, object_index: Some(object_index - 1)})
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
            insert_slice(slices, staves, *slice_index, Slice{objects: vec![], slice_type:
                SliceType::Duration{rhythmic_position: position}, distance_from_previous_slice: 0});
            break;
        }        
        if let SliceType::Duration{rhythmic_position} = &slices[*slice_index].slice_type
        {
            if *rhythmic_position > position
            {
                insert_slice(slices, staves, *slice_index, Slice{objects: vec![],
                    slice_type: SliceType::Duration{rhythmic_position: position},
                    distance_from_previous_slice: 0});
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

fn remove_object_range(staves: &mut Vec<Staff>, slices: &mut Vec<Slice>, staff_index: usize,
    range_index: usize, slice_index: usize)
{
    let objects_in_slice_count = slices[slice_index].objects.len();
    if objects_in_slice_count == 1
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
    staves[staff_index].object_ranges.remove(range_index);
    increment_range_indices(&staves[staff_index], slices,
        &RangeAddress{staff_index: staff_index, range_index: range_index}, decrement);
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
        if let ObjectType::Duration{log2_duration,..} =
            &staff.object_ranges[address.range_index].slice_object.object_type
        {
            range_width += left_edge_to_origin_distance(space_height, *log2_duration)
        }
        for object_index in (0..staff.object_ranges[address.range_index].other_objects.len()).rev()
        {
            let range_object =
                &mut staff.object_ranges[address.range_index].other_objects[object_index];
            let object = &range_object.object.object_type;
            range_width += object_width(device_context, &font_set, space_height, object) +
                spacer(object, space_height);
            spacer = space_between_objects(object);
            range_object.distance_to_slice_object = range_width;
        }
        if address.range_index > 0
        {
            let previous_range = &staff.object_ranges[address.range_index - 1];
            let previous_slice_object = &previous_range.slice_object.object_type;
            range_width += object_width(device_context, &font_set, space_height,
                previous_slice_object) + spacer(previous_slice_object, space_height);
            if let ObjectType::Duration{log2_duration, augmentation_dot_count,..} =
                previous_slice_object
            {
                    range_width -= left_edge_to_origin_distance(space_height, *log2_duration);
                    range_width = std::cmp::max(range_width,
                        duration_width(*log2_duration, *augmentation_dot_count));
            }
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

fn resolve_address<'a>(staves: &'a mut Vec<Staff>, address: &Address) -> &'a mut Object
{
    let range = &mut staves[address.range_address.staff_index].
        object_ranges[address.range_address.range_index];
    if let Some(object_index) = address.object_index
    {
        &mut range.other_objects[object_index].object
    }
    else
    {
        &mut range.slice_object
    }
}

fn rest_codepoint(log2_duration: i8) -> u16
{
    (0xe4e3 - log2_duration as i32) as u16
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

fn space_between_objects(right_object: &ObjectType) -> fn(&ObjectType, f32) -> i32
{
    match right_object
    {
        ObjectType::Clef{..} =>
        {
            |_left_object: &ObjectType, staff_space_height: f32|
            {
                staff_space_height.round() as i32
            }
        },
        ObjectType::Duration{..} => |left_object: &ObjectType, staff_space_height: f32|
            {
                let multiplier =
                match left_object
                {
                    ObjectType::Clef{header,..} =>
                    {
                        if *header
                        {
                            2.5
                        }
                        else
                        {
                            1.0
                        }
                    },
                    ObjectType::Duration{..} => 0.0,
                    ObjectType::KeySignature{header,..} =>
                    {
                        if *header
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
            },
        ObjectType::KeySignature{..} => 
        {
            |_left_object: &ObjectType, staff_space_height: f32|
            {
                staff_space_height.round() as i32
            }
        },
        ObjectType::None => |_left_object: &ObjectType, _staff_space_height: f32|{0}
    }
}

fn space_new_object(main_window_memory: &mut MainWindowMemory, main_window_handle: HWND,
    staff_index: usize, mut insertion_range_index: usize)
{
    let space_heights = staff_space_heights(&main_window_memory.staves,
        &main_window_memory.staff_scales, main_window_memory.default_staff_space_height);
    let slice_index =
        main_window_memory.staves[staff_index].object_ranges[insertion_range_index].slice_index;
    let device_context =
    unsafe
    {
        GetDC(main_window_handle)
    };
    reset_distance_from_previous_slice(device_context, &mut main_window_memory.slices,
        &mut main_window_memory.staves, &space_heights, slice_index);
    insertion_range_index += 1;
    if insertion_range_index < main_window_memory.staves[staff_index].object_ranges.len()
    {
        let slice_index =
            main_window_memory.staves[staff_index].object_ranges[insertion_range_index].slice_index;
        reset_distance_from_previous_slice(device_context, &mut main_window_memory.slices,
            &mut main_window_memory.staves, &space_heights, slice_index);
    }
    invalidate_work_region(main_window_handle);
}

fn staff_font(staff_space_height: f32, staff_height_multiple: f32) -> HFONT
{
    unsafe
    {
        CreateFontW(-(4.0 * staff_height_multiple * staff_space_height).round() as i32,
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

fn staff_middle_pitch_at_address(staff: &Staff, address: &Address) -> i8
{
    let mut previous_address = previous_address(staff, address);
    loop
    {
        if let Some(address) = previous_address
        {
            let object_type =
            if let Some(object_index) = address.object_index
            {
                &staff.object_ranges[address.range_address.range_index].
                    other_objects[object_index].object.object_type
            }
            else
            {
                &staff.object_ranges[address.range_address.range_index].slice_object.object_type
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