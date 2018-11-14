extern crate winapi;

use winapi::shared::minwindef::*;
use winapi::um::winuser::*;

mod init;

fn wide_char_string(value: &str) -> Vec<u16>
{    
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(value).encode_wide().chain(std::iter::once(0)).collect()
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
    let mut template: Vec<u8> = Vec::with_capacity(26 + 2 * (window_class.len() + text.len()));
    template.append(&mut vec![0; 8]);
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
    template.append(&mut vec![0; 2]);
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
    template.append(&mut vec![0; 4]);
    for character in title
    {
        add_u16(&mut template, character);
    }    
    for control in controls
    {
        if template.len() % 4 != 0
        {
            template.append(&mut vec![0; 2]);
        }
        template.append(control);
    }
    template.shrink_to_fit();
    template    
}

fn main()
{
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
    let ADD_CLEF_DIALOG_TEMPLATE = create_dialog_template(DS_CENTER, 0, 0, 160, 100,
        wide_char_string("Add Clef"), vec![&mut add_clef_dialog_ok, &mut add_clef_dialog_cancel,
        &mut add_clef_dialog_shape, &mut add_clef_dialog_octave, &mut add_clef_dialog_g_clef,
        &mut add_clef_dialog_c_clef, &mut add_clef_dialog_f_clef,
        &mut add_clef_dialog_unpitched_clef, &mut add_clef_dialog_15ma, &mut add_clef_dialog_8va,
        &mut add_clef_dialog_none, &mut add_clef_dialog_8vb, &mut add_clef_dialog_15mb]);
}