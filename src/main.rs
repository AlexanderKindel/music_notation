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

const WHOLE_NOTE_WIDTH: i32 = 100;
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

struct Point<T>
{
    x: T,
    y: T
}

enum StaffObjectType
{
    DurationObject
    {
        //Denotes the power of two times the duration of a whole note of the object's duration.
        log2_duration: isize,
        steps_above_c4: Option<i8>,
        whole_notes_from_staff_start: num_rational::BigRational
    },
    Clef
    {
        font_codepoint: u16,
        staff_spaces_of_baseline_above_bottom_line: u8,
        steps_of_bottom_staff_line_above_c4: i8
    }
}

struct StaffObject
{
    distance_from_staff_start: i32,
    object_type: StaffObjectType,
    is_selected: bool
}

struct Staff
{
    line_count: u8,
    line_thickness_in_staff_spaces: f32,
    left_edge: i32,
    bottom_line_vertical_center: i32,
    height: i32,
    contents: Vec<StaffObject>
}

struct ObjectAddress
{
    staff_index: usize,
    object_index: usize
}

struct SizedMusicFont
{
    font: HFONT,
    number_of_staves_with_size: u8 
}

enum Selection<'a>
{
    ActiveCursor(ObjectAddress, i8),
    Objects(Vec<&'a mut StaffObject>),
    None
}

struct MainWindowMemory<'a>
{
    sized_music_fonts: HashMap<i32, SizedMusicFont>,
    staves: Vec<Staff>,
    ghost_cursor: Option<ObjectAddress>,
    selection: Selection<'a>,
    add_staff_button_handle: HWND,
    add_clef_button_handle: HWND,
    duration_display_handle: HWND,
    duration_spin_handle: HWND,
    default_beam_spacing: f32,
    default_beam_thickness: f32,
    default_leger_line_thickness: f32,
    default_leger_line_extension: f32,
    default_staff_line_thickness: f32,
    default_stem_thickness: f32,
    default_black_notehead_stem_up_se: Point<f32>,
    default_black_notehead_stem_down_nw: Point<f32>,
    default_half_notehead_stem_up_se: Point<f32>,
    default_half_notehead_stem_down_nw: Point<f32>
}

fn get_character_width(device_context: HDC, window_memory: *const MainWindowMemory,
    staff_height: i32, font_codepoint: u32) -> i32
{
    unsafe
    {
        SelectObject(device_context, (*window_memory).sized_music_fonts.get(
            &staff_height).unwrap().font as *mut winapi::ctypes::c_void);
        let mut abc_array: [ABC; 1] = [ABC{abcA: 0, abcB: 0, abcC: 0}];
        GetCharABCWidthsW(device_context, font_codepoint,
            font_codepoint + 1, abc_array.as_mut_ptr());
        abc_array[0].abcB as i32
    }
}

fn get_duration_width(log2_duration: isize) -> i32
{
    (WHOLE_NOTE_WIDTH as f32 * DURATION_RATIO.powi(-log2_duration as i32)).round() as i32
}

fn draw_note(device_context: HDC, window_memory: *const MainWindowMemory, staff: &Staff,
    steps_of_bottom_staff_line_above_c4: i8, log2_duration: isize, steps_above_c4: i8,
    distance_from_staff_start: i32)
{
    let space_count = staff.line_count as i32 - 1;
    let steps_above_bottom_line = steps_above_c4 - steps_of_bottom_staff_line_above_c4; 
    let notehead_x = staff.left_edge + distance_from_staff_start;
    let notehead_y = staff.bottom_line_vertical_center -
        (staff.height * steps_above_bottom_line as i32) / (2 * (staff.line_count as i32 - 1));
    let get_flagless_up_stem_ne_coordinates = |stem_right_edge_relative_to_notehead: f32|
    {
        let stem_right_edge =
            notehead_x + staff.to_logical_units(stem_right_edge_relative_to_notehead);
        let mut stem_top_steps_above_bottom_line = steps_above_bottom_line as i32 + 7;
        if stem_top_steps_above_bottom_line < space_count
        {
            stem_top_steps_above_bottom_line = space_count;
        }
        let mut stem_top = staff.get_line_vertical_center_relative_to_bottom_line(
            stem_top_steps_above_bottom_line / 2);
        if stem_top_steps_above_bottom_line % 2 != 0
        {
            stem_top -= staff.height / (2 * space_count);
        }
        Point{x: stem_right_edge, y: stem_top}
    };
    unsafe
    {
        let draw_up_stem = |stem_ne: &Point<i32>,
            stem_right_edge_relative_to_notehead_y: f32, stem_left_edge: i32|
        {
            Rectangle(device_context, stem_left_edge, stem_ne.y, stem_ne.x,
                notehead_y - staff.to_logical_units(stem_right_edge_relative_to_notehead_y));
        };
        let draw_flagless_up_stem = |stem_se_relative_to_notehead: &Point<f32>|
        {
            let stem_ne_coordinates =
                get_flagless_up_stem_ne_coordinates(stem_se_relative_to_notehead.x);
            draw_up_stem(
                &stem_ne_coordinates, stem_se_relative_to_notehead.y, stem_ne_coordinates.x -
                staff.get_logical_line_thickness((*window_memory).default_stem_thickness));
        };
        let get_flagless_down_stem_se_coordinates = |stem_left_edge_relative_to_notehead: f32|
        {
            let stem_left_edge =
                notehead_x + staff.to_logical_units(stem_left_edge_relative_to_notehead);
            let mut stem_bottom_steps_above_bottom_line = steps_above_bottom_line as i32 - 7;
            if stem_bottom_steps_above_bottom_line > space_count
            {
                stem_bottom_steps_above_bottom_line = space_count;
            }
            let mut stem_bottom = staff.get_line_vertical_center_relative_to_bottom_line(
                stem_bottom_steps_above_bottom_line / 2);
            let remainder = stem_bottom_steps_above_bottom_line % 2;
            if remainder != 0
            {
                stem_bottom -= remainder * staff.height / (2 * space_count);
            }
            Point{x: stem_left_edge, y: stem_bottom}
        };                            
        let notehead_codepoint =
        match log2_duration
        {
            1 => 0xe0a0,
            0 => 0xe0a2,
            -1 =>
            {                                    
                if space_count > steps_above_bottom_line as i32
                {
                    draw_flagless_up_stem(&(*window_memory).default_half_notehead_stem_up_se);                                        
                }
                else
                {
                    let stem_nw_relative_to_notehead =
                        &(*window_memory).default_half_notehead_stem_down_nw;
                    let stem_nw_coordinates =
                        get_flagless_down_stem_se_coordinates(stem_nw_relative_to_notehead.x);
                    Rectangle(device_context, stem_nw_coordinates.x,
                        notehead_y - staff.to_logical_units(stem_nw_relative_to_notehead.y),
                        stem_nw_coordinates.x + staff.get_logical_line_thickness(
                        (*window_memory).default_stem_thickness), stem_nw_coordinates.y);
                }
                0xe0a3
            },
            -2 =>
            {
                if space_count > steps_above_bottom_line as i32 
                {
                    draw_flagless_up_stem(&(*window_memory).default_black_notehead_stem_up_se);                                        
                }
                else
                {
                    let stem_nw_relative_to_notehead =
                        &(*window_memory).default_black_notehead_stem_down_nw;
                    let stem_nw_coordinates =
                        get_flagless_down_stem_se_coordinates(stem_nw_relative_to_notehead.x);
                    Rectangle(device_context, stem_nw_coordinates.x,
                        notehead_y - staff.to_logical_units(stem_nw_relative_to_notehead.y),
                        stem_nw_coordinates.x + staff.get_logical_line_thickness(
                        (*window_memory).default_stem_thickness), stem_nw_coordinates.y);
                }
                0xe0a4
            },
            -3 =>
            {
                if space_count > steps_above_bottom_line as i32 
                {
                    let stem_se_relative_to_notehead =
                        &(*window_memory).default_black_notehead_stem_up_se;
                    let stem_ne_coordinates =
                        get_flagless_up_stem_ne_coordinates(stem_se_relative_to_notehead.x);
                    let stem_left_edge = stem_ne_coordinates.x - staff.get_logical_line_thickness(
                        (*window_memory).default_stem_thickness);
                    TextOutW(device_context, stem_left_edge, stem_ne_coordinates.y,
                        vec![0xe240, 0].as_ptr(), 1);
                    draw_up_stem(&stem_ne_coordinates, stem_se_relative_to_notehead.y,
                        stem_left_edge);                                        
                }
                else
                {
                    let stem_nw_relative_to_notehead =
                        &(*window_memory).default_black_notehead_stem_down_nw;
                    let stem_se_coordinates =
                        get_flagless_down_stem_se_coordinates(stem_nw_relative_to_notehead.x);
                    TextOutW(device_context, stem_se_coordinates.x, stem_se_coordinates.y,
                        vec![0xe241, 0].as_ptr(), 1);
                    Rectangle(device_context, stem_se_coordinates.x,
                        notehead_y - staff.to_logical_units(stem_nw_relative_to_notehead.y),
                        stem_se_coordinates.x + staff.get_logical_line_thickness(
                        (*window_memory).default_stem_thickness), stem_se_coordinates.y);
                }
                0xe0a4
            },
            _ =>
            {
                if space_count > steps_above_bottom_line as i32 
                {
                    let stem_se_relative_to_notehead =
                        &(*window_memory).default_black_notehead_stem_up_se;
                    let stem_right_edge =
                        notehead_x + staff.to_logical_units(stem_se_relative_to_notehead.x);
                    let mut stem_top_steps_above_bottom_line = steps_above_bottom_line as i32 + 7;
                    if stem_top_steps_above_bottom_line < space_count
                    {
                        stem_top_steps_above_bottom_line = space_count;
                    }
                    let stem_left_edge = stem_right_edge -
                        staff.get_logical_line_thickness((*window_memory).default_stem_thickness);
                    let extra_step =
                    if stem_top_steps_above_bottom_line % 2 != 0
                    {
                        staff.height / (2 * space_count)
                    }
                    else
                    {
                        0
                    };
                    let stem_top = staff.get_line_vertical_center_relative_to_bottom_line(
                        stem_top_steps_above_bottom_line / 2) - extra_step;                                       
                    TextOutW(device_context, stem_left_edge, stem_top, vec![0xe242, 0].as_ptr(), 1);
                    let flag_spacing = (*window_memory).default_beam_spacing +
                        (*window_memory).default_beam_thickness;
                    let mut offset_from_first_flag = 0.0;
                    for _ in 0..-log2_duration - 4
                    {         
                        offset_from_first_flag -= flag_spacing;
                        TextOutW(device_context, stem_left_edge, stem_top + staff.to_logical_units(
                            offset_from_first_flag), vec![0xe250, 0].as_ptr(), 1);                                            
                    }                                        
                    draw_up_stem(&Point{x: stem_right_edge, y: stem_top + staff.to_logical_units(
                        offset_from_first_flag)}, stem_se_relative_to_notehead.y, stem_left_edge);                                        
                }
                else
                {
                    let stem_nw_relative_to_notehead =
                        &(*window_memory).default_black_notehead_stem_down_nw;
                    let stem_left_edge =
                        notehead_x + staff.to_logical_units(stem_nw_relative_to_notehead.x);
                    let mut stem_bottom_steps_above_bottom_line =
                        steps_above_bottom_line as i32 - 7;
                    if stem_bottom_steps_above_bottom_line > space_count
                    {
                        stem_bottom_steps_above_bottom_line = space_count;
                    }
                    let extra_step = -(stem_bottom_steps_above_bottom_line % 2) * staff.height /
                        (2 * space_count);
                    let stem_bottom = staff.get_line_vertical_center_relative_to_bottom_line(
                        stem_bottom_steps_above_bottom_line / 2) + extra_step;                                       
                    TextOutW(device_context, stem_left_edge, stem_bottom, vec![0xe243, 0].as_ptr(),
                        1);
                    let flag_spacing = (*window_memory).default_beam_spacing +
                        (*window_memory).default_beam_thickness;
                    let mut offset_from_first_flag = 0.0;
                    for _ in 0..-log2_duration - 4
                    {      
                        offset_from_first_flag += flag_spacing;
                        TextOutW(device_context, stem_left_edge, stem_bottom +
                            staff.to_logical_units(offset_from_first_flag),
                            vec![0xe251, 0].as_ptr(), 1);
                    }
                    Rectangle(device_context, stem_left_edge,
                        notehead_y - staff.to_logical_units(stem_nw_relative_to_notehead.y),
                        stem_left_edge + staff.get_logical_line_thickness(
                        (*window_memory).default_stem_thickness),
                        stem_bottom + staff.to_logical_units(offset_from_first_flag));
                }
                0xe0a4
            }
        };
        let get_leger_line_metrics = || -> (i32, i32, i32)
        {
            let extension = staff.to_logical_units((*window_memory).default_leger_line_extension);
            let left_edge = notehead_x - extension;
            let right_edge = notehead_x + get_character_width(device_context, window_memory,
                staff.height, notehead_codepoint as u32) + extension;
            (staff.get_logical_line_thickness((*window_memory).default_leger_line_thickness),
                left_edge, right_edge)
        };
        if steps_above_bottom_line < -1
        {
            let (leger_line_thickness, left_edge, right_edge) = get_leger_line_metrics();
            let lines_above_bottom_line = steps_above_bottom_line as i32 / 2;
            staff.draw_lines(device_context, lines_above_bottom_line, -lines_above_bottom_line,
                leger_line_thickness, left_edge, right_edge);
        } 
        else if steps_above_bottom_line >= 2 * staff.line_count as i8
        {
            let (leger_line_thickness, left_edge, right_edge) = get_leger_line_metrics();
            staff.draw_lines(device_context, staff.line_count as i32,
                steps_above_bottom_line as i32 / 2 - space_count, leger_line_thickness, left_edge,
                right_edge);                                
        }                            
        TextOutW(device_context, notehead_x, notehead_y, vec![notehead_codepoint, 0].as_ptr(), 1);  
    }
}

fn draw_clef(device_context: HDC, staff: &Staff, distance_from_staff_start: i32,
    font_codepoint: u16, staff_spaces_of_baseline_above_bottom_line: u8)
{
    unsafe
    {
        TextOutW(device_context,
            staff.left_edge + distance_from_staff_start, staff.bottom_line_vertical_center -
            (staff.height * staff_spaces_of_baseline_above_bottom_line as i32) /
            (staff.line_count as i32 - 1), vec![font_codepoint, 0].as_ptr(), 1);
    }
}

impl Staff
{
    fn to_logical_units(&self, staff_spaces: f32) -> i32
    {
        ((self.height as f32 * staff_spaces) / (self.line_count - 1) as f32).round() as i32
    }
    fn get_logical_line_thickness(&self, line_thickness_in_staff_spaces: f32) -> i32
    {
        let line_thickness = self.to_logical_units(line_thickness_in_staff_spaces);
        if line_thickness == 0
        {
            1
        }
        else
        {
            line_thickness
        }
    }
    fn get_line_vertical_center_relative_to_bottom_line(&self, spaces_above_bottom_line: i32) -> i32
    {
        self.bottom_line_vertical_center -
            (self.height * spaces_above_bottom_line) / (self.line_count as i32 - 1)
    }
    fn draw_lines(&self, device_context: HDC, spaces_of_lowest_line_above_bottom_line: i32,
        line_count: i32, line_thickness: i32, left_edge: i32, right_edge: i32)
    {
        let line_offset = line_thickness / 2 + line_thickness % 2;        
        for spaces_above_bottom_line in spaces_of_lowest_line_above_bottom_line..
            spaces_of_lowest_line_above_bottom_line + line_count
        {
            let current_line_bottom = self.get_line_vertical_center_relative_to_bottom_line(
                spaces_above_bottom_line) + line_offset;
            unsafe
            {
                Rectangle(device_context, left_edge, current_line_bottom - line_thickness,
                    right_edge, current_line_bottom);
            }
        }
    }
    fn draw_object(&self, device_context: HDC, window_memory: *const MainWindowMemory,
        steps_of_bottom_staff_line_above_c4: i8, object_index: usize) -> i8
    {
        let object = &self.contents[object_index];
        match object.object_type
        {
            StaffObjectType::DurationObject{log2_duration, steps_above_c4,..} =>
            {
                match steps_above_c4
                {
                    Some(steps_above_c4) => draw_note(device_context, window_memory, self,
                        steps_of_bottom_staff_line_above_c4, log2_duration, steps_above_c4,
                        object.distance_from_staff_start),
                    None =>
                    {
                        let spaces_above_bottom_line =
                        if log2_duration == 0
                        {
                            if self.line_count == 1
                            {
                                0
                            }
                            else
                            {
                                self.line_count / 2 + self.line_count % 2
                            }
                        }
                        else
                        {                        
                            self.line_count / 2 + self.line_count % 2 - 1
                        };
                        unsafe
                        {
                            TextOutW(device_context,
                                self.left_edge + object.distance_from_staff_start,
                                self.get_line_vertical_center_relative_to_bottom_line(
                                spaces_above_bottom_line as i32),
                                vec![get_rest_codepoint(log2_duration), 0].as_ptr(), 1);
                        }
                    }
                }
                steps_of_bottom_staff_line_above_c4
            },
            StaffObjectType::Clef{font_codepoint, staff_spaces_of_baseline_above_bottom_line,
                steps_of_bottom_staff_line_above_c4} =>
            {
                if object_index > 0
                {
                    unsafe
                    {
                        SelectObject(device_context, (*window_memory).sized_music_fonts.get(
                            &((2 * self.height) / 3)).unwrap().font as *mut winapi::ctypes::c_void);
                        draw_clef(device_context, self, object.distance_from_staff_start,
                            font_codepoint, staff_spaces_of_baseline_above_bottom_line);
                        SelectObject(device_context, (*window_memory).sized_music_fonts.get(
                            &self.height).unwrap().font as *mut winapi::ctypes::c_void);
                    }
                }
                else
                {
                    draw_clef(device_context, self, object.distance_from_staff_start,
                        font_codepoint, staff_spaces_of_baseline_above_bottom_line);
                }
                steps_of_bottom_staff_line_above_c4
            }
        }
    }
    fn get_bottom_line_pitch(&self, contents_index: usize) -> i8
    {
        for index in (0..contents_index).rev()
        {    
            if let StaffObjectType::Clef{steps_of_bottom_staff_line_above_c4,..} =
                self.contents[index].object_type
            {
                return steps_of_bottom_staff_line_above_c4;
            }
        }
        2
    }  
    fn object_width(&self, device_context: HDC, window_memory: *const MainWindowMemory,
        staff_height: i32, object_index: usize) -> i32
    {
        match self.contents[object_index].object_type
        {
            StaffObjectType::DurationObject{log2_duration, steps_above_c4,..} =>
            {
                let codepoint =
                match steps_above_c4
                {
                    Some(_) => get_notehead_codepoint(log2_duration),
                    None => get_rest_codepoint(log2_duration)
                };
                get_character_width(device_context, window_memory, staff_height, codepoint as u32)
            },
            StaffObjectType::Clef{font_codepoint,..} => 
            {
                let height =
                if object_index > 0
                {
                    (2 * staff_height) / 3
                }
                else
                {
                    staff_height
                };
                get_character_width(device_context, window_memory, height, font_codepoint as u32)
            }           
        }
    }  
    fn insert_object(&mut self, device_context: HDC, window_memory: *const MainWindowMemory,
        object: StaffObject, object_index: usize)
    {
        self.contents.insert(object_index, object);
        match self.contents[object_index].object_type
        {
            StaffObjectType::DurationObject{log2_duration,..} =>
            {
                if object_index > 0
                {
                    let previous_object_index = object_index - 1;
                    self.contents[object_index].distance_from_staff_start =
                        self.contents[previous_object_index].distance_from_staff_start +
                        self.object_width(device_context, window_memory, self.height,
                        previous_object_index);
                    if let StaffObjectType::DurationObject{log2_duration,..} =
                        self.contents[previous_object_index].object_type
                    {
                        self.contents[object_index].distance_from_staff_start +=
                            get_duration_width(log2_duration);
                    }
                }
                else
                {
                    self.contents[object_index].distance_from_staff_start = 0;
                }
                let duration_width = get_duration_width(log2_duration);
                let character_width = get_character_width(device_context, window_memory,
                    self.height, get_notehead_codepoint(log2_duration) as u32);
                let object_right_edge =
                    self.contents[object_index].distance_from_staff_start + character_width;
                let next_durationless_object_index = object_index + 1;
                let mut next_durationless_object_left_edge = object_right_edge;
                for i in next_durationless_object_index..self.contents.len()
                {            
                    if let StaffObjectType::DurationObject{..} = self.contents[i].object_type
                    {
                        let duration_right_edge = object_right_edge + duration_width;
                        let duration_width_overflow =
                            next_durationless_object_left_edge - duration_right_edge;
                        let remaining_object_offset =
                        if duration_width_overflow > 0
                        {
                            next_durationless_object_left_edge -
                                self.contents[i].distance_from_staff_start
                        }
                        else
                        {
                            for j in next_durationless_object_index..i
                            {
                                self.contents[j].distance_from_staff_start -=
                                    duration_width_overflow;
                            }
                            duration_right_edge - self.contents[i].distance_from_staff_start                            
                        };
                        for j in i..self.contents.len()
                        {
                            self.contents[j].distance_from_staff_start += remaining_object_offset;
                        }    
                        return;
                    } 
                    self.contents[i].distance_from_staff_start = next_durationless_object_left_edge;
                    next_durationless_object_left_edge +=
                        self.object_width(device_context, window_memory, self.height, i);                             
                }    
                let duration_width_overflow = next_durationless_object_left_edge -
                    object_right_edge - duration_width;
                if duration_width_overflow < 0
                {
                    for i in object_index + 1..self.contents.len()
                    {
                        self.contents[i].distance_from_staff_start -= duration_width_overflow;
                    }
                }
            },
            _ =>
            {
                for i in (0..object_index).rev()
                {
                    if let StaffObjectType::DurationObject{log2_duration,..} =
                        self.contents[i].object_type
                    {
                        let first_durationless_object_index = i + 1;
                        let duration_width = get_duration_width(log2_duration);
                        let previous_duration_object_right_edge =
                            self.contents[i].distance_from_staff_start +
                            get_character_width(device_context, window_memory, self.height,
                            get_notehead_codepoint(log2_duration) as u32);
                        let mut next_object_left_edge = previous_duration_object_right_edge;
                        for j in first_durationless_object_index..self.contents.len()
                        {            
                            if let StaffObjectType::DurationObject{..} =
                                self.contents[j].object_type
                            {
                                let duration_width_overflow = next_object_left_edge -
                                    previous_duration_object_right_edge - duration_width;
                                if duration_width_overflow > 0
                                {
                                    for k in j..self.contents.len()
                                    {
                                        self.contents[k].distance_from_staff_start +=
                                            duration_width_overflow;
                                    }
                                }
                                else
                                {
                                    for k in first_durationless_object_index..j
                                    {
                                        self.contents[k].distance_from_staff_start -=
                                            duration_width_overflow;
                                    }
                                }
                                return;
                            } 
                            self.contents[j].distance_from_staff_start = next_object_left_edge;
                            next_object_left_edge +=
                                self.object_width(device_context, window_memory, self.height, j);                             
                        }    
                        let duration_width_overflow = next_object_left_edge -
                            previous_duration_object_right_edge - duration_width;
                        if duration_width_overflow < 0
                        {
                            for j in first_durationless_object_index..self.contents.len()
                            {
                                self.contents[j].distance_from_staff_start -=
                                    duration_width_overflow;
                            }
                        }
                        return;
                    }
                }
                let mut next_object_left_edge = 0;  
                for i in 0..=object_index
                {
                    self.contents[i].distance_from_staff_start = next_object_left_edge;
                    next_object_left_edge +=
                        self.object_width(device_context, window_memory, self.height, i);
                }
                let next_object_index = object_index + 1;
                if next_object_index < self.contents.len()
                {
                    let remaining_object_offset = next_object_left_edge -
                        self.contents[next_object_index].distance_from_staff_start;
                    for index in next_object_index..self.contents.len()
                    {
                        self.contents[index].distance_from_staff_start += remaining_object_offset;
                    }
                }
            }
        }
    }
    fn remove_object(&mut self, device_context: HDC, window_memory: *const MainWindowMemory,
        object_index: usize)
    {
        self.contents.remove(object_index);
        for i in (0..object_index).rev()
        {
            if let StaffObjectType::DurationObject{log2_duration,..} = self.contents[i].object_type
            {
                let first_durationless_object_index = i + 1;
                let duration_width = get_duration_width(log2_duration);
                let previous_duration_object_right_edge =
                    self.contents[i].distance_from_staff_start +
                    get_character_width(device_context, window_memory, self.height,
                    get_notehead_codepoint(log2_duration) as u32);
                let mut next_object_left_edge = previous_duration_object_right_edge;
                for j in first_durationless_object_index..self.contents.len()
                {            
                    if let StaffObjectType::DurationObject{..} = self.contents[j].object_type
                    {
                        let default_position =
                            previous_duration_object_right_edge + duration_width;
                        let offset_to_default =
                            default_position - self.contents[j].distance_from_staff_start;
                        for k in j..self.contents.len()
                        {
                            self.contents[k].distance_from_staff_start += offset_to_default;
                        }
                        let duration_width_overflow = next_object_left_edge - default_position;
                        if duration_width_overflow > 0
                        {
                            for k in j..self.contents.len()
                            {
                                self.contents[k].distance_from_staff_start +=
                                    duration_width_overflow;
                            }
                        }
                        else
                        {
                            for k in first_durationless_object_index..j
                            {
                                self.contents[k].distance_from_staff_start -=
                                    duration_width_overflow;
                            }
                        }
                        return;
                    } 
                    self.contents[j].distance_from_staff_start = next_object_left_edge;
                    next_object_left_edge +=
                        self.object_width(device_context, window_memory, self.height, j);                             
                }    
                let duration_width_overflow =
                    next_object_left_edge - previous_duration_object_right_edge - duration_width;
                if duration_width_overflow < 0
                {
                    for index in first_durationless_object_index..self.contents.len()
                    {
                        self.contents[index].distance_from_staff_start -= duration_width_overflow;
                    }
                }
                return;
            }
        }
        let mut next_object_left_edge = 0;  
        for i in 0..object_index
        {
            self.contents[i].distance_from_staff_start = next_object_left_edge;
            next_object_left_edge +=
                self.object_width(device_context, window_memory, self.height, i);
        }
        if object_index < self.contents.len()
        {
            let remaining_object_offset =
                next_object_left_edge - self.contents[object_index].distance_from_staff_start;
            for i in object_index..self.contents.len()
            {
                self.contents[i].distance_from_staff_start += remaining_object_offset;
            }
        }
    }
}

fn wide_char_string(value: &str) -> Vec<u16>
{    
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(value).encode_wide().chain(std::iter::once(0)).collect()
}

fn invalidate_client_rect(window_handle: HWND)
{
    unsafe
    {
        let mut client_rect: RECT = std::mem::uninitialized();
        GetClientRect(window_handle, &mut client_rect);
        InvalidateRect(window_handle, &client_rect, TRUE);
    }
}

fn get_whole_notes_from_start(staff: &Staff, object_index: usize) ->
    num_rational::BigRational
{    
    for index in (0..object_index).rev()
    {
        if let StaffObjectType::DurationObject{log2_duration, ref whole_notes_from_staff_start,..} =
            staff.contents[index].object_type
        {
            let whole_notes_long =
            if log2_duration >= 0
            {
                num_rational::Ratio::new(num_bigint::BigInt::from(2u32.pow(log2_duration as u32)),
                    1.to_bigint().unwrap())
            }
            else
            {
                num_rational::Ratio::new(1.to_bigint().unwrap(),
                    num_bigint::BigInt::from(2u32.pow(-log2_duration as u32)))
            };                
            return whole_notes_from_staff_start.clone() + whole_notes_long;
        }
    }
    num_rational::Ratio::new(0.to_bigint().unwrap(), 1.to_bigint().unwrap())
}

fn cancel_selection(window_handle: HWND)
{
    unsafe
    {
        let window_memory =
            GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
        match (*window_memory).selection
        {
            Selection::ActiveCursor(..) =>
            {
                invalidate_client_rect(window_handle);
                EnableWindow((*window_memory).add_clef_button_handle, FALSE);
            }
            Selection::Objects(ref mut objects) =>
            {
                for object in objects
                {
                    object.is_selected = false;
                }
                invalidate_client_rect(window_handle);
                EnableWindow((*window_memory).add_clef_button_handle, FALSE);
            },
            Selection::None => ()
        }        
        (*window_memory).selection = Selection::None;
    }
}

fn get_notehead_codepoint(log2_duration: isize) -> u16
{
    match log2_duration
    {
        1 => 0xe0a0,
        0 => 0xe0a2,
        -1 => 0xe0a3,
        _ => 0xe0a4
    }
}

fn get_rest_codepoint(log2_duration: isize) -> u16
{
    (0xe4e3 - log2_duration) as u16
}

fn get_selected_duration(window_memory: *const MainWindowMemory) -> isize
{
    unsafe
    {
        1 - (SendMessageW((*window_memory).duration_spin_handle, UDM_GETPOS, 0, 0) & 0xff)
    }
}

fn add_sized_music_font(window_memory: *mut MainWindowMemory, size: i32)
{
    unsafe
    {
        match (*window_memory).sized_music_fonts.get_mut(&size)
        {
            Some(sized_font) =>
            {
                sized_font.number_of_staves_with_size +=1;
            }
            None =>
            {
                (*window_memory).sized_music_fonts.insert(size, SizedMusicFont{
                    font: CreateFontW(-size, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    wide_char_string("Bravura").as_ptr()), number_of_staves_with_size: 1});
            }
        };
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
                        Selection::ActiveCursor(ref address,..) =>
                        {
                            let template =
                            match &ADD_CLEF_DIALOG_TEMPLATE
                            {
                                Some(template) => template.as_ptr(),
                                None => panic!("Add clef dialog template not found.")
                            };
                            let clef_selection = DialogBoxIndirectParamW(null_mut(), template as
                                *const DLGTEMPLATE, window_handle, Some(add_clef_dialog_proc), 0);
                            let (codepoint, baseline_offset, steps_of_bottom_line_above_c4) =
                            match (clef_selection & ADD_CLEF_SHAPE_BITS) as i32
                            {                                
                                IDC_ADD_CLEF_G =>
                                {
                                    let (codepoint, steps_of_bottom_line_above_c4) =
                                    match (clef_selection & ADD_CLEF_TRANSPOSITION_BITS) as i32
                                    {                                        
                                        IDC_ADD_CLEF_15MA => (0xe054, 16),
                                        IDC_ADD_CLEF_8VA => (0xe053, 9),
                                        IDC_ADD_CLEF_NONE => (0xe050, 2),
                                        IDC_ADD_CLEF_8VB => (0xe052, -5),
                                        IDC_ADD_CLEF_15MB => (0xe051, -12),
                                        _ => panic!("Unknown clef octave transposition.")
                                    };
                                    (codepoint, 1, steps_of_bottom_line_above_c4)
                                },
                                IDC_ADD_CLEF_C =>
                                {
                                    let (codepoint, steps_of_bottom_line_above_c4) =
                                    match (clef_selection & ADD_CLEF_TRANSPOSITION_BITS) as i32
                                    {
                                        IDC_ADD_CLEF_NONE => (0xe05c, -4),
                                        IDC_ADD_CLEF_8VB => (0xe05d, -11),
                                        _ => panic!("Unknown clef octave transposition.")
                                    };
                                    (codepoint, 2, steps_of_bottom_line_above_c4)
                                },
                                IDC_ADD_CLEF_F =>
                                {
                                    let (codepoint, steps_of_bottom_line_above_c4) =
                                    match (clef_selection & ADD_CLEF_TRANSPOSITION_BITS) as i32
                                    {                                        
                                        IDC_ADD_CLEF_15MA => (0xe066, 4),
                                        IDC_ADD_CLEF_8VA => (0xe065, -3),
                                        IDC_ADD_CLEF_NONE => (0xe062, -10),
                                        IDC_ADD_CLEF_8VB => (0xe064, -17),
                                        IDC_ADD_CLEF_15MB => (0xe063, -24),
                                        _ => panic!("Unknown clef octave transposition.")
                                    };
                                    (codepoint, 3, steps_of_bottom_line_above_c4)
                                },
                                IDC_ADD_CLEF_UNPITCHED =>
                                {
                                    (0xe069, 2, 2)
                                },
                                _ => return 0                                
                            };
                            let staff = &mut(*window_memory).staves[address.staff_index];
                            let device_context = GetDC(window_handle);
                            fn remove_old_clef_clef(device_context: HDC,
                                window_memory: *const MainWindowMemory, staff: &mut Staff,
                                cursor_index: usize) -> usize
                            {
                                if cursor_index < staff.contents.len()
                                {                                    
                                    if let StaffObjectType::Clef{..} =
                                        staff.contents[cursor_index].object_type
                                    {
                                        staff.remove_object(device_context, window_memory,
                                            cursor_index);                                        
                                        return cursor_index;
                                    }
                                }
                                if cursor_index > 0
                                {
                                    let clef_index = cursor_index - 1;
                                    if let StaffObjectType::Clef{..} =
                                        staff.contents[clef_index].object_type
                                    {
                                        staff.remove_object(device_context, window_memory,
                                            clef_index);
                                        return clef_index;
                                    }
                                }
                                cursor_index
                            };
                            let new_clef_index = remove_old_clef_clef(device_context,
                                window_memory, staff, address.object_index);
                            staff.insert_object(device_context, window_memory,
                                StaffObject{distance_from_staff_start: 0,
                                object_type: StaffObjectType::Clef{font_codepoint: codepoint as u16,
                                staff_spaces_of_baseline_above_bottom_line: baseline_offset,
                                steps_of_bottom_staff_line_above_c4: steps_of_bottom_line_above_c4},
                                is_selected: true}, new_clef_index);
                            (*window_memory).selection =
                                Selection::Objects(vec![&mut staff.contents[new_clef_index]]);
                            invalidate_client_rect(window_handle);
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
                        (*window_memory).staves[
                            (*window_memory).staves.len() - 1].bottom_line_vertical_center + 80
                    };
                    let height = 40;
                    (*window_memory).staves.push(Staff{line_count: 5,
                        line_thickness_in_staff_spaces:
                        (*window_memory).default_staff_line_thickness, left_edge: 20,
                        bottom_line_vertical_center: bottom_line_y, height: height,
                        contents: Vec::new()});
                    invalidate_client_rect(window_handle);
                    add_sized_music_font(window_memory, height);
                    add_sized_music_font(window_memory, (2 * height) / 3);
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
                65..=71 =>
                {
                    let window_memory =
                        GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
                    if let Selection::ActiveCursor(ref address, range_floor) =
                        (*window_memory).selection
                    {            
                        let log2_duration = get_selected_duration(window_memory);
                        let staff = &mut(*window_memory).staves[address.staff_index];
                        let device_context = GetDC(window_handle);
                        let whole_notes_from_start =
                            get_whole_notes_from_start(staff, address.object_index);
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
                        staff.insert_object(device_context, window_memory, StaffObject{object_type:
                            StaffObjectType::DurationObject{log2_duration: log2_duration,
                            steps_above_c4: Some(pitch), whole_notes_from_staff_start:
                            whole_notes_from_start}, distance_from_staff_start: 0,
                            is_selected: false}, address.object_index);
                        (*window_memory).selection =
                            Selection::ActiveCursor(ObjectAddress{staff_index: address.staff_index,
                            object_index: address.object_index + 1}, pitch - 3);
                        invalidate_client_rect(window_handle);
                    }
                    0
                },
                VK_DOWN =>
                {
                    let window_memory =
                        GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
                    match (*window_memory).selection
                    {
                        Selection::Objects(ref mut objects) =>
                        {
                            for object in objects
                            {
                                match object.object_type
                                {
                                    StaffObjectType::Clef{
                                        ref mut staff_spaces_of_baseline_above_bottom_line, 
                                        ref mut steps_of_bottom_staff_line_above_c4,..} => 
                                    {
                                        *staff_spaces_of_baseline_above_bottom_line -= 1;
                                        *steps_of_bottom_staff_line_above_c4 += 2;
                                    },
                                    StaffObjectType::DurationObject{mut steps_above_c4,..} =>
                                    {
                                        if let Some(ref mut steps_above_c4) = steps_above_c4
                                        {
                                            *steps_above_c4 -= 1
                                        }
                                    }
                                }
                            }
                            invalidate_client_rect(window_handle);                        
                        },
                        Selection::ActiveCursor(ref _address, ref mut range_floor) =>
                        {
                            *range_floor -= 7;
                            invalidate_client_rect(window_handle);
                        },
                        Selection::None => ()
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
                        Selection::ActiveCursor(ref mut address, ref mut range_floor) =>
                        {
                            if address.object_index > 0
                            {                             
                                address.object_index -= 1;
                                *range_floor = (*window_memory).staves[address.staff_index].
                                    get_bottom_line_pitch(address.object_index) + 1;  
                                invalidate_client_rect(window_handle);
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
                        Selection::ActiveCursor(ref mut address, ref mut range_floor) =>
                        {
                            let staff = &(*window_memory).staves[address.staff_index];
                            if address.object_index < staff.contents.len()
                            {        
                                if let StaffObjectType::Clef{
                                    steps_of_bottom_staff_line_above_c4,..} =
                                    staff.contents[address.object_index].object_type
                                {
                                    *range_floor = steps_of_bottom_staff_line_above_c4 + 1;
                                }
                                address.object_index += 1;  
                                invalidate_client_rect(window_handle);
                            }
                        }
                        _ => ()
                    }
                    0
                },
                VK_SPACE =>
                {
                    let window_memory =
                        GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
                    if let Selection::ActiveCursor(ref address, range_floor) =
                        (*window_memory).selection
                    {
                        (*window_memory).staves[address.staff_index].insert_object(
                            GetDC(window_handle), window_memory,
                            StaffObject{distance_from_staff_start: 0,
                            object_type: StaffObjectType::DurationObject{
                            log2_duration: get_selected_duration(window_memory),
                            whole_notes_from_staff_start: get_whole_notes_from_start(
                            &(*window_memory).staves[address.staff_index], address.object_index),
                            steps_above_c4: None}, is_selected: false}, address.object_index);
                        (*window_memory).selection =
                            Selection::ActiveCursor(ObjectAddress{staff_index: address.staff_index,
                            object_index: address.object_index + 1}, range_floor);
                        invalidate_client_rect(window_handle);
                    }
                    0
                },
                VK_UP =>
                {
                    let window_memory =
                        GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
                    match (*window_memory).selection
                    {
                        Selection::Objects(ref mut objects) =>
                        {
                            for object in objects
                            {
                                match object.object_type
                                {
                                    StaffObjectType::Clef{
                                        ref mut staff_spaces_of_baseline_above_bottom_line, 
                                        ref mut steps_of_bottom_staff_line_above_c4,..} =>
                                    {
                                        *staff_spaces_of_baseline_above_bottom_line += 1;
                                        *steps_of_bottom_staff_line_above_c4 -= 2;
                                    },
                                    StaffObjectType::DurationObject{mut steps_above_c4,..} =>
                                    {
                                        if let Some(ref mut steps_above_c4) = steps_above_c4
                                        {
                                            *steps_above_c4 += 1
                                        }
                                    }
                                }
                            }
                            invalidate_client_rect(window_handle);
                        },
                        Selection::ActiveCursor(ref _address, ref mut range_floor) =>
                        {
                            *range_floor += 7;
                            invalidate_client_rect(window_handle);
                        },
                        Selection::None => ()
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
                    (*window_memory).selection = Selection::ActiveCursor(ObjectAddress{staff_index:
                        ghost_address.staff_index, object_index: ghost_address.object_index}, 3);
                    let ref staff = (*window_memory).staves[ghost_address.staff_index];
                    InvalidateRect(window_handle, &RECT{left: staff.left_edge,
                        top: staff.bottom_line_vertical_center - staff.height,
                        right: WHOLE_NOTE_WIDTH, bottom: staff.bottom_line_vertical_center}, TRUE);
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
                    staff.bottom_line_vertical_center - staff.height <= cursor_y &&
                    cursor_y <= staff.bottom_line_vertical_center
                {
                    match (*window_memory).selection
                    {
                        Selection::ActiveCursor(ref address,..) =>
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
                                top: old_staff.bottom_line_vertical_center - old_staff.height,
                                right: WHOLE_NOTE_WIDTH,
                                bottom: old_staff.bottom_line_vertical_center}, TRUE);
                        }
                        None => ()
                    }
                    (*window_memory).ghost_cursor =
                        Some(ObjectAddress{staff_index: staff_index, object_index: 0});
                    InvalidateRect(window_handle, &RECT{left: staff.left_edge,
                        top: staff.bottom_line_vertical_center - staff.height,
                        right: WHOLE_NOTE_WIDTH, bottom: staff.bottom_line_vertical_center}, TRUE);
                    return 0;
                }
            }
            match (*window_memory).ghost_cursor
            {
                Some(ref address) =>
                {                     
                    let staff = &(*window_memory).staves[address.staff_index];
                    InvalidateRect(window_handle, &RECT{left: staff.left_edge,
                        top: staff.bottom_line_vertical_center - staff.height,
                        right: WHOLE_NOTE_WIDTH, bottom: staff.bottom_line_vertical_center}, TRUE);
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
                else if new_position > 11
                {
                    wide_char_string("1024th")
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
                let line_thickness =
                    staff.get_logical_line_thickness(staff.line_thickness_in_staff_spaces);                	        
                let mut right_edge = staff.left_edge + WHOLE_NOTE_WIDTH;
                if staff.contents.len() > 0
                {
                    let last_object_index = staff.contents.len() - 1;
                    right_edge += staff.contents[last_object_index].distance_from_staff_start +
                        staff.object_width(device_context, window_memory, staff.height,
                        last_object_index);
                }
                let original_device_context = SaveDC(device_context);
                SelectObject(device_context, GetStockObject(BLACK_PEN as i32));
                SelectObject(device_context, GetStockObject(BLACK_BRUSH as i32));
                staff.draw_lines(device_context, 0, staff.line_count as i32, line_thickness,
                    staff.left_edge, right_edge);
                SelectObject(device_context, (*window_memory).sized_music_fonts.get(
                    &staff.height).unwrap().font as *mut winapi::ctypes::c_void);
                let mut steps_of_bottom_line_above_c4 = 2;
                for index in 0..staff.contents.len()
                {
                    if staff.contents[index].is_selected
                    {
                        SetTextColor(device_context, RED.unwrap());
                        steps_of_bottom_line_above_c4 = staff.draw_object(device_context,
                            window_memory, steps_of_bottom_line_above_c4, index);
                        SetTextColor(device_context, BLACK.unwrap());
                    }
                    else
                    {
                        steps_of_bottom_line_above_c4 = staff.draw_object(device_context,
                            window_memory, steps_of_bottom_line_above_c4, index);
                    }                    
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
                    Rectangle(device_context, staff.left_edge, staff.bottom_line_vertical_center -
                        staff.height, staff.left_edge + 1, staff.bottom_line_vertical_center);
                    SelectObject(device_context, original_pen);
                    SelectObject(device_context, original_brush);
                },
                None => ()
            }
            match (*window_memory).selection
            {
                Selection::ActiveCursor(ref address, range_floor) =>
                {
                    let original_pen = SelectObject(device_context,
                        RED_PEN.unwrap() as *mut winapi::ctypes::c_void);
                    let original_brush = SelectObject(device_context,
                        RED_BRUSH.unwrap() as *mut winapi::ctypes::c_void);
                    let staff = &(*window_memory).staves[address.staff_index];
                    let mut cursor_left_edge = staff.left_edge;
                    if address.object_index > 0
                    {
                        let previous_object_index = address.object_index - 1;
                        cursor_left_edge +=
                            staff.contents[previous_object_index].distance_from_staff_start +
                            staff.object_width(device_context, window_memory, staff.height,
                            previous_object_index);
                    }
                    let bottom_line_pitch = staff.get_bottom_line_pitch(address.object_index);                    
                    let steps_of_floor_above_bottom_line = range_floor - bottom_line_pitch;
                    let spaces_of_floor_above_bottom_line =
                        steps_of_floor_above_bottom_line as i32 / 2;
                    let mut range_indicator_bottom =
                        staff.get_line_vertical_center_relative_to_bottom_line(
                        spaces_of_floor_above_bottom_line);
                    let mut range_indicator_top =
                        staff.get_line_vertical_center_relative_to_bottom_line(
                        spaces_of_floor_above_bottom_line + 3);
                    let remainder = steps_of_floor_above_bottom_line as i32 % 2;
                    if remainder != 0
                    {
                        let half_space = remainder * staff.height /
                            (2 * (staff.line_count as i32 - 1));
                        range_indicator_bottom -= half_space; 
                        range_indicator_top -= half_space;
                    } 
                    let range_indicator_left_edge = cursor_left_edge + 5;
                    Rectangle(device_context, cursor_left_edge, range_indicator_bottom - 1,
                        range_indicator_left_edge, range_indicator_bottom);
                    Rectangle(device_context, cursor_left_edge, range_indicator_top - 1,
                        range_indicator_left_edge, range_indicator_top);
                    let leger_left_edge = cursor_left_edge - 5;
                    let cursor_bottom =
                    if steps_of_floor_above_bottom_line < 0
                    {
                        staff.draw_lines(device_context, spaces_of_floor_above_bottom_line,
                            -spaces_of_floor_above_bottom_line, 1, leger_left_edge,
                            cursor_left_edge);
                        range_indicator_bottom
                    }
                    else
                    {
                        staff.bottom_line_vertical_center
                    };
                    let steps_of_ceiling_above_top_line =
                        steps_of_floor_above_bottom_line + 8 - 2 * staff.line_count as i8;
                    let cursor_top =
                    if steps_of_ceiling_above_top_line > 0
                    {
                        staff.draw_lines(device_context, staff.line_count as i32,
                            steps_of_ceiling_above_top_line as i32 / 2, 1, leger_left_edge,
                            cursor_left_edge);
                        range_indicator_top
                    }
                    else
                    {
                        staff.bottom_line_vertical_center - staff.height
                    };
                    Rectangle(device_context, cursor_left_edge, cursor_top, cursor_left_edge + 1,
                        cursor_bottom);
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

fn json_value_to_point(value: &serde_json::value::Value) -> Point<f32>
{
    let array = value.as_array().unwrap();
    Point{x: array[0].as_f64().unwrap() as f32, y: array[1].as_f64().unwrap() as f32}
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
    let device_context = GetDC(main_window_handle);
    SetBkMode(device_context, TRANSPARENT as i32);
    SetTextAlign(device_context, TA_BASELINE);
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
    SendMessageW(duration_spin_handle, UDM_SETRANGE, 0, 11 << 16);
    let bravura_metadata_file =
        File::open("bravura_metadata.json").expect("Failed to open bravura_metadata.json");    
    let bravura_metadata: serde_json::Value = 
        serde_json::from_reader(bravura_metadata_file).unwrap();
    let engraving_defaults = &bravura_metadata["engravingDefaults"];
    let glyphs_with_anchors = &bravura_metadata["glyphsWithAnchors"];
    let black_notehead_anchors = &glyphs_with_anchors["noteheadBlack"];
    let half_notehead_anchors = &glyphs_with_anchors["noteheadHalf"];
    let main_window_memory = MainWindowMemory{sized_music_fonts: HashMap::new(),
        staves: Vec::new(), ghost_cursor: None, selection: Selection::None,
        add_staff_button_handle: add_staff_button_handle,
        add_clef_button_handle: add_clef_button_handle,
        duration_display_handle: duration_display_handle,
        duration_spin_handle: duration_spin_handle,
        default_beam_spacing: engraving_defaults["beamSpacing"].as_f64().unwrap() as f32,
        default_beam_thickness: engraving_defaults["beamThickness"].as_f64().unwrap() as f32,
        default_leger_line_extension:
        engraving_defaults["legerLineExtension"].as_f64().unwrap() as f32,
        default_leger_line_thickness:
        engraving_defaults["legerLineThickness"].as_f64().unwrap() as f32,
        default_staff_line_thickness:
        engraving_defaults["staffLineThickness"].as_f64().unwrap() as f32,
        default_stem_thickness:
        engraving_defaults["stemThickness"].as_f64().unwrap() as f32,
        default_black_notehead_stem_up_se: json_value_to_point(&black_notehead_anchors["stemUpSE"]),
        default_black_notehead_stem_down_nw:
        json_value_to_point(&black_notehead_anchors["stemDownNW"]),
        default_half_notehead_stem_up_se: json_value_to_point(&half_notehead_anchors["stemUpSE"]),
        default_half_notehead_stem_down_nw:
        json_value_to_point(&half_notehead_anchors["stemDownNW"])};
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
        let mut message: MSG = MSG{hwnd: null_mut(), message: 0, wParam: 0, lParam: 0, time: 0,
            pt: POINT{x: 0, y: 0}};        
        while GetMessageW(&mut message, main_window_handle, 0, 0) > 0
        {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}