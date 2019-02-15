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

pub const IDC_ADD_STAFF_LINE_COUNT: i32 = 8;
pub const IDC_ADD_STAFF_SCALE_LIST: i32 = 9;
pub const IDC_ADD_STAFF_ADD_SCALE: i32 = 10;
pub const IDC_ADD_STAFF_EDIT_SCALE: i32 = 11;
pub const IDC_ADD_STAFF_REMOVE_SCALE: i32 = 12;

pub const IDC_EDIT_STAFF_SCALE_NAME: i32 = 8;
pub const IDC_EDIT_STAFF_SCALE_VALUE: i32 = 9;

pub const IDC_REMAP_STAFF_SCALE_LIST: i32 = 8;

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

pub struct Point<T>
{
    pub x: T,
    pub y: T
}

pub fn unterminated_wide_char_string(value: &str) -> Vec<u16>
{
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(value).encode_wide().collect()
}

pub fn wide_char_string(value: &str) -> Vec<u16>
{
    let mut string = unterminated_wide_char_string(value);
    string.push(0);
    string
}