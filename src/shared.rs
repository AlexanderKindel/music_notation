pub const IDC_ADD_CLEF_G: i32 = 8;
pub const IDC_ADD_CLEF_C: i32 = 9;
pub const IDC_ADD_CLEF_F: i32 = 10;
pub const IDC_ADD_CLEF_UNPITCHED: i32 = 11;
pub const IDC_ADD_CLEF_15MA: i32 = 12;
pub const IDC_ADD_CLEF_8VA: i32 = 13;
pub const IDC_ADD_CLEF_NONE: i32 = 14;
pub const IDC_ADD_CLEF_8VB: i32 = 15;
pub const IDC_ADD_CLEF_15MB: i32 = 16;

pub const IDC_ADD_STAFF_LINE_COUNT_SPIN: i32 = 8;
pub const IDC_ADD_STAFF_LINE_COUNT_DISPLAY: i32 = 9;
pub const IDC_ADD_STAFF_SCALE_LIST: i32 = 10;
pub const IDC_ADD_STAFF_ADD_SCALE: i32 = 11;
pub const IDC_ADD_STAFF_EDIT_SCALE: i32 = 12;
pub const IDC_ADD_STAFF_REMOVE_SCALE: i32 = 13;

pub const IDC_EDIT_STAFF_SCALE_NAME: i32 = 8;
pub const IDC_EDIT_STAFF_SCALE_VALUE: i32 = 9;

pub const IDC_REMAP_STAFF_SCALE_LIST: i32 = 8;

pub const IDC_ADD_KEY_SIG_ACCIDENTAL_COUNT: i32 = 8;
pub const IDC_ADD_KEY_SIG_SHARPS: i32 = 9;
pub const IDC_ADD_KEY_SIG_FLATS: i32 = 10;

pub const WHOLE_NOTE_WIDTH: f32 = 7.0;

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