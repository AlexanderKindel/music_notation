extern crate num_bigint;
extern crate num_integer;
extern crate num_rational;
extern crate winapi;

mod init;

use init::*;
use num_integer::Integer;
use std::collections::HashMap;
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

const WHOLE_NOTE_WIDTH: u16 = 90;
const DURATION_RATIO: f32 = 0.61803399;
const DURATIONS: [&str; 4] = ["double whole", "whole", "half", "quarter"];

static mut GRAY_PEN: Option<HPEN> = None;
static mut GRAY_BRUSH: Option<HBRUSH> = None;
static mut RED_PEN: Option<HPEN> = None;
static mut RED_BRUSH: Option<HBRUSH> = None;

struct ObjectAddress
{
    staff_index: usize,
    object_index: usize
}

struct Duration
{
    //Denotes the power of two times the duration of a whole note of the object's duration.
    log2_duration: isize,  
    object_index: usize,
    steps_above_c4: Option<i8>
}

enum StaffObjectType
{
    Duration(usize),
    Clef
    {
        distance_from_staff_start: i32,
        font_codepoint: u16,
        staff_spaces_of_baseline_above_bottom_line: u8,
        steps_of_bottom_staff_line_above_c4: i8
    }
}

struct StaffObject
{
    object_type: StaffObjectType,
    is_selected: bool
}

struct Staff
{
    contents: Vec<StaffObject>,
    durations: Vec<Duration>,
    system_slice_indices: Vec<usize>,
    line_thickness_in_staff_spaces: f32,
    left_edge: i32,
    bottom_line_vertical_center: i32,
    height: u16,
    line_count: u8    
}

struct SizedMusicFont
{
    font: HFONT,
    number_of_staves_with_size: u8 
}

struct DurationAddress
{
    staff_index: usize,
    duration_index: usize
}

struct SystemSlice
{
    durations_at_position: Vec<DurationAddress>,
    whole_notes_from_start: num_rational::Ratio<num_bigint::BigUint>,
    distance_from_system_start: i32        
}

enum Selection
{
    ActiveCursor(ObjectAddress, i8),
    Objects(Vec<ObjectAddress>),
    None
}

struct MainWindowMemory
{
    sized_music_fonts: HashMap<u16, SizedMusicFont>,
    staves: Vec<Staff>,
    system_slices: Vec<SystemSlice>,
    ghost_cursor: Option<ObjectAddress>,
    selection: Selection,
    add_staff_button_handle: HWND,
    add_clef_button_handle: HWND,
    duration_display_handle: HWND,
    duration_spin_handle: HWND
}

fn get_character_width(device_context: HDC, music_fonts: &HashMap<u16, SizedMusicFont>,
    staff_height: u16, font_codepoint: u32) -> i32
{
    unsafe
    {
        SelectObject(device_context,
            music_fonts.get(&staff_height).unwrap().font as *mut winapi::ctypes::c_void);
        let mut abc_array: [ABC; 1] = [ABC{abcA: 0, abcB: 0, abcC: 0}];
        GetCharABCWidthsW(device_context, font_codepoint,
            font_codepoint + 1, abc_array.as_mut_ptr());
        abc_array[0].abcB as i32
    }
}

fn log2_duration_to_duration_width(log2_duration: isize) -> i32
{
    (WHOLE_NOTE_WIDTH as f32 * DURATION_RATIO.powi(-log2_duration as i32)).round() as i32
}

