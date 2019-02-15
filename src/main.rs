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
const WM_GET_STAVES: u32 = WM_USER;

static mut GRAY_PEN: Option<HPEN> = None;
static mut GRAY_BRUSH: Option<HBRUSH> = None;
static mut RED_PEN: Option<HPEN> = None;
static mut RED_BRUSH: Option<HBRUSH> = None;

struct Address
{
    staff_index: usize,
    address_type: AddressType
}

enum AddressType
{
    Object
    {
        range_index: usize,
        object_index: usize
    },
    Duration
    {
        duration_index: usize
    },
    HeaderClef
}

struct Clef
{
    codepoint: u16,
    steps_of_baseline_above_middle: i8,
    is_selected: bool
}

struct Duration
{
    //Denotes the power of two times the duration of a whole note of the object's duration.
    log2_duration: i8,
    pitch: Option<i8>,//In steps above c4.
    augmentation_dot_count: u8,
    is_selected: bool
}

struct DurationAddress
{
    staff_index: usize,
    duration_index: usize
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
    slices: Vec<RhythmicSlice>,
    header_spacer: i32,
    header_clef_width: i32,
    staves: Vec<Staff>,
    system_left_edge: i32,
    ghost_cursor: Option<Address>,
    selection: Selection,
    add_staff_button_handle: HWND,
    add_clef_button_handle: HWND,
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
    objects: Vec<RangeObject>
}

enum ObjectType
{
    Clef(Clef),
    KeySignature
    {
        accidental_count: u8,
        flats: bool
    }
}

struct RangeObject
{    
    object: Object,
    distance_to_next_slice: i32
}

