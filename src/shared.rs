//The add clef dialog returns the button identifiers of the selected clef shape and octave
//transposition ored together, so the nonzero bits of the shape identifiers must not overlap with
//those of the transposition identifiers.
pub const IDC_ADD_CLEF_G: i32 = 0b1000;
pub const IDC_ADD_CLEF_C: i32 = 0b1001;
pub const IDC_ADD_CLEF_F: i32 = 0b1010;
pub const IDC_ADD_CLEF_UNPITCHED: i32 = 0b1011;
pub const IDC_ADD_CLEF_15MA: i32 = 0b10000;
pub const IDC_ADD_CLEF_8VA: i32 = 0b100000;
pub const IDC_ADD_CLEF_NONE: i32 = 0b110000;
pub const IDC_ADD_CLEF_8VB: i32 = 0b1000000;
pub const IDC_ADD_CLEF_15MB: i32 = 0b1010000;
pub const ADD_CLEF_SHAPE_BITS: isize = 0b1111;
pub const ADD_CLEF_TRANSPOSITION_BITS: isize = 0b1110000;

pub struct Point<T>
{
    pub x: T,
    pub y: T
}

pub struct FontMetadata
{
    pub black_notehead_stem_up_se: Point<f32>,
    pub black_notehead_stem_down_nw: Point<f32>,
    pub half_notehead_stem_up_se: Point<f32>,
    pub half_notehead_stem_down_nw: Point<f32>,
    pub beam_spacing: f32,
    pub beam_thickness: f32,
    pub double_whole_notehead_x_offset: f32,
    pub leger_line_thickness: f32,
    pub leger_line_extension: f32,
    pub staff_line_thickness: f32,
    pub stem_thickness: f32    
}

pub fn wide_char_string(value: &str) -> Vec<u16>
{    
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::iter;
    OsStr::new(value).encode_wide().chain(iter::once(0)).collect()
}