fn draw_note(device_context: HDC, music_fonts: &HashMap<u16, SizedMusicFont>, staff: &Staff,
    steps_of_bottom_staff_line_above_c4: i8, log2_duration: isize, steps_above_c4: i8,
    distance_from_staff_start: i32)
{
    let space_count = staff.line_count as i32 - 1;
    let steps_above_bottom_line = steps_above_c4 - steps_of_bottom_staff_line_above_c4; 
    let notehead_x = staff.left_edge + distance_from_staff_start;
    let notehead_y = staff.bottom_line_vertical_center - (staff.height as i32 *
        steps_above_bottom_line as i32) / (2 * (staff.line_count as i32 - 1));
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
            stem_top -= staff.height as i32 / (2 * space_count);
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
                staff.get_logical_line_thickness(BRAVURA_METADATA.stem_thickness));
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
                stem_bottom -= remainder * staff.height as i32 / (2 * space_count);
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
                    draw_flagless_up_stem(&BRAVURA_METADATA.half_notehead_stem_up_se);                                        
                }
                else
                {
                    let stem_nw_relative_to_notehead = &BRAVURA_METADATA.half_notehead_stem_down_nw;
                    let stem_nw_coordinates =
                        get_flagless_down_stem_se_coordinates(stem_nw_relative_to_notehead.x);
                    Rectangle(device_context, stem_nw_coordinates.x,
                        notehead_y - staff.to_logical_units(stem_nw_relative_to_notehead.y),
                        stem_nw_coordinates.x + staff.get_logical_line_thickness(
                        BRAVURA_METADATA.stem_thickness), stem_nw_coordinates.y);
                }
                0xe0a3
            },
            -2 =>
            {
                if space_count > steps_above_bottom_line as i32 
                {
                    draw_flagless_up_stem(&BRAVURA_METADATA.black_notehead_stem_up_se);                                        
                }
                else
                {
                    let stem_nw_relative_to_notehead =
                        &BRAVURA_METADATA.black_notehead_stem_down_nw;
                    let stem_nw_coordinates =
                        get_flagless_down_stem_se_coordinates(stem_nw_relative_to_notehead.x);
                    Rectangle(device_context, stem_nw_coordinates.x,
                        notehead_y - staff.to_logical_units(stem_nw_relative_to_notehead.y),
                        stem_nw_coordinates.x + staff.get_logical_line_thickness(
                        BRAVURA_METADATA.stem_thickness), stem_nw_coordinates.y);
                }
                0xe0a4
            },
            -3 =>
            {
                if space_count > steps_above_bottom_line as i32 
                {
                    let stem_se_relative_to_notehead = &BRAVURA_METADATA.black_notehead_stem_up_se;
                    let stem_ne_coordinates =
                        get_flagless_up_stem_ne_coordinates(stem_se_relative_to_notehead.x);
                    let stem_left_edge = stem_ne_coordinates.x -
                        staff.get_logical_line_thickness(BRAVURA_METADATA.stem_thickness);
                    TextOutW(device_context, stem_left_edge, stem_ne_coordinates.y,
                        vec![0xe240, 0].as_ptr(), 1);
                    draw_up_stem(&stem_ne_coordinates, stem_se_relative_to_notehead.y,
                        stem_left_edge);                                        
                }
                else
                {
                    let stem_nw_relative_to_notehead =
                        &BRAVURA_METADATA.black_notehead_stem_down_nw;
                    let stem_se_coordinates =
                        get_flagless_down_stem_se_coordinates(stem_nw_relative_to_notehead.x);
                    TextOutW(device_context, stem_se_coordinates.x, stem_se_coordinates.y,
                        vec![0xe241, 0].as_ptr(), 1);
                    Rectangle(device_context, stem_se_coordinates.x,
                        notehead_y - staff.to_logical_units(stem_nw_relative_to_notehead.y),
                        stem_se_coordinates.x + staff.get_logical_line_thickness(
                        BRAVURA_METADATA.stem_thickness), stem_se_coordinates.y);
                }
                0xe0a4
            },
            _ =>
            {
                if space_count > steps_above_bottom_line as i32 
                {
                    let stem_se_relative_to_notehead = &BRAVURA_METADATA.black_notehead_stem_up_se;
                    let stem_right_edge =
                        notehead_x + staff.to_logical_units(stem_se_relative_to_notehead.x);
                    let mut stem_top_steps_above_bottom_line = steps_above_bottom_line as i32 + 7;
                    if stem_top_steps_above_bottom_line < space_count
                    {
                        stem_top_steps_above_bottom_line = space_count;
                    }
                    let stem_left_edge = stem_right_edge -
                        staff.get_logical_line_thickness(BRAVURA_METADATA.stem_thickness);
                    let extra_step =
                    if stem_top_steps_above_bottom_line % 2 != 0
                    {
                        staff.height as i32 / (2 * space_count)
                    }
                    else
                    {
                        0
                    };
                    let stem_top = staff.get_line_vertical_center_relative_to_bottom_line(
                        stem_top_steps_above_bottom_line / 2) - extra_step;                                       
                    TextOutW(device_context, stem_left_edge, stem_top, vec![0xe242, 0].as_ptr(), 1);
                    let flag_spacing =
                        BRAVURA_METADATA.beam_spacing + BRAVURA_METADATA.beam_thickness;
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
                        &BRAVURA_METADATA.black_notehead_stem_down_nw;
                    let stem_left_edge =
                        notehead_x + staff.to_logical_units(stem_nw_relative_to_notehead.x);
                    let mut stem_bottom_steps_above_bottom_line =
                        steps_above_bottom_line as i32 - 7;
                    if stem_bottom_steps_above_bottom_line > space_count
                    {
                        stem_bottom_steps_above_bottom_line = space_count;
                    }
                    let extra_step = -(stem_bottom_steps_above_bottom_line % 2) *
                        staff.height as i32 / (2 * space_count);
                    let stem_bottom = staff.get_line_vertical_center_relative_to_bottom_line(
                        stem_bottom_steps_above_bottom_line / 2) + extra_step;                                       
                    TextOutW(device_context, stem_left_edge, stem_bottom, vec![0xe243, 0].as_ptr(),
                        1);
                    let flag_spacing =
                        BRAVURA_METADATA.beam_spacing + BRAVURA_METADATA.beam_thickness;
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
                        stem_left_edge +
                        staff.get_logical_line_thickness(BRAVURA_METADATA.stem_thickness),
                        stem_bottom + staff.to_logical_units(offset_from_first_flag));
                }
                0xe0a4
            }
        };
        let get_leger_line_metrics = || -> (i32, i32, i32)
        {
            let extension = staff.to_logical_units(BRAVURA_METADATA.leger_line_extension);
            let left_edge = notehead_x - extension;
            let right_edge = notehead_x + extension + get_character_width(device_context,
                music_fonts, staff.height, notehead_codepoint as u32);
            (staff.get_logical_line_thickness(BRAVURA_METADATA.leger_line_thickness), left_edge,
                right_edge)
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
            (staff.height as i32 * staff_spaces_of_baseline_above_bottom_line as i32) /
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
            (self.height as i32 * spaces_above_bottom_line) / (self.line_count as i32 - 1)
    }
    fn draw_lines(&self, device_context: HDC, spaces_of_lowest_line_above_bottom_line: i32,
        line_count: i32, line_thickness: i32, left_edge: i32, right_edge: i32)
    {
        let line_offset = line_thickness / 2;        
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
    fn draw_object(&self, device_context: HDC, music_fonts: &HashMap<u16, SizedMusicFont>,
        system_slices: &Vec<SystemSlice>, steps_of_bottom_staff_line_above_c4: i8,
        object_index: usize) -> i8
    {
        let object = &self.contents[object_index];
        match object.object_type
        {
            StaffObjectType::Duration(duration_index) =>
            {
                let duration = &self.durations[duration_index];
                match duration.steps_above_c4
                {
                    Some(steps_above_c4) => draw_note(device_context, music_fonts, self,
                        steps_of_bottom_staff_line_above_c4, duration.log2_duration, steps_above_c4,
                        self.distance_from_start(system_slices, object_index)),
                    None =>
                    {
                        let spaces_above_bottom_line =
                        if duration.log2_duration == 0
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
                            TextOutW(device_context, self.left_edge +
                                self.distance_from_start(system_slices, object_index),
                                self.get_line_vertical_center_relative_to_bottom_line(
                                spaces_above_bottom_line as i32),
                                vec![get_rest_codepoint(duration.log2_duration), 0].as_ptr(), 1);
                        }
                    }
                }
                steps_of_bottom_staff_line_above_c4
            },
            StaffObjectType::Clef{distance_from_staff_start, font_codepoint,
                staff_spaces_of_baseline_above_bottom_line, steps_of_bottom_staff_line_above_c4} =>
            {
                if object_index > 0
                {
                    unsafe
                    {
                        SelectObject(device_context, music_fonts.get(
                            &((2 * self.height) / 3)).unwrap().font as *mut winapi::ctypes::c_void);
                        draw_clef(device_context, self, distance_from_staff_start, font_codepoint,
                            staff_spaces_of_baseline_above_bottom_line);
                        SelectObject(device_context, music_fonts.get(
                            &self.height).unwrap().font as *mut winapi::ctypes::c_void);
                    }
                }
                else
                {
                    draw_clef(device_context, self, distance_from_staff_start, font_codepoint,
                        staff_spaces_of_baseline_above_bottom_line);
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
    fn remove_object(&mut self, device_context: HDC, music_fonts: &HashMap<u16, SizedMusicFont>,
        system_slices: &mut Vec<SystemSlice>, object_index: usize)
    {
        self.contents.remove(object_index);
        for i in (0..object_index).rev()
        {
            if let StaffObjectType::Duration(duration_index) = self.contents[i].object_type
            {
                let log2_duration = self.durations[duration_index].log2_duration;
                let first_durationless_object_index = i + 1;
                let duration_width = log2_duration_to_duration_width(log2_duration);
                let previous_duration_object_right_edge = self.object_right_edge(device_context,
                    music_fonts, system_slices, i);
                let mut next_object_left_edge = previous_duration_object_right_edge;
                for j in first_durationless_object_index..self.contents.len()
                {            
                    if let StaffObjectType::Duration{..} = self.contents[j].object_type
                    {
                        let default_position =
                            previous_duration_object_right_edge + duration_width;
                        let offset_to_default =
                            default_position - self.distance_from_start(system_slices, j);                        
                        for k in j..self.contents.len()
                        {
                            self.offset_distance_from_start(system_slices, k, offset_to_default);
                        }
                        let duration_width_overflow = next_object_left_edge - default_position;
                        if duration_width_overflow > 0
                        {
                            for k in j..self.contents.len()
                            {
                                self.offset_distance_from_start(system_slices, k,
                                    duration_width_overflow);
                            }
                        }
                        else
                        {
                            for k in first_durationless_object_index..j
                            {
                                self.offset_distance_from_start(system_slices, k,
                                    duration_width_overflow);
                            }
                        }
                        return;
                    } 
                    self.set_distance_from_start(system_slices, j, next_object_left_edge);
                    next_object_left_edge += self.object_width(device_context, music_fonts, j);                             
                }    
                let duration_width_overflow =
                    next_object_left_edge - previous_duration_object_right_edge - duration_width;
                if duration_width_overflow < 0
                {
                    for index in first_durationless_object_index..self.contents.len()
                    {
                        self.offset_distance_from_start(system_slices, index,
                            -duration_width_overflow);
                    }
                }
                return;
            }
        }
        let mut next_object_left_edge = 0;  
        for i in 0..object_index
        {
            self.set_distance_from_start(system_slices, i, next_object_left_edge);
            next_object_left_edge += self.object_width(device_context, music_fonts, i);
        }
        if object_index < self.contents.len()
        {
            let remaining_object_offset =
                next_object_left_edge - self.distance_from_start(system_slices, object_index);
            for i in object_index..self.contents.len()
            {
                self.offset_distance_from_start(system_slices, i, remaining_object_offset);
            }                      
        }
    }
    fn distance_from_start(&self, system_slices: &Vec<SystemSlice>, object_index: usize) -> i32
    {
        match self.contents[object_index].object_type
        {
            StaffObjectType::Duration(duration_index) =>
                system_slices[self.system_slice_indices[duration_index]].distance_from_system_start,
            StaffObjectType::Clef{distance_from_staff_start,..} => distance_from_staff_start
        }
    }
    fn set_distance_from_start(&mut self, system_slices: &mut Vec<SystemSlice>, object_index: usize,
        new_distance_from_staff_start: i32)
    {
        match self.contents[object_index].object_type
        {
            StaffObjectType::Duration(duration_index) =>
                system_slices[self.system_slice_indices[duration_index]].
                distance_from_system_start = new_distance_from_staff_start,
            StaffObjectType::Clef{ref mut distance_from_staff_start,..} =>
                *distance_from_staff_start = new_distance_from_staff_start
        }
    }
    fn offset_distance_from_start(&mut self, system_slices: &mut Vec<SystemSlice>,
        object_index: usize, offset: i32)
    {
        match self.contents[object_index].object_type
        {
            StaffObjectType::Duration(duration_index) => system_slices[
                self.system_slice_indices[duration_index]].distance_from_system_start += offset,
            StaffObjectType::Clef{ref mut distance_from_staff_start,..} =>
                *distance_from_staff_start += offset
        }
    }
    fn object_width(&self, device_context: HDC, music_fonts: &HashMap<u16, SizedMusicFont>,
        object_index: usize) -> i32
    {
        match self.contents[object_index].object_type
        {
            StaffObjectType::Duration(duration_index) =>
            {
                let duration = &self.durations[duration_index];
                let codepoint =
                match duration.steps_above_c4
                {
                    Some(_) => get_notehead_codepoint(duration.log2_duration),
                    None => get_rest_codepoint(duration.log2_duration)
                };
                get_character_width(device_context, music_fonts, self.height, codepoint as u32)
            },
            StaffObjectType::Clef{font_codepoint,..} => 
            {
                let height =
                if object_index > 0
                {
                    (2 * self.height) / 3
                }
                else
                {
                    self.height
                };
                get_character_width(device_context, music_fonts, height, font_codepoint as u32)
            }           
        }
    }
    fn object_right_edge(&self, device_context: HDC, music_fonts: &HashMap<u16, SizedMusicFont>,
        system_slices: &Vec<SystemSlice>, object_index: usize) -> i32
    {
        self.object_width(device_context, music_fonts, object_index) +
        match self.contents[object_index].object_type
        {
            StaffObjectType::Duration(duration_index) =>
                system_slices[self.system_slice_indices[duration_index]].distance_from_system_start,
            StaffObjectType::Clef{distance_from_staff_start,..} => distance_from_staff_start
        }
    }
}

impl Clone for ObjectAddress
{
    fn clone(&self) -> ObjectAddress
    {
        ObjectAddress{staff_index: self.staff_index, object_index: self.object_index}
    }
}

fn increment_system_slice_indices_on_staves(staves: &mut Vec<Staff>,
    smallest_system_slice_index_to_increment: usize, increment_operation: fn(&mut usize))
{
    for staff in staves
    {
        for system_slice_index in staff.system_slice_indices.iter_mut().rev()
        {
            if *system_slice_index < smallest_system_slice_index_to_increment
            {
                break;
            }
            increment_operation(system_slice_index);
        }
    }
}

fn remove_system_slice_if_unused(system_slices: &mut Vec<SystemSlice>, staves: &mut Vec<Staff>,
    index_of_slice_to_remove: usize)
{
    if system_slices[index_of_slice_to_remove].durations_at_position.len() > 0
    {
        return;
    }
    let mut duration_has_been_found_on_staff = vec![false; staves.len()];
    let mut durations_found = 0;
    for system_slice_index in (0..index_of_slice_to_remove).rev()
    {
        for duration_address_index
            in 0..system_slices[system_slice_index].durations_at_position.len()
        {
            let DurationAddress{staff_index, duration_index} = system_slices[system_slice_index].
                durations_at_position[duration_address_index];
            if !duration_has_been_found_on_staff[staff_index]
            {
                if &system_slices[system_slice_index].whole_notes_from_start + get_whole_notes_long(
                    staves[staff_index].durations[duration_index].log2_duration) ==
                    system_slices[index_of_slice_to_remove].whole_notes_from_start
                {
                    return;
                }
                duration_has_been_found_on_staff[staff_index] = true;
                durations_found += 1;
                if durations_found == staves.len()
                {
                    system_slices.remove(index_of_slice_to_remove);
                    increment_system_slice_indices_on_staves(staves, index_of_slice_to_remove,
                        |index|{*index -= 1;});
                    return;
                }
            }
        }
    }
    system_slices.remove(index_of_slice_to_remove);
    increment_system_slice_indices_on_staves(staves, index_of_slice_to_remove,
        |index|{*index -= 1;});
}

fn remove_duration_object(system_slices: &mut Vec<SystemSlice>, staves: &mut Vec<Staff>,
    staff_index: usize, duration_index: usize)
{
    let object_index = staves[staff_index].durations[duration_index].object_index;
    staves[staff_index].contents.remove(object_index);  
    let system_slice_index = staves[staff_index].system_slice_indices[duration_index];
    {
        let system_slice_durations = &mut system_slices[system_slice_index].durations_at_position;
        for index_of_address in 0..system_slice_durations.len()
        {
            if system_slice_durations[index_of_address].staff_index == staff_index
            {
                system_slice_durations.remove(index_of_address);
                break;
            }
        }
    }
    staves[staff_index].system_slice_indices.remove(duration_index);
    staves[staff_index].durations.remove(duration_index);
    for duration_index in system_slice_index..staves[staff_index].durations.len()   
    {
        staves[staff_index].durations[duration_index].object_index -= 1;
        if let StaffObjectType::Duration(ref mut index) =
            staves[staff_index].contents[object_index].object_type
        {
            *index -= 1;
        }            
        for address in &mut system_slices[staves[staff_index].
            system_slice_indices[duration_index]].durations_at_position
        {
            if address.staff_index == staff_index
            {
                address.duration_index -= 1;
                break;
            }
        }
    }        
    remove_system_slice_if_unused(system_slices, staves, system_slice_index);
}

fn offset_slice(system_slices: &mut Vec<SystemSlice>, staves: &mut Vec<Staff>,
    system_slice_index: usize, offset: i32)
{
    system_slices[system_slice_index].distance_from_system_start += offset;
    for duration_address_index in 0..system_slices[system_slice_index].durations_at_position.len()
    {
        let DurationAddress{staff_index, duration_index} =
            system_slices[system_slice_index].durations_at_position[duration_address_index];
        let staff = &mut staves[staff_index];
        let object_index_of_duration =
        if duration_index < staff.durations.len()
        {
            staff.durations[duration_index].object_index
        }
        else
        {
            staff.contents.len()
        };
        for object_index in (0..object_index_of_duration).rev()
        {                    
            if let StaffObjectType::Duration(_) = staff.contents[object_index].object_type
            {
                break;
            }
            staff.offset_distance_from_start(system_slices, object_index, offset);
        }
    }
}

fn respace_slice(device_context: HDC, music_fonts: &HashMap<u16, SizedMusicFont>,
    system_slices: &mut Vec<SystemSlice>, staves: &mut Vec<Staff>, system_slice_index: usize) -> i32
{
    let mut slice_distance_from_start = 0;
    for duration_address_index in 0..system_slices[system_slice_index].durations_at_position.len()
    {
        let DurationAddress{staff_index, duration_index} =
            system_slices[system_slice_index].durations_at_position[duration_address_index];
        let staff = &mut staves[staff_index];
        let mut next_object_right_edge =
            system_slices[system_slice_index].distance_from_system_start;
        let mut slice_minimum_distance_from_start = 0;
        let object_index_of_duration =
        if duration_index < staff.durations.len()
        {
            staff.durations[duration_index].object_index
        }
        else
        {
            staff.contents.len()
        };
        for object_index in (0..object_index_of_duration).rev()
        {
            let character_width = staff.object_width(device_context, music_fonts, object_index);              
            if let StaffObjectType::Duration(duration_index) =
                staff.contents[object_index].object_type
            {
                let duration_width =
                    log2_duration_to_duration_width(staff.durations[duration_index].log2_duration);
                if duration_width > slice_minimum_distance_from_start
                {
                    slice_minimum_distance_from_start = duration_width;
                }
                slice_minimum_distance_from_start +=
                    character_width + staff.distance_from_start(system_slices, object_index);
                break;
            }
            slice_minimum_distance_from_start += character_width;
            next_object_right_edge -= character_width;
            staff.set_distance_from_start(system_slices, object_index, next_object_right_edge);
        }
        if slice_minimum_distance_from_start > slice_distance_from_start
        {
            slice_distance_from_start = slice_minimum_distance_from_start;
        }
    }
    let offset =
        slice_distance_from_start - system_slices[system_slice_index].distance_from_system_start;
    offset_slice(system_slices, staves, system_slice_index, offset);
    offset
}

fn insert_duration_object(device_context: HDC, music_fonts: &HashMap<u16, SizedMusicFont>,
    system_slices: &mut Vec<SystemSlice>, staves: &mut Vec<Staff>, log2_duration: isize,
    steps_above_c4: Option<i8>, object_address: ObjectAddress)
{      
    fn register_staff_slice(system_slices: &mut Vec<SystemSlice>, staves: &mut Vec<Staff>,
        system_slice_index: &mut usize,
        whole_notes_from_start: &num_rational::Ratio<num_bigint::BigUint>)
    {
        while *system_slice_index < system_slices.len()
        {
            if system_slices[*system_slice_index].whole_notes_from_start == *whole_notes_from_start
            {
                return;
            }
            if system_slices[*system_slice_index].whole_notes_from_start > *whole_notes_from_start
            {
                increment_system_slice_indices_on_staves(staves, *system_slice_index,
                    |index|{*index += 1;});
                system_slices.insert(*system_slice_index, SystemSlice{
                    durations_at_position: vec![], whole_notes_from_start:
                    whole_notes_from_start.clone(), distance_from_system_start: 0});
                return;
            }
            *system_slice_index += 1;
        }
        system_slices.push(SystemSlice{durations_at_position: vec![],
            whole_notes_from_start: whole_notes_from_start.clone(), distance_from_system_start: 0});
    }
    let mut whole_notes_from_start = num_rational::Ratio::new(
        num_bigint::BigUint::new(vec![]), num_bigint::BigUint::new(vec![1]));
    for index in (0..object_address.object_index).rev()
    {
        if let StaffObjectType::Duration(duration_index) =
            staves[object_address.staff_index].contents[index].object_type
        {
            whole_notes_from_start = &system_slices[staves[object_address.staff_index].
                system_slice_indices[duration_index]].whole_notes_from_start + get_whole_notes_long(
                staves[object_address.staff_index].durations[duration_index].log2_duration);;
            break;
        }
    }
    let mut object_index = object_address.object_index;
    let mut system_slice_index = 0;
    let mut offset = 0;
    loop
    {
        if object_index < staves[object_address.staff_index].contents.len()
        {
            if let StaffObjectType::Duration(duration_index) =
                staves[object_address.staff_index].contents[object_index].object_type
            {
                staves[object_address.staff_index].durations[duration_index] =
                    Duration{log2_duration: log2_duration,
                    object_index: object_address.object_index, steps_above_c4: steps_above_c4};
                let mut next_whole_notes_from_start =
                    whole_notes_from_start + get_whole_notes_long(log2_duration);
                let mut next_duration_index = duration_index + 1;
                object_index += 1;
                let zero = num_bigint::BigUint::new(vec![]);
                let two = num_bigint::BigUint::new(vec![2]);
                loop
                {
                    if object_index == staves[object_address.staff_index].contents.len()
                    {
                        let mut system_slice_index_of_last_staff_slice =
                            staves[object_address.staff_index].system_slice_indices[
                            staves[object_address.staff_index].system_slice_indices.len() - 1];
                        remove_system_slice_if_unused(system_slices, staves,
                            system_slice_index_of_last_staff_slice);
                        register_staff_slice(system_slices, staves,
                            &mut system_slice_index_of_last_staff_slice,
                            &next_whole_notes_from_start);
                        respace_slice(device_context, music_fonts, system_slices, staves,
                            system_slice_index_of_last_staff_slice);
                        break;
                    }
                    if let StaffObjectType::Duration(duration_index) =
                        staves[object_address.staff_index].contents[object_index].object_type
                    {
                        let next_system_slice_index =
                            staves[object_address.staff_index].system_slice_indices[duration_index];
                        if next_whole_notes_from_start ==
                            system_slices[next_system_slice_index].whole_notes_from_start
                        {
                            let system_slice_index = staves[object_address.staff_index].
                            system_slice_indices[duration_index];
                            offset = respace_slice(device_context, music_fonts, system_slices,
                                staves, system_slice_index);
                            break;
                        }
                        if next_whole_notes_from_start <
                                system_slices[next_system_slice_index].whole_notes_from_start
                        {
                            let rest_duration = &system_slices[next_system_slice_index].
                                whole_notes_from_start - &next_whole_notes_from_start;
                            let mut denominator = rest_duration.denom().clone();
                            let mut numerator = rest_duration.numer().clone();
                            let mut division;
                            let mut rest_log2_duration = 0;                                
                            while denominator != zero
                            {
                                division = numerator.div_rem(&denominator);
                                denominator /= &two;
                                if division.0 != zero
                                {                 
                                    register_staff_slice(system_slices, staves,
                                        &mut system_slice_index, &next_whole_notes_from_start);                                         
                                    for duration_index in next_duration_index..staves[
                                        object_address.staff_index].durations.len()   
                                    {
                                        let duration_object_index =
                                            staves[object_address.staff_index].
                                            durations[duration_index].object_index;
                                        if let StaffObjectType::Duration(ref mut index) =
                                            staves[object_address.staff_index].
                                            contents[duration_object_index].object_type
                                        {
                                            *index += 1;
                                        }
                                        staves[object_address.staff_index].
                                            durations[duration_index].object_index += 1;
                                        for address in &mut system_slices[staves[
                                            object_address.staff_index].system_slice_indices[
                                            duration_index]].durations_at_position
                                        {
                                            if address.staff_index == object_address.staff_index
                                            {
                                                address.duration_index += 1;
                                                break;
                                            }
                                        }
                                    }    
                                    staves[object_address.staff_index].contents.insert(
                                        object_index, StaffObject{object_type:
                                        StaffObjectType::Duration(next_duration_index),
                                        is_selected: false});                              
                                    staves[object_address.staff_index].durations.insert(
                                        next_duration_index, Duration{
                                        log2_duration: rest_log2_duration,
                                        object_index: object_index, steps_above_c4: None});                                        
                                    system_slices[system_slice_index].durations_at_position.push(
                                        DurationAddress{staff_index: object_address.staff_index,
                                        duration_index: next_duration_index});
                                    staves[object_address.staff_index].system_slice_indices.insert(
                                        next_duration_index, system_slice_index);                                               
                                    respace_slice(device_context, music_fonts, system_slices,
                                        staves, system_slice_index);
                                    numerator = division.1;
                                    next_whole_notes_from_start = &next_whole_notes_from_start +
                                        get_whole_notes_long(rest_log2_duration);
                                    system_slice_index += 1;  
                                    next_duration_index += 1;                              
                                }
                                rest_log2_duration -= 1;  
                            }
                            offset = respace_slice(device_context, music_fonts, system_slices,
                                staves, system_slice_index);
                            break;
                        }
                        remove_duration_object(system_slices, staves, object_address.staff_index,
                            duration_index);
                    }
                    else
                    {
                        staves[object_address.staff_index].remove_object(device_context,
                            music_fonts, system_slices, object_index);
                    }
                }
                break;
            }
            object_index += 1;
        }
        else
        {
            let new_object_index = staves[object_address.staff_index].durations.len();
            staves[object_address.staff_index].contents.push(StaffObject{object_type:
                StaffObjectType::Duration(new_object_index), is_selected: false});
            staves[object_address.staff_index].durations.push(Duration{log2_duration: log2_duration,
                object_index: object_address.object_index, steps_above_c4: steps_above_c4});   
            register_staff_slice(system_slices, staves, &mut system_slice_index,
                &whole_notes_from_start);             
            if staves[object_address.staff_index].system_slice_indices.len() == 0
            {                    
                staves[object_address.staff_index].system_slice_indices.push(system_slice_index);
            }
            system_slices[system_slice_index].durations_at_position.push(
                DurationAddress{staff_index: object_address.staff_index,
                duration_index: staves[object_address.staff_index].durations.len() - 1});     
            respace_slice(device_context, music_fonts, system_slices, staves, system_slice_index);
            register_staff_slice(system_slices, staves, &mut system_slice_index,
                &(whole_notes_from_start + get_whole_notes_long(log2_duration)));
            staves[object_address.staff_index].system_slice_indices.push(system_slice_index);
            offset = respace_slice(device_context, music_fonts, system_slices, staves,
                system_slice_index);                   
            break;
        }
    }
    for index in system_slice_index + 1..system_slices.len()
    {
        offset_slice(system_slices, staves, index, offset);
    }
}

fn insert_durationless_object(device_context: HDC, music_fonts: &HashMap<u16, SizedMusicFont>,
    system_slices: &mut Vec<SystemSlice>, staves: &mut Vec<Staff>, object: StaffObject,
    object_address: ObjectAddress)
{
    let staff = &mut staves[object_address.staff_index];
    staff.contents.insert(object_address.object_index, object);
    for i in (0..object_address.object_index).rev()
    {
        if let StaffObjectType::Duration(duration_index) = staff.contents[i].object_type
        {
            let first_durationless_object_index = i + 1;
            let duration_width =
                log2_duration_to_duration_width(staff.durations[duration_index].log2_duration);
            let previous_duration_right_edge = staff.distance_from_start(system_slices, i) +
                get_character_width(device_context, music_fonts, staff.height,
                get_notehead_codepoint(staff.durations[duration_index].log2_duration) as u32);
            let mut next_object_left_edge = previous_duration_right_edge;
            for j in first_durationless_object_index..staff.contents.len()
            {            
                if let StaffObjectType::Duration{..} = staff.contents[j].object_type
                {
                    let duration_width_overflow =
                        next_object_left_edge - previous_duration_right_edge - duration_width;
                    if duration_width_overflow > 0
                    {
                        for k in j..staff.contents.len()
                        {
                            staff.offset_distance_from_start(system_slices, k,
                                duration_width_overflow);
                        }
                    }
                    else
                    {
                        for k in first_durationless_object_index..j
                        {
                            staff.offset_distance_from_start(system_slices, k,
                                -duration_width_overflow);
                        }
                    }
                    return;
                } 
                staff.set_distance_from_start(system_slices, j, next_object_left_edge);
                next_object_left_edge += staff.object_width(device_context, music_fonts, j);                             
            }    
            let duration_width_overflow =
                next_object_left_edge - previous_duration_right_edge - duration_width;
            if duration_width_overflow < 0
            {
                for j in first_durationless_object_index..staff.contents.len()
                {
                    staff.offset_distance_from_start(system_slices, j, -duration_width_overflow);
                }
            }
            return;
        }
    }
    let mut next_object_left_edge = 0;  
    for i in 0..=object_address.object_index
    {
        staff.set_distance_from_start(system_slices, i, next_object_left_edge);
        next_object_left_edge += staff.object_width(device_context, music_fonts, i);
    }
    let next_object_index = object_address.object_index + 1;
    if next_object_index < staff.contents.len()
    {
        let remaining_object_offset =
            next_object_left_edge - staff.distance_from_start(system_slices, next_object_index);
        for index in next_object_index..staff.contents.len()
        {
            staff.offset_distance_from_start(system_slices, index, remaining_object_offset);
        }
    }
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

fn get_whole_notes_long(log2_duration: isize) -> num_rational::Ratio<num_bigint::BigUint>
{
    if log2_duration >= 0
    {
        num_rational::Ratio::new(num_bigint::BigUint::from(2u32.pow(log2_duration as u32)),
            num_bigint::BigUint::new(vec![1]))
    }
    else
    {
        num_rational::Ratio::new(num_bigint::BigUint::new(vec![1]),
            num_bigint::BigUint::from(2u32.pow(-log2_duration as u32)))
    }
}

fn cancel_selection(window_handle: HWND)
{
    unsafe
    {
        let window_memory =
            GetWindowLongPtrW(window_handle, GWLP_USERDATA) as *mut MainWindowMemory;
        match &(*window_memory).selection
        {
            Selection::ActiveCursor(..) =>
            {
                invalidate_client_rect(window_handle);
                EnableWindow((*window_memory).add_clef_button_handle, FALSE);
            }
            Selection::Objects(addresses) =>
            {
                for address in addresses
                {
                    (*window_memory).staves[address.staff_index].contents[address.object_index].
                        is_selected = false;
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

fn get_selected_duration(duration_spin_handle: HWND) -> isize
{
    unsafe
    {
        1 - (SendMessageW(duration_spin_handle, UDM_GETPOS, 0, 0) & 0xff)
    }
}

fn add_sized_music_font(music_fonts: &mut HashMap<u16, SizedMusicFont>, size: u16)
{
    if let Some(sized_font) = music_fonts.get_mut(&size)
    {
        sized_font.number_of_staves_with_size +=1;
        return;
    }
    unsafe
    {
        music_fonts.insert(size,
            SizedMusicFont{font: CreateFontW(-(size as i32), 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            wide_char_string("Bravura").as_ptr()), number_of_staves_with_size: 1});
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
                            let template = ADD_CLEF_DIALOG_TEMPLATE.as_ptr();
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
                            let staff = &mut (*window_memory).staves[address.staff_index];
                            let device_context = GetDC(window_handle);
                            fn remove_old_clef_clef(device_context: HDC,
                                music_fonts: &HashMap<u16, SizedMusicFont>,
                                system_slices: &mut Vec<SystemSlice>, staff: &mut Staff,
                                cursor_index: usize) -> usize
                            {
                                if cursor_index < staff.contents.len()
                                {                                    
                                    if let StaffObjectType::Clef{..} =
                                        staff.contents[cursor_index].object_type
                                    {
                                        staff.remove_object(device_context,
                                            music_fonts,system_slices, cursor_index);                                        
                                        return cursor_index;
                                    }
                                }
                                if cursor_index > 0
                                {
                                    let clef_index = cursor_index - 1;
                                    if let StaffObjectType::Clef{..} =
                                        staff.contents[clef_index].object_type
                                    {
                                        staff.remove_object(device_context, music_fonts,
                                            system_slices, clef_index);
                                        return clef_index;
                                    }
                                }
                                cursor_index
                            };
                            let new_clef_index = remove_old_clef_clef(device_context,
                                &(*window_memory).sized_music_fonts,
                                &mut (*window_memory).system_slices, staff, address.object_index);
                            insert_durationless_object(device_context,
                                &(*window_memory).sized_music_fonts,
                                &mut (*window_memory).system_slices, &mut (*window_memory).staves,
                                StaffObject{object_type: StaffObjectType::Clef{
                                distance_from_staff_start: 0, font_codepoint: codepoint as u16,
                                staff_spaces_of_baseline_above_bottom_line: baseline_offset,
                                steps_of_bottom_staff_line_above_c4: steps_of_bottom_line_above_c4},
                                is_selected: true}, ObjectAddress{object_index: new_clef_index,
                                staff_index: address.staff_index});
                            (*window_memory).selection = Selection::Objects(
                                vec![ObjectAddress{staff_index: address.staff_index,
                                object_index: new_clef_index}]);
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
                    (*window_memory).staves.push(Staff{contents: vec![], durations: vec![],
                        system_slice_indices: vec![], line_thickness_in_staff_spaces:
                        BRAVURA_METADATA.staff_line_thickness, left_edge: 20,
                        bottom_line_vertical_center: bottom_line_y, height: height, line_count: 5});
                    invalidate_client_rect(window_handle);
                    add_sized_music_font(&mut (*window_memory).sized_music_fonts, height);
                    add_sized_music_font(&mut (*window_memory).sized_music_fonts, (2 * height) / 3);
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
                        let log2_duration =
                            get_selected_duration((*window_memory).duration_spin_handle);
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
                        insert_duration_object(device_context, &(*window_memory).sized_music_fonts,
                            &mut (*window_memory).system_slices, &mut (*window_memory).staves,
                            log2_duration, Some(pitch), address.clone());
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
                    match &mut(*window_memory).selection
                    {
                        Selection::Objects(addresses) =>
                        {
                            for address in addresses
                            {
                                let staff = &mut (*window_memory).staves[address.staff_index];
                                match staff.contents[address.object_index].object_type
                                {
                                    StaffObjectType::Clef{
                                        ref mut staff_spaces_of_baseline_above_bottom_line, 
                                        ref mut steps_of_bottom_staff_line_above_c4,..} => 
                                    {
                                        *staff_spaces_of_baseline_above_bottom_line -= 1;
                                        *steps_of_bottom_staff_line_above_c4 += 2;
                                    },
                                    StaffObjectType::Duration(duration_index) =>
                                    {
                                        if let Some(ref mut steps_above_c4) =
                                            staff.durations[duration_index].steps_above_c4
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
                        insert_duration_object(GetDC(window_handle),
                            &(*window_memory).sized_music_fonts,
                            &mut (*window_memory).system_slices, &mut (*window_memory).staves,
                            get_selected_duration((*window_memory).duration_spin_handle), None,
                            address.clone());
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
                    match &mut(*window_memory).selection
                    {
                        Selection::Objects(addresses) =>
                        {
                            for address in addresses
                            {
                                let staff = &mut(*window_memory).staves[address.staff_index];
                                match staff.contents[address.object_index].object_type
                                {
                                    StaffObjectType::Clef{
                                        ref mut staff_spaces_of_baseline_above_bottom_line, 
                                        ref mut steps_of_bottom_staff_line_above_c4,..} =>
                                    {
                                        *staff_spaces_of_baseline_above_bottom_line += 1;
                                        *steps_of_bottom_staff_line_above_c4 -= 2;
                                    },
                                    StaffObjectType::Duration(duration_index) =>
                                    {
                                        if let Some(ref mut steps_above_c4) =
                                            staff.durations[duration_index].steps_above_c4
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
                        top: staff.bottom_line_vertical_center - staff.height as i32,
                        right: WHOLE_NOTE_WIDTH as i32, bottom: staff.bottom_line_vertical_center},
                        TRUE);
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
                if staff.left_edge <= cursor_x && cursor_x <=
                    staff.left_edge + WHOLE_NOTE_WIDTH as i32 &&
                    staff.bottom_line_vertical_center - staff.height as i32 <= cursor_y &&
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
                                top: old_staff.bottom_line_vertical_center -
                                old_staff.height as i32, right: WHOLE_NOTE_WIDTH as i32,
                                bottom: old_staff.bottom_line_vertical_center}, TRUE);
                        }
                        None => ()
                    }
                    (*window_memory).ghost_cursor =
                        Some(ObjectAddress{staff_index: staff_index, object_index: 0});
                    InvalidateRect(window_handle, &RECT{left: staff.left_edge,
                        top: staff.bottom_line_vertical_center - staff.height as i32,
                        right: WHOLE_NOTE_WIDTH as i32, bottom: staff.bottom_line_vertical_center},
                        TRUE);
                    return 0;
                }
            }
            match (*window_memory).ghost_cursor
            {
                Some(ref address) =>
                {                     
                    let staff = &(*window_memory).staves[address.staff_index];
                    InvalidateRect(window_handle, &RECT{left: staff.left_edge,
                        top: staff.bottom_line_vertical_center - staff.height as i32,
                        right: WHOLE_NOTE_WIDTH as i32, bottom: staff.bottom_line_vertical_center},
                        TRUE);
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
                let mut right_edge = staff.left_edge + WHOLE_NOTE_WIDTH as i32;
                if staff.contents.len() > 0
                {
                    right_edge += staff.object_right_edge(device_context,
                        &(*window_memory).sized_music_fonts, &(*window_memory).system_slices,
                        staff.contents.len() - 1);
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
                        SetTextColor(device_context, RED);
                        steps_of_bottom_line_above_c4 = staff.draw_object(device_context,
                            &(*window_memory).sized_music_fonts, &(*window_memory).system_slices,
                            steps_of_bottom_line_above_c4, index);
                        SetTextColor(device_context, BLACK);
                    }
                    else
                    {
                        steps_of_bottom_line_above_c4 = staff.draw_object(device_context,
                            &(*window_memory).sized_music_fonts, &(*window_memory).system_slices,
                            steps_of_bottom_line_above_c4, index);
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
                        staff.height as i32, staff.left_edge + 1,
                        staff.bottom_line_vertical_center);
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
                        cursor_left_edge += staff.object_right_edge(device_context,
                            &(*window_memory).sized_music_fonts, &(*window_memory).system_slices,
                            address.object_index - 1);
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
                        let half_space = remainder * staff.height as i32 /
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
                        staff.bottom_line_vertical_center - staff.height as i32
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

unsafe fn init() -> (HWND, MainWindowMemory)
{
    let gray = RGB(127, 127, 127);
    GRAY_PEN = Some(CreatePen(PS_SOLID as i32, 1, gray));
    GRAY_BRUSH = Some(CreateSolidBrush(gray));
    let red = RGB(255, 0, 0);
    RED_PEN = Some(CreatePen(PS_SOLID as i32, 1, red));
    RED_BRUSH = Some(CreateSolidBrush(red));
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
    let main_window_memory = MainWindowMemory{sized_music_fonts: HashMap::new(),
        staves: Vec::new(), system_slices: vec![], ghost_cursor: None, selection: Selection::None,
        add_staff_button_handle: add_staff_button_handle,
        add_clef_button_handle: add_clef_button_handle,
        duration_display_handle: duration_display_handle,
        duration_spin_handle: duration_spin_handle};
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