struct RhythmicSlice
{
    durations: Vec<DurationAddress>,
    rhythmic_position: num_rational::Ratio<num_bigint::BigUint>,
    distance_from_previous_slice: i32
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

struct Staff
{
    header_clef: Option<Clef>,
    object_ranges: Vec<ObjectRange>,
    durations: Vec<Duration>,
    line_thickness: f32,
    vertical_center: i32,
    scale_index: usize,
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

trait Drawable
{
    fn draw(&self, device_context: HDC, zoomed_font_set: &FontSet, staff: &Staff,
        staff_space_height: f32, x: i32, staff_middle_pitch: &mut i8, zoom_factor: f32);
    fn draw_with_highlight(&self, device_context: HDC, zoomed_font_set: &FontSet, staff: &Staff,
        staff_space_height: f32, x: i32, staff_middle_pitch: &mut i8, zoom_factor: f32)
    {
        unsafe
        {
            if self.is_selected()
            {
                SetTextColor(device_context, RED);                        
            }
            else
            {
                SetTextColor(device_context, BLACK);
            }
        }
        self.draw(device_context, zoomed_font_set, staff, staff_space_height, x, staff_middle_pitch,
            zoom_factor);
    }
    fn is_selected(&self) -> bool;
}

impl Drawable for Clef
{
    fn draw(&self, device_context: HDC, zoomed_font_set: &FontSet, staff: &Staff,
        staff_space_height: f32, x: i32, staff_middle_pitch: &mut i8, zoom_factor: f32)
    {
        draw_clef(device_context, zoomed_font_set.two_thirds_size, staff, staff_space_height, self,
            x, staff_middle_pitch, zoom_factor);
    }
    fn is_selected(&self) -> bool
    {
        self.is_selected
    }
}

impl Drawable for Duration
{
    fn draw(&self, device_context: HDC, zoomed_font_set: &FontSet, staff: &Staff,
        staff_space_height: f32, x: i32, staff_middle_pitch: &mut i8, zoom_factor: f32)
    {
        let duration_codepoint;
        let mut duration_left_edge = x;
        let duration_right_edge;
        let duration_y;
        let augmentation_dot_y;
        let unzoomed_font = staff_font(staff_space_height, 1.0);
        if let Some(pitch) = self.pitch
        {        
            let steps_above_bottom_line =
                pitch - bottom_line_pitch(staff.line_count, *staff_middle_pitch);
            duration_y =
                y_of_steps_above_bottom_line(staff, staff_space_height, steps_above_bottom_line);
            augmentation_dot_y =
            if steps_above_bottom_line % 2 == 0
            {
                y_of_steps_above_bottom_line(staff, staff_space_height, steps_above_bottom_line + 1)
            }
            else
            {
                duration_y
            };
            if self.log2_duration == 1
            {
                duration_codepoint = 0xe0a0;
                duration_left_edge -= (staff_space_height *
                    BRAVURA_METADATA.double_whole_notehead_x_offset).round() as i32;
            }
            else if self.log2_duration == 0
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
                    if self.log2_duration == -1
                    {
                        duration_codepoint = 0xe0a3;
                        stem_right_edge = x as f32 +
                            staff_space_height * BRAVURA_METADATA.half_notehead_stem_up_se.x;
                        stem_left_edge =
                            stem_right_edge - staff_space_height * BRAVURA_METADATA.stem_thickness;
                        stem_bottom = duration_y as f32 -
                            staff_space_height * BRAVURA_METADATA.half_notehead_stem_up_se.y;                        
                    }
                    else
                    {
                        duration_codepoint = 0xe0a4;
                        stem_right_edge = x as f32 +
                            staff_space_height * BRAVURA_METADATA.black_notehead_stem_up_se.x;
                        stem_left_edge =
                            stem_right_edge - staff_space_height * BRAVURA_METADATA.stem_thickness;
                        stem_bottom = duration_y as f32 -
                            staff_space_height * BRAVURA_METADATA.black_notehead_stem_up_se.y;
                        if self.log2_duration == -3
                        {
                            draw_character(device_context, zoomed_font_set.full_size, 0xe240,
                                stem_left_edge, stem_top, zoom_factor);
                        }
                        else if self.log2_duration < -3
                        {
                            draw_character(device_context, zoomed_font_set.full_size, 0xe242,
                                stem_left_edge, stem_top, zoom_factor);
                            let flag_spacing = staff_space_height * (
                                BRAVURA_METADATA.beam_spacing + BRAVURA_METADATA.beam_thickness);
                            for _ in 0..-self.log2_duration - 4
                            {
                                stem_top -= flag_spacing;
                                draw_character(device_context, zoomed_font_set.full_size, 0xe250,
                                    stem_left_edge, stem_top, zoom_factor);
                            }
                        }
                    }
                }
                else
                {
                    stem_bottom = y_of_steps_above_bottom_line(staff, staff_space_height,
                        std::cmp::min(steps_above_bottom_line - 7, space_count));
                    if self.log2_duration == -1
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
                        if self.log2_duration == -3
                        {
                            draw_character(device_context, zoomed_font_set.full_size,
                                0xe241, stem_left_edge, stem_bottom, zoom_factor);
                        }
                        else if self.log2_duration < -3
                        {
                            draw_character(device_context, zoomed_font_set.full_size, 0xe243,
                                stem_left_edge, stem_bottom, zoom_factor);
                            let flag_spacing = staff_space_height * 
                                (BRAVURA_METADATA.beam_spacing + BRAVURA_METADATA.beam_thickness);
                            for _ in 0..-self.log2_duration - 4
                            {      
                                stem_bottom += flag_spacing;
                                draw_character(device_context, zoomed_font_set.full_size, 0xe251,
                                    stem_left_edge, stem_bottom, zoom_factor);
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
            if self.log2_duration == 0
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
            duration_codepoint = rest_codepoint(self.log2_duration);  
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
        for _ in 0..self.augmentation_dot_count
        {
            draw_character(device_context, zoomed_font_set.full_size, 0xe1e7,
                next_dot_left_edge as f32, augmentation_dot_y, zoom_factor);
            next_dot_left_edge += dot_offset;
        }
    }
    fn is_selected(&self) -> bool
    {
        self.is_selected
    }
}

impl Drawable for RangeObject
{
    fn draw(&self, device_context: HDC, zoomed_font_set: &FontSet, staff: &Staff,
        staff_space_height: f32, x: i32, staff_middle_pitch: &mut i8, zoom_factor: f32)
    {
        match &self.object.object_type
        {
            ObjectType::Clef(clef) => draw_clef(device_context, zoomed_font_set.two_thirds_size,
                staff, staff_space_height, &clef, x, staff_middle_pitch, zoom_factor),
            ObjectType::KeySignature{accidental_count, flats} =>
            {
                let codepoint;
                let stride;
                let mut steps_of_accidental_above_floor;
                let steps_of_floor_above_middle;
                if *flats
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
                        steps_of_accidental_above_floor = 5;
                    }
                    else
                    {
                        steps_of_floor_above_middle = -1 - steps_of_middle_above_b;
                        steps_of_accidental_above_floor = 0;
                    }
                }
                let steps_of_floor_above_bottom_line =
                    steps_of_floor_above_middle + staff.line_count as i8 - 1;
                let accidental_width =
                    character_width(device_context, staff_font(staff_space_height, 1.0), codepoint);
                let mut x = x;
                for _ in 0..*accidental_count
                {
                    draw_character(device_context, zoomed_font_set.full_size, codepoint as u16,
                        x as f32, y_of_steps_above_bottom_line(staff, staff_space_height,
                        steps_of_accidental_above_floor + steps_of_floor_above_bottom_line),
                        zoom_factor);
                    steps_of_accidental_above_floor =
                        (steps_of_accidental_above_floor + stride) % 7;
                    x += accidental_width;
                }
            }
        }
    }
    fn is_selected(&self) -> bool
    {
        self.object.is_selected
    }
}

fn add_clef(device_context: HDC, slices: &mut Vec<RhythmicSlice>, address: &mut Address,
    staves: &mut Vec<Staff>, staff_space_heights: &Vec<f32>, header_clef_width: &mut i32, clef: Clef)
{
    let insertion_range_index;
    let insertion_object_index;
    match address.address_type
    {
        AddressType::Duration{duration_index} =>
        {
            insertion_range_index = duration_index;
            insertion_object_index =
                staves[address.staff_index].object_ranges[duration_index].objects.len();
        },
        AddressType::HeaderClef =>
        {
            insertion_range_index = 0;
            insertion_object_index = 0;
        },
        AddressType::Object{range_index, object_index} =>
        {
            insertion_range_index = range_index;
            insertion_object_index = object_index;
        }
    }
    address.address_type = next_address(&staves[address.staff_index],
        &AddressType::Object{range_index: insertion_range_index,
        object_index: insertion_object_index}).unwrap();
    if insertion_range_index == 0 && insertion_object_index == 0
    {
        let new_clef_width = character_width(device_context,
            staff_font(staff_space_heights[address.staff_index], 1.0), clef.codepoint as u32);
        if new_clef_width < *header_clef_width
        {
            if let Some(_) = staves[address.staff_index].header_clef
            {
                *header_clef_width = 0;
                for staff_index in 0..staves.len()
                {
                    if let Some(clef) = &staves[staff_index].header_clef
                    {
                        let clef_width = character_width(device_context,
                            staff_font(staff_space_heights[staff_index], 1.0),
                            clef.codepoint as u32);
                        *header_clef_width = std::cmp::max(clef_width, *header_clef_width);
                    }
                }        
            }            
        }
        else
        {
            *header_clef_width = new_clef_width;
        }
        staves[address.staff_index].header_clef = Some(clef);     
        return;
    }
    add_non_header_clef(&mut staves[address.staff_index].object_ranges[insertion_range_index], clef,
        insertion_object_index);
    reset_distance_from_previous_slice(device_context, slices, staves, staff_space_heights,
        insertion_range_index);
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

fn add_duration(device_context: HDC, slices: &mut Vec<RhythmicSlice>, staves: &mut Vec<Staff>,
    staff_scales: &Vec<f32>, slice_index: &mut usize,
    rhythmic_position: num_rational::Ratio<num_bigint::BigUint>, log2_duration: i8,
    pitch: Option<i8>, augmentation_dots: u8, staff_index: usize, duration_index: usize)
{
    register_rhythmic_position(slices, staves, slice_index, rhythmic_position, staff_index,
        duration_index);
    staves[staff_index].durations.insert(duration_index, Duration{log2_duration: log2_duration,
        pitch: pitch, augmentation_dot_count: augmentation_dots, is_selected: false});
    reset_distance_from_previous_slice(device_context, slices, staves, staff_scales, *slice_index);
}

fn add_non_header_clef(object_range: &mut ObjectRange, clef: Clef, object_index: usize)
{
    let clef = RangeObject{object: Object{object_type: ObjectType::Clef(clef), is_selected: false},
        distance_to_next_slice: 0};
    if object_index > 0
    {
        if let ObjectType::Clef(_) = object_range.objects[object_index - 1].object.object_type
        {
            object_range.objects[object_index - 1] = clef;
            return;
        }
    }
    if object_index < object_range.objects.len()
    {
        if let ObjectType::Clef(_) = object_range.objects[object_index].object.object_type
        {
            object_range.objects[object_index] = clef;
            return;
        }
    }
    object_range.objects.insert(object_index, clef);
}

fn add_staff_dialog_memory<'a>(dialog_handle: HWND) -> &'a mut Vec<StaffScale>
{
    unsafe
    {
        &mut *(GetWindowLongPtrW(dialog_handle, DWLP_USER) as *mut Vec<StaffScale>)
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
                    let staff_scales = add_staff_dialog_memory(dialog_handle);
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
                    let staff_scales = add_staff_dialog_memory(dialog_handle);
                    let scale_index = SendMessageW(GetDlgItem(dialog_handle,
                        IDC_ADD_STAFF_SCALE_LIST), CB_GETCURSEL, 0, 0) as usize;
                    let template = EDIT_STAFF_SCALE_DIALOG_TEMPLATE.data.as_ptr();
                    DialogBoxIndirectParamW(null_mut(), template as *mut DLGTEMPLATE,
                        dialog_handle, Some(edit_staff_scale_dialog_proc),
                        &mut staff_scales[scale_index] as *mut _ as isize);
                    let edited_scale = staff_scales.remove(scale_index);
                    let edited_scale_index = insert_staff_scale(staff_scales, edited_scale);
                    let scale_list_handle = GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST);
                    SendMessageW(scale_list_handle, CB_DELETESTRING, scale_index, 0);
                    SendMessageW(scale_list_handle, CB_INSERTSTRING, edited_scale_index,
                        to_string(&staff_scales[edited_scale_index]).as_ptr() as isize);
                    SendMessageW(scale_list_handle, CB_SETCURSEL, edited_scale_index, 0);
                    let mut staves: &mut Vec<Staff> = &mut vec![];
                    SendMessageW(GetWindow(dialog_handle, GW_OWNER), WM_GET_STAVES,
                        &mut staves as *mut _ as usize, 0);
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
                    for staff in staves
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
                    let mut staves: &mut Vec<Staff> = &mut vec![];
                    SendMessageW(GetWindow(dialog_handle, GW_OWNER), WM_GET_STAVES,
                        &mut staves as *mut _ as usize, 0);
                    let mut scale_is_used = false;
                    for staff_index in 0..staves.len()
                    {
                        if staves[staff_index].scale_index == removal_index
                        {
                            scale_is_used = true;
                        }
                    }
                    let remapped_index;
                    if scale_is_used == true
                    {
                        let mut reassignment_candidates = vec![]; 
                        for scale_index in 0..add_staff_dialog_memory(dialog_handle).len()
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
                        let template = REMAP_STAFF_SCALE_DIALOG_TEMPLATE.data.as_ptr();
                        remapped_index = DialogBoxIndirectParamW(null_mut(),
                            template as *mut DLGTEMPLATE, dialog_handle,
                            Some(remap_staff_scale_dialog_proc),
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
                    let staff_scales = add_staff_dialog_memory(dialog_handle);
                    staff_scales.remove(removal_index);
                    for staff in staves
                    {
                        if staff.scale_index == removal_index
                        {
                            staff.scale_index = remapped_index as usize;
                        }
                        else if staff.scale_index > removal_index
                        {
                            staff.scale_index -= 1;
                        }
                    }
                    SendMessageW(scale_list_handle, CB_DELETESTRING, removal_index, 0);
                    SendMessageW(scale_list_handle, CB_SETCURSEL, remapped_index as usize, 0);
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
                    EndDialog(dialog_handle, (SendMessageW(GetDlgItem(dialog_handle,
                        IDC_ADD_STAFF_LINE_COUNT), UDM_GETPOS32, 0, 0) |
                        SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST),
                        CB_GETCURSEL, 0, 0) << 32) as isize);
                    TRUE as isize
                },
                _ => FALSE as isize               
            }
        },
        WM_INITDIALOG =>
        {
            let line_count_spin_handle = GetDlgItem(dialog_handle, IDC_ADD_STAFF_LINE_COUNT);
            SendMessageW(line_count_spin_handle, UDM_SETRANGE32, 1, 5);
            SendMessageW(line_count_spin_handle, UDM_SETPOS32, 0, 5);
            let scale_list_handle = GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST);
            SendMessageW(scale_list_handle, CB_ADDSTRING, 0,
                wide_char_string("Default").as_ptr() as isize);
            SetWindowLongPtrW(dialog_handle, DWLP_USER, l_param);
            let staff_scales = add_staff_dialog_memory(dialog_handle);
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
    slices: &Vec<RhythmicSlice>, header_spacer: i32, header_clef_width: i32,
    staves: &mut Vec<Staff>, staff_space_height: f32, system_left_edge: i32, staff_index: usize,
    click_x: i32, click_y: i32, zoom_factor: f32) -> Option<AddressType>
{
    let staff = &staves[staff_index];
    let zoomed_font_set = staff_font_set(zoom_factor * staff_space_height);                
    let mut x = system_left_edge + header_spacer;
    if click_x < to_screen_coordinate(x as f32, zoom_factor)
    {
        return None;
    }
    let mut staff_middle_pitch = 6;
    if header_clef_width > 0
    {
        if let Some(clef) = &staff.header_clef
        {
            draw_clef(buffer_device_context, zoomed_font_set.full_size, staff, staff_space_height,
                clef, x, &mut staff_middle_pitch, zoom_factor);
            unsafe
            {
                if GetPixel(buffer_device_context, click_x, click_y) == WHITE
                {
                    cancel_selection(window_handle);
                    staves[staff_index].header_clef.as_mut().unwrap().is_selected = true;
                    return Some(AddressType::HeaderClef);
                } 
            }    
        }                    
        x += header_clef_width + header_spacer;
    }
    let mut slice_index = 0;
    for range_index in 0..staff.object_ranges.len()
    {
        let object_range = &staff.object_ranges[range_index];
        while slice_index <= object_range.slice_index
        {
            x += slices[slice_index].distance_from_previous_slice;
            slice_index += 1;
        }
        for object_index in 0..object_range.objects.len()
        {
            let object = &object_range.objects[object_index];
            let object_x = x - object.distance_to_next_slice;
            if click_x < to_screen_coordinate(object_x as f32, zoom_factor)
            {
                return None;
            }
            object.draw(buffer_device_context, &zoomed_font_set, staff, staff_space_height,
                x - object.distance_to_next_slice, &mut staff_middle_pitch, zoom_factor);
            unsafe
            {
                if GetPixel(buffer_device_context, click_x, click_y) == WHITE
                {
                    cancel_selection(window_handle);
                    staves[staff_index].object_ranges[range_index].objects[object_index].object.
                        is_selected = true;
                    return Some(AddressType::Object{range_index: range_index,
                        object_index: object_index});
                }
            }
        }
        if click_x < to_screen_coordinate(x as f32, zoom_factor)
        {
            return None;
        }
        if range_index < staff.durations.len()
        {
            staff.durations[range_index].draw(buffer_device_context, &zoomed_font_set, staff,
                staff_space_height, x, &mut staff_middle_pitch, zoom_factor);
            unsafe
            {
                if GetPixel(buffer_device_context, click_x, click_y) == WHITE
                {
                    cancel_selection(window_handle);
                    staves[staff_index].durations[range_index].is_selected = true;
                    return Some(AddressType::Duration{duration_index: range_index});
                }
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
                let staff = &mut window_memory.staves[address.staff_index];
                match address.address_type
                {
                    AddressType::Object{range_index, object_index} =>
                    {
                        staff.object_ranges[range_index].objects[object_index].object.is_selected =
                            false;
                    },
                    AddressType::Duration{duration_index} =>
                        staff.durations[duration_index].is_selected = false,
                    AddressType::HeaderClef =>
                        staff.header_clef.as_mut().unwrap().is_selected = false
                }
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

fn clef_baseline(staff: &Staff, staff_space_height: f32, clef: &Clef) -> f32
{
    y_of_steps_above_bottom_line(staff, staff_space_height,
        staff.line_count as i8 - 1 + clef.steps_of_baseline_above_middle)
}

fn cursor_x(slices: &Vec<RhythmicSlice>, header_spacer: i32, header_clef_width: i32,
    staves: &Vec<Staff>, system_left_edge: i32, address: &Address) -> i32
{
    let mut x = system_left_edge + header_spacer;
    let staff = &staves[address.staff_index];
    match address.address_type
    {
        AddressType::Duration{duration_index} =>
        {
            if header_clef_width > 0
            {
                x += header_clef_width + header_spacer;
            }
            for slice_index in 0..=staff.object_ranges[duration_index].slice_index
            {
                x += slices[slice_index].distance_from_previous_slice;
            }
        },
        AddressType::HeaderClef => (),
        AddressType::Object{range_index, object_index} =>
        {
            if header_clef_width > 0
            {
                x += header_clef_width + header_spacer;
            }            
            let object_range = &staff.object_ranges[range_index];
            for slice_index in 0..=object_range.slice_index
            {
                x += slices[slice_index].distance_from_previous_slice;
            }
            x -= object_range.objects[object_index].distance_to_next_slice;
        }
    };
    x
}

fn decrement(index: &mut usize)
{
    *index -= 1;
}

fn decrement_baseline(staff_line_count: u8, clef: &mut Clef)
{
    let new_baseline = clef.steps_of_baseline_above_middle - 1;
    if new_baseline > -(staff_line_count as i8)
    {
        clef.steps_of_baseline_above_middle = new_baseline;
    }
}

fn draw_character(device_context: HDC, font: HFONT, codepoint: u16, x: f32, y: f32,
    zoom_factor: f32)
{
    unsafe
    {
        SelectObject(device_context, font as *mut winapi::ctypes::c_void);
        TextOutW(device_context, to_screen_coordinate(x, zoom_factor),
            to_screen_coordinate(y, zoom_factor), vec![codepoint, 0].as_ptr(), 1);
    }
}

fn draw_clef(device_context: HDC, font: HFONT, staff: &Staff, staff_space_height: f32, clef: &Clef,
    x: i32, staff_middle_pitch: &mut i8, zoom_factor: f32)
{
    *staff_middle_pitch = self::staff_middle_pitch(clef);
    draw_character(device_context, font, clef.codepoint, x as f32,
        clef_baseline(staff, staff_space_height, clef), zoom_factor);
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

fn duration_codepoint(duration: &Duration) -> u16
{
    match duration.pitch
    {
        Some(_) =>
        {
            match duration.log2_duration
            {
                1 => 0xe0a0,
                0 => 0xe0a2,
                -1 => 0xe0a3,
                _ => 0xe0a4
            }
        },
        None =>
        {
            rest_codepoint(duration.log2_duration)
        }
    }
}

fn duration_width(duration: &Duration) -> i32
{
    if duration.augmentation_dot_count == 0
    {
        return (WHOLE_NOTE_WIDTH as f32 *
            DURATION_RATIO.powi(duration.log2_duration as i32)).round() as i32;
    }
    let whole_notes_long =
        whole_notes_long(duration.log2_duration, duration.augmentation_dot_count);
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
                            let name_edit = GetDlgItem(dialog_handle, IDC_EDIT_STAFF_SCALE_NAME);
                            let name_length =
                                SendMessageW(name_edit, WM_GETTEXTLENGTH, 0, 0) as usize + 1;
                            let mut name: Vec<u16> = vec![0; name_length];
                            SendMessageW(name_edit, WM_GETTEXT, name_length,
                                name.as_ptr() as isize);
                            name.pop();
                            *edit_staff_scale_dialog_memory(dialog_handle) =
                                StaffScale{name: name, value: value};
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

fn increment_baseline(staff_line_count: u8, clef: &mut Clef)
{
    let new_baseline = clef.steps_of_baseline_above_middle + 1;
    if new_baseline < staff_line_count as i8
    {
        clef.steps_of_baseline_above_middle = new_baseline;
    }
}

fn increment_slice_indices(slices: &mut Vec<RhythmicSlice>,
    staves: &mut Vec<Staff>, starting_slice_index: usize, increment_operation: fn(&mut usize))
{
    for slice_index in starting_slice_index..slices.len()
    {
        for duration_address in &slices[slice_index].durations
        {
            increment_operation(&mut staves[duration_address.staff_index].
                object_ranges[duration_address.duration_index].slice_index);
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
    let add_clef_button_handle = CreateWindowExW(0, button_string.as_ptr(),
        wide_char_string("Add clef").as_ptr(), BS_PUSHBUTTON | WS_DISABLED | WS_CHILD | WS_VISIBLE |
        BS_VCENTER, 0, 0, 70, 20, main_window_handle, null_mut(), instance, null_mut());
    if add_clef_button_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create add clef button; error code {}", GetLastError());
    }
    let add_staff_button_handle = CreateWindowExW(0, button_string.as_ptr(),
        wide_char_string("Add staff").as_ptr(), WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON | BS_VCENTER,
        0, 20, 70, 20, main_window_handle, null_mut(), instance, null_mut());
    if add_staff_button_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create add staff button; error code {}", GetLastError());
    } 
    if CreateWindowExW(0, static_string.as_ptr(), wide_char_string("Selected duration:").as_ptr(),
        SS_CENTER | WS_CHILD | WS_VISIBLE, 70, 0, 140, 20, main_window_handle, null_mut(),
        instance, null_mut()) == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create selected duration label; error code {}", GetLastError());
    }
    let duration_display_handle = CreateWindowExW(0, static_string.as_ptr(),
        wide_char_string("quarter").as_ptr(), WS_BORDER | WS_CHILD | WS_VISIBLE, 70,
        20, 130, 20, main_window_handle, null_mut(), instance, null_mut());
    if duration_display_handle == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create selected duration display; error code {}", GetLastError());
    }
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
    if CreateWindowExW(0, static_string.as_ptr(), wide_char_string("Augmentation dots:").as_ptr(),
        SS_CENTER | WS_VISIBLE | WS_CHILD, 200, 0, 140, 20, main_window_handle, null_mut(),
        instance, null_mut()) == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create augmentation dot label; error code {}", GetLastError());
    }
    if CreateWindowExW(0, static_string.as_ptr(), wide_char_string("0").as_ptr(),
        WS_BORDER | WS_VISIBLE | WS_CHILD, 200, 20, 130, 20, main_window_handle, null_mut(),
        instance, null_mut()) == winapi::shared::ntdef::NULL as HWND
    {
        panic!("Failed to create augmentation dot display; error code {}", GetLastError());
    }
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
        staff_scales: vec![StaffScale{name: vec![], value: 1.0},
        StaffScale{name: unterminated_wide_char_string("Cue"), value: 0.75}], slices: vec![],
        header_spacer: 0, header_clef_width: 0, staves: vec![], system_left_edge: 20,
        ghost_cursor: None, selection: Selection::None,
        add_staff_button_handle: add_staff_button_handle,
        add_clef_button_handle: add_clef_button_handle,
        duration_display_handle: duration_display_handle,
        duration_spin_handle: duration_spin_handle,
        augmentation_dot_spin_handle: augmentation_dot_spin_handle,
        zoom_trackbar_handle: zoom_trackbar_handle};        
    (main_window_handle, main_window_memory)
}

fn insert_duration(device_context: HDC, slices: &mut Vec<RhythmicSlice>,
    staves: &mut Vec<Staff>, staff_space_heights: &Vec<f32>, new_duration: Duration,
    insertion_address: &Address) -> Address
{
    let mut duration_index =
    match insertion_address.address_type
    {
        AddressType::Object{range_index, object_index} =>
        {
            staves[insertion_address.staff_index].object_ranges[range_index].objects.
                split_off(object_index);
            range_index
        },
        AddressType::Duration{duration_index} => duration_index,
        AddressType::HeaderClef => 0
    };
    let zero = num_bigint::BigUint::new(vec![]);
    let mut slice_index;
    let new_duration_rhythmic_position;
    if duration_index == 0
    {
        new_duration_rhythmic_position =
            num_rational::Ratio::new(zero.clone(), num_bigint::BigUint::new(vec![1]));
    }
    else
    {
        let staff = &staves[insertion_address.staff_index];
        slice_index = staff.object_ranges[duration_index - 1].slice_index;
        let previous_duration = &staff.durations[duration_index - 1];
        new_duration_rhythmic_position = &slices[slice_index].rhythmic_position +
            whole_notes_long(previous_duration.log2_duration,
            previous_duration.augmentation_dot_count);
    };
    let mut rest_rhythmic_position = &new_duration_rhythmic_position +
        whole_notes_long(new_duration.log2_duration, new_duration.augmentation_dot_count);
    loop
    {       
        let staff = &mut staves[insertion_address.staff_index];
        slice_index = staff.object_ranges[duration_index].slice_index; 
        if staff.durations.len() == duration_index
        {
            staff.durations.push(new_duration);
            break;
        }        
        if slices[slice_index].rhythmic_position == new_duration_rhythmic_position
        {
            staff.durations[duration_index] = new_duration;            
            break;
        }
        duration_index += 1;
    }
    reset_distance_from_previous_slice(device_context, slices, staves, staff_space_heights,
        slice_index);
    duration_index += 1;
    let mut rest_duration;    
    loop 
    {
        if duration_index == staves[insertion_address.staff_index].durations.len()
        {
            register_rhythmic_position(slices, staves, &mut slice_index, rest_rhythmic_position,
                insertion_address.staff_index, duration_index);
            reset_distance_from_previous_slice(device_context, slices, staves, staff_space_heights,
                slice_index);
            slice_index += 1;
            if slice_index < slices.len()
            {
                reset_distance_from_previous_slice(device_context, slices, staves,
                    staff_space_heights, slice_index);
            }
            return Address{staff_index: insertion_address.staff_index,
                address_type: AddressType::Duration{duration_index}};
        }
        let slice_index =
            staves[insertion_address.staff_index].object_ranges[duration_index].slice_index;
        if slices[slice_index].rhythmic_position < rest_rhythmic_position
        {
            let durations_in_slice_count = slices[slice_index].durations.len();
            if durations_in_slice_count == 1
            {
                slices.remove(slice_index);
                increment_slice_indices(slices, staves, slice_index, decrement);                
            }
            else
            {
                for duration_address_index in 0..durations_in_slice_count
                {
                    if slices[slice_index].durations[duration_address_index].staff_index ==
                        insertion_address.staff_index
                    {
                        slices[slice_index].durations.remove(duration_address_index);
                    }
                }
            }
            let staff = &mut staves[insertion_address.staff_index];
            staff.durations.remove(duration_index);
            staff.object_ranges.remove(duration_index);
        }
        else
        {
            rest_duration = &slices[slice_index].rhythmic_position - &rest_rhythmic_position;
            break;
        }
    }
    let mut denominator = rest_duration.denom().clone();
    let mut numerator = rest_duration.numer().clone();
    let mut division;
    let mut rest_log2_duration = 0;
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
            add_duration(device_context, slices, staves, staff_space_heights, &mut slice_index,
                old_rest_rhythmic_position, rest_log2_duration, None, 0,
                insertion_address.staff_index, duration_index);
            numerator = division.1;            
            duration_index += 1;
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
    Address{staff_index: insertion_address.staff_index,
        address_type: AddressType::Duration{duration_index}}
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

fn left_edge_to_origin_distance(staff_space_height: f32, duration: &Duration) -> i32
{
    if duration.log2_duration == 1
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
                    if let Selection::ActiveCursor{ref mut address,..} = &mut window_memory.selection
                    {
                        let template = ADD_CLEF_DIALOG_TEMPLATE.data.as_ptr();
                        let clef_selection = DialogBoxIndirectParamW(null_mut(), template as
                            *const DLGTEMPLATE, window_handle, Some(add_clef_dialog_proc), 0);
                        let baseline_offset;
                        let codepoint;
                        match (clef_selection & ADD_CLEF_SHAPE_BITS) as i32
                        {                                
                            IDC_ADD_CLEF_G =>
                            {
                                baseline_offset = -2;
                                codepoint =
                                match (clef_selection & ADD_CLEF_TRANSPOSITION_BITS) as i32
                                {                                        
                                    IDC_ADD_CLEF_15MA => 0xe054,
                                    IDC_ADD_CLEF_8VA => 0xe053,
                                    IDC_ADD_CLEF_NONE => 0xe050,
                                    IDC_ADD_CLEF_8VB => 0xe052,
                                    IDC_ADD_CLEF_15MB => 0xe051,
                                    _ => panic!("Unknown clef octave transposition.")
                                };
                            },
                            IDC_ADD_CLEF_C =>
                            {
                                baseline_offset = 0;
                                codepoint =
                                match (clef_selection & ADD_CLEF_TRANSPOSITION_BITS) as i32
                                {
                                    IDC_ADD_CLEF_NONE => 0xe05c,
                                    IDC_ADD_CLEF_8VB => 0xe05d,
                                    _ => panic!("Unknown clef octave transposition.")
                                };
                            },
                            IDC_ADD_CLEF_F =>
                            {
                                baseline_offset = 2;
                                codepoint =
                                match (clef_selection & ADD_CLEF_TRANSPOSITION_BITS) as i32
                                {                                        
                                    IDC_ADD_CLEF_15MA => 0xe066,
                                    IDC_ADD_CLEF_8VA => 0xe065,
                                    IDC_ADD_CLEF_NONE => 0xe062,
                                    IDC_ADD_CLEF_8VB => 0xe064,
                                    IDC_ADD_CLEF_15MB => 0xe063,
                                    _ => panic!("Unknown clef octave transposition.")
                                };
                            },
                            IDC_ADD_CLEF_UNPITCHED =>
                            {
                                baseline_offset = 0;
                                codepoint = 0xe069;
                            },
                            _ => return 0                                
                        };
                        let device_context = GetDC(window_handle);
                        let space_heights = staff_space_heights(&window_memory.staves,
                            &window_memory.staff_scales, window_memory.default_staff_space_height);
                        add_clef(device_context, &mut window_memory.slices, address,
                            &mut window_memory.staves, &space_heights,
                            &mut window_memory.header_clef_width, Clef{codepoint: codepoint,
                            steps_of_baseline_above_middle: baseline_offset, is_selected: false});
                        invalidate_work_region(window_handle);
                        return 0;
                    }
                }
                else if l_param == window_memory.add_staff_button_handle as isize
                {
                    let template = ADD_STAFF_DIALOG_TEMPLATE.data.as_ptr();
                    let result = DialogBoxIndirectParamW(null_mut(), template as
                        *const DLGTEMPLATE, window_handle, Some(add_staff_dialog_proc),
                        &window_memory.staff_scales as *const _ as isize);
                    invalidate_work_region(window_handle);
                    let line_count = (result & 0xffff) as u8;
                    if line_count == 0
                    {
                        return 0;
                    }
                    let vertical_center =
                    if window_memory.staves.len() == 0
                    {
                        110
                    }
                    else
                    {
                        window_memory.staves[window_memory.staves.len() - 1].vertical_center + 80
                    };
                    let staff_index = window_memory.staves.len(); 
                    window_memory.staves.push(
                        Staff{header_clef: None, object_ranges: vec![], durations: vec![],
                        line_thickness: 10.0 * BRAVURA_METADATA.staff_line_thickness,
                        vertical_center: vertical_center,
                        scale_index: ((result >> 32) & 0xffff) as usize, line_count: line_count});
                    register_rhythmic_position(&mut window_memory.slices, &mut window_memory.staves,
                        &mut 0, num_rational::Ratio::new(num_bigint::BigUint::new(vec![]),
                        num_bigint::BigUint::new(vec![1])), staff_index, 0);                                              
                    if 10 > window_memory.header_spacer
                    {
                        window_memory.header_spacer = 10;
                    }
                }
                0
            }
            else
            {
                DefWindowProcW(window_handle, u_msg, w_param, l_param)
            }
        },  
        WM_CTLCOLORSTATIC =>
        {
            GetStockObject(WHITE_BRUSH as i32) as isize
        },
        WM_GET_STAVES =>
        {
            *(w_param as *mut &mut Vec<Staff>) =
                &mut main_window_memory(window_handle).staves;
            0
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
                            Duration{log2_duration: SendMessageW(window_memory.duration_spin_handle,
                            UDM_GETPOS32, 0, 0) as i8, pitch: Some(pitch), augmentation_dot_count:
                            SendMessageW(window_memory.augmentation_dot_spin_handle, UDM_GETPOS32,
                            0, 0) as u8, is_selected: false}, address.clone());
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
                                let staff = &mut window_memory.staves[address.staff_index];
                                match address.address_type
                                {
                                    AddressType::Duration{duration_index} =>
                                    {
                                        if duration_index < staff.durations.len()
                                        {
                                            if let Some(pitch) =
                                                &mut staff.durations[duration_index].pitch
                                            {
                                                if *pitch > i8::min_value()
                                                {
                                                    *pitch -= 1;
                                                }
                                            }
                                        }
                                    },
                                    AddressType::HeaderClef => 
                                    {
                                        if let Some(clef) = &mut staff.header_clef
                                        {
                                            decrement_baseline(staff.line_count, clef);
                                        }
                                        else
                                        {
                                            return 0;
                                        }
                                    },
                                    AddressType::Object{range_index, object_index} =>
                                    {
                                        match &mut staff.object_ranges[range_index].
                                            objects[object_index].object.object_type
                                        {
                                            ObjectType::Clef(clef) =>
                                            {
                                                decrement_baseline(staff.line_count, clef);
                                            },
                                            _ => return 0
                                        }
                                    }
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
                        let staff = &window_memory.staves[address.staff_index];
                        if let Some(previous_address) =
                            previous_address(staff, &address.address_type)
                        {                            
                            if let AddressType::Duration{duration_index} = &previous_address
                            {
                                if let Some(pitch) = staff.durations[*duration_index].pitch
                                {
                                    *range_floor = pitch - 3;
                                }
                            }
                            address.address_type = previous_address;
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
                        let staff = &window_memory.staves[address.staff_index];
                        if let Some(next_address) = next_address(staff, &address.address_type)
                        {
                            if let AddressType::Duration{duration_index} = &next_address
                            {
                                if *duration_index < staff.durations.len()
                                {
                                    if let Some(pitch) = staff.durations[*duration_index].pitch
                                    {
                                        *range_floor = pitch - 3;
                                    }
                                }
                            }
                            address.address_type = next_address;
                            invalidate_work_region(window_handle);
                        }
                    }
                    invalidate_work_region(window_handle);
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
                            Duration{log2_duration: SendMessageW(window_memory.duration_spin_handle,
                            UDM_GETPOS32, 0, 0) as i8, pitch: None, augmentation_dot_count:
                            SendMessageW(window_memory.augmentation_dot_spin_handle, UDM_GETPOS32,
                            0, 0) as u8, is_selected: false}, &address);
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
                            if *range_floor < i8::max_value() - 7
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
                                let staff = &mut window_memory.staves[address.staff_index];
                                match address.address_type
                                {
                                    AddressType::Duration{duration_index} =>
                                    {
                                        if duration_index < staff.durations.len()
                                        {
                                            if let Some(pitch) =
                                                &mut staff.durations[duration_index].pitch
                                            {
                                                if *pitch < i8::max_value()
                                                {
                                                    *pitch += 1;
                                                }
                                            }
                                        }
                                    },
                                    AddressType::HeaderClef => 
                                    {
                                        if let Some(clef) = &mut staff.header_clef
                                        {
                                            increment_baseline(staff.line_count, clef);
                                        }
                                        else
                                        {
                                            return 0;
                                        }
                                    },
                                    AddressType::Object{range_index, object_index} =>
                                    {
                                        match &mut staff.object_ranges[range_index].
                                            objects[object_index].object.object_type
                                        {
                                            ObjectType::Clef(clef) =>
                                            {
                                                increment_baseline(staff.line_count, clef);
                                            },
                                            _ => return 0
                                        }
                                    }
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
                    &window_memory.slices, window_memory.header_spacer,
                    window_memory.header_clef_width, &mut window_memory.staves, space_height,
                    window_memory.system_left_edge, staff_index, click_x, click_y, zoom_factor);                            
                if let Some(address) = address
                {
                    window_memory.selection = Selection::Objects(
                        vec![Address{staff_index: staff_index, address_type: address}]);
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
                            if address.staff_index == staff_index
                            {
                                return 0;
                            }
                        }
                        _ => ()
                    }
                    match window_memory.ghost_cursor
                    {
                        Some(ref address) =>
                        {
                            if address.staff_index == staff_index
                            {
                                return 0;
                            }
                            invalidate_work_region(window_handle);
                        }
                        None => ()
                    }
                    let address_type =
                    if let Some(_) = staff.header_clef
                    {
                        AddressType::HeaderClef
                    }  
                    else if staff.object_ranges[0].objects.len() > 0
                    {
                        AddressType::Object{range_index: 0, object_index: 0}
                    }   
                    else
                    {
                        AddressType::Duration{duration_index: 0}
                    };  
                    window_memory.ghost_cursor =
                        Some(Address{staff_index: staff_index, address_type: address_type});          
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
            let mut client_rect = RECT{bottom: 0, left: 0, right: 0, top: 0};
            GetClientRect(window_handle, &mut client_rect);
            for staff in &window_memory.staves
            {
                let space_height = staff_space_height(staff, &window_memory.staff_scales,
                    window_memory.default_staff_space_height);
                let zoomed_font_set = staff_font_set(zoom_factor * space_height);
                for line_index in 0..staff.line_count
                {
                    draw_horizontal_line(device_context, window_memory.system_left_edge as f32,                        
                        client_rect.right as f32, y_of_steps_above_bottom_line(staff, space_height,
                        2 * line_index as i8), staff.line_thickness, zoom_factor);
                }
                let mut x = window_memory.system_left_edge + window_memory.header_spacer;
                let mut staff_middle_pitch = 6;
                if window_memory.header_clef_width > 0
                {
                    if let Some(clef) = &staff.header_clef
                    {
                        if clef.is_selected
                        {
                            SetTextColor(device_context, RED);
                        }
                        else
                        {
                            SetTextColor(device_context, BLACK);                        
                        }
                        draw_clef(device_context, zoomed_font_set.full_size, staff, space_height,
                            clef, x, &mut staff_middle_pitch, zoom_factor);                
                    }
                    x += window_memory.header_clef_width + window_memory.header_spacer;
                }
                let mut slice_index = 0;
                for index in 0..staff.object_ranges.len()
                {
                    let object_range = &staff.object_ranges[index];
                    while slice_index <= object_range.slice_index
                    {
                        x += window_memory.slices[slice_index].distance_from_previous_slice;
                        slice_index += 1;
                    }
                    for range_object in &object_range.objects
                    {
                        range_object.draw_with_highlight(device_context, &zoomed_font_set, staff,
                            space_height, x - range_object.distance_to_next_slice,
                            &mut staff_middle_pitch, zoom_factor);
                    }
                    if index < staff.durations.len()
                    {
                        staff.durations[index].draw_with_highlight(device_context, &zoomed_font_set,
                            staff, space_height, x, &mut staff_middle_pitch, zoom_factor);
                    }
                }
            }            
            if let Some(address) = &window_memory.ghost_cursor
            {
                SelectObject(device_context, GRAY_PEN.unwrap() as *mut winapi::ctypes::c_void);
                SelectObject(device_context, GRAY_BRUSH.unwrap() as *mut winapi::ctypes::c_void);
                let cursor_x = cursor_x(&window_memory.slices, window_memory.header_spacer,
                    window_memory.header_clef_width, &window_memory.staves,
                    window_memory.system_left_edge, address);
                let staff = &window_memory.staves[address.staff_index];
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
                let cursor_x = cursor_x(&window_memory.slices, window_memory.header_spacer,
                    window_memory.header_clef_width, &window_memory.staves,
                    window_memory.system_left_edge, address);
                let staff = &window_memory.staves[address.staff_index];   
                let staff_space_height = staff_space_height(staff, &window_memory.staff_scales,
                    window_memory.default_staff_space_height);           
                let steps_of_floor_above_bottom_line = range_floor - bottom_line_pitch(
                    staff.line_count, staff_middle_pitch_at_address(staff, &address.address_type));                    
                let range_indicator_bottom = y_of_steps_above_bottom_line(staff, staff_space_height,
                    steps_of_floor_above_bottom_line);
                let range_indicator_top = y_of_steps_above_bottom_line(staff, staff_space_height,
                    steps_of_floor_above_bottom_line + 6);
                let range_indicator_right_edge = cursor_x as f32 + staff_space_height;
                draw_horizontal_line(device_context, cursor_x as f32, range_indicator_right_edge,
                    range_indicator_bottom, staff.line_thickness, zoom_factor);
                draw_horizontal_line(device_context, cursor_x as f32, range_indicator_right_edge,
                    range_indicator_top, staff.line_thickness, zoom_factor);
                let leger_left_edge = cursor_x as f32 - staff_space_height;
                let cursor_bottom =
                if steps_of_floor_above_bottom_line < 0
                {
                    for line_index in steps_of_floor_above_bottom_line / 2..0
                    {
                        draw_horizontal_line(device_context, leger_left_edge, cursor_x as f32,
                            y_of_steps_above_bottom_line(staff, staff_space_height, 2 * line_index),
                            staff.line_thickness, zoom_factor);
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
                            staff.line_thickness, zoom_factor);
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

fn next_address(staff: &Staff, address: &AddressType) -> Option<AddressType>
{
    let new_range_index;
    let new_object_index;
    match address
    {
        AddressType::Duration{duration_index} =>
        {
            new_range_index = duration_index + 1;   
            if new_range_index == staff.object_ranges.len()
            {
                return None;
            }
            new_object_index = 0;   
        },
        AddressType::HeaderClef =>
        {
            new_range_index = 0;
            new_object_index = 0;
        },
        AddressType::Object{range_index, object_index} =>
        {
            new_range_index = *range_index;
            new_object_index = *object_index + 1;
        }
    }    
    if new_object_index < staff.object_ranges[new_range_index].objects.len()
    {
        return Some(AddressType::Object{range_index: new_range_index,
            object_index: new_object_index});
    } 
    Some(AddressType::Duration{duration_index: new_range_index})
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

fn previous_address(staff: &Staff, address: &AddressType) -> Option<AddressType>
{
    let current_range_index;
    let current_object_index;
    match address
    {
        AddressType::Duration{duration_index} =>
        {
            current_range_index = *duration_index;  
            current_object_index = staff.object_ranges[current_range_index].objects.len(); 
        },
        AddressType::HeaderClef => return None,
        AddressType::Object{range_index, object_index} =>
        {
            current_range_index = *range_index;
            current_object_index = *object_index;
        }
    }
    if current_object_index > 0
    {
        return Some(AddressType::Object{range_index: current_range_index,
            object_index: current_object_index - 1});
    }
    if current_range_index > 0
    {
        return Some(AddressType::Duration{duration_index: current_range_index - 1});
    }
    if let Some(_) = staff.header_clef
    {
        return Some(AddressType::HeaderClef);
    }
    None
}

fn register_rhythmic_position(slices: &mut Vec<RhythmicSlice>, staves: &mut Vec<Staff>,
    slice_index: &mut usize, rhythmic_position: num_rational::Ratio<num_bigint::BigUint>,
    staff_index: usize, duration_index: usize)
{
    loop
    {
        if *slice_index == slices.len() ||
            slices[*slice_index].rhythmic_position > rhythmic_position
        {
            increment_slice_indices(slices, staves, *slice_index, increment);
            slices.insert(*slice_index, RhythmicSlice{durations: vec![],
                rhythmic_position: rhythmic_position, distance_from_previous_slice: 0});
            break;
        }
        if slices[*slice_index].rhythmic_position == rhythmic_position
        {
            break;
        }
        *slice_index += 1;
    }
    staves[staff_index].object_ranges.insert(duration_index,
        ObjectRange{slice_index: *slice_index, objects: vec![]});
    slices[*slice_index].durations.push(DurationAddress{staff_index: staff_index,
        duration_index: duration_index});
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

fn reset_distance_from_previous_slice(device_context: HDC, slices: &mut Vec<RhythmicSlice>,
    staves: &mut Vec<Staff>, staff_space_heights: &Vec<f32>, slice_index: usize)
{
    let mut distance_from_previous_slice = 0;
    for duration_address in &slices[slice_index].durations
    {
        let staff = &mut staves[duration_address.staff_index];
        let space_height = staff_space_heights[duration_address.staff_index];
        let font_set = staff_font_set(space_height);
        let mut range_width =
        if duration_address.duration_index < staff.durations.len()
        {
            left_edge_to_origin_distance(space_height,
                &staff.durations[duration_address.duration_index])
        }
        else
        {
            0
        };
        let objects = &mut staff.object_ranges[duration_address.duration_index].objects;
        if objects.len() > 0
        {
            for object in objects.into_iter().rev()
            {
                range_width +=
                match &object.object.object_type
                {
                    ObjectType::Clef(clef) => character_width(device_context,
                        font_set.two_thirds_size, clef.codepoint as u32),
                    ObjectType::KeySignature{accidental_count, flats} =>
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
                };
                object.distance_to_next_slice = range_width;
            }
        }
        if duration_address.duration_index > 0
        {
            let previous_duration = &staff.durations[duration_address.duration_index - 1];
            range_width += previous_duration.augmentation_dot_count as i32 *
                ((space_height * DISTANCE_BETWEEN_AUGMENTATION_DOTS).round() as i32 +
                character_width(device_context, font_set.full_size, 0xe1e7));
            range_width = std::cmp::max(range_width, duration_width(previous_duration));
            range_width += character_width(device_context, font_set.full_size,
                duration_codepoint(previous_duration) as u32) -
                left_edge_to_origin_distance(space_height, previous_duration);
            for slice_index in &staff.object_ranges[duration_address.duration_index - 1].
                slice_index + 1..slice_index
            {
                range_width -= slices[slice_index].distance_from_previous_slice;
            }
        }
        distance_from_previous_slice = std::cmp::max(distance_from_previous_slice, range_width);
    }
    slices[slice_index].distance_from_previous_slice = distance_from_previous_slice;
}

fn rest_codepoint(log2_duration: i8) -> u16
{
    (0xe4e3 - log2_duration as i32) as u16
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
    baseline_pitch - clef.steps_of_baseline_above_middle
}

fn staff_middle_pitch_at_address(staff: &Staff, address: &AddressType) -> i8
{
    let index;
    match address
    {
        AddressType::Object{range_index, object_index} =>
        {
            for index in (0..*object_index).rev()
            {
                if let ObjectType::Clef(clef) =
                    &staff.object_ranges[*range_index].objects[index].object.object_type
                {
                    return staff_middle_pitch(clef);
                }
            }
            if *range_index == 0
            {
                if let Some(clef) = &staff.header_clef
                {
                    return staff_middle_pitch(clef);
                }
                return DEFAULT_STAFF_MIDDLE_PITCH;
            }
            index = *range_index - 1;
        },
        AddressType::Duration{duration_index} =>
        {
            index = *duration_index;
        },
        AddressType::HeaderClef =>
        {
            return DEFAULT_STAFF_MIDDLE_PITCH;
        }
    }
    for index in (0..=index).rev()
    {
        for range_object in staff.object_ranges[index].objects.iter().rev()
        {
            if let ObjectType::Clef(clef) = &range_object.object.object_type
            {
                return staff_middle_pitch(clef);
            }
        }
    }
    if let Some(clef) = &staff.header_clef
    {
        return staff_middle_pitch(clef);
    }
    DEFAULT_STAFF_MIDDLE_PITCH
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
    VerticalInterval{top: horizontal_line_vertical_bounds(y_of_steps_above_bottom_line(
        staff, space_height, 2 * (staff.line_count as i8 - 1)),
        staff.line_thickness, zoom_factor).top, bottom: horizontal_line_vertical_bounds(
        y_of_steps_above_bottom_line(staff, space_height, 0),
        staff.line_thickness, zoom_factor).bottom}
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