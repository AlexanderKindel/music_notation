extern crate serde_json;
extern crate winapi;

#[path="src/shared.rs"] mod shared;

use shared::*;
use std::fs::File;
use std::io::Write;
use winapi::shared::minwindef::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

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

fn create_dialog_template_constant(constant_name: Vec<u8>, style: DWORD, left_edge: u16,
    top_edge: u16, width: u16, height: u16, title: Vec<u16>, controls: Vec<&mut Vec<u8>>) -> Vec<u8>
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
    let mut template_string = b"static ".to_vec();
    template_string.append(&mut constant_name.to_vec());
    template_string.append(&mut b": [u8; ".to_vec());
    template_string.append(&mut template.len().to_string().as_bytes().to_vec());
    template_string.append(&mut b"] = [".to_vec());
    for entry in template
    {
        template_string.append(&mut entry.to_string().as_bytes().to_vec());
        template_string.append(&mut b", ".to_vec());
    }
    template_string.pop();
    template_string.append(&mut b"];\n".to_vec());
    template_string
}

fn write_serde_float_as_char_bytes(file: &mut File, value: &serde_json::value::Value)
{
    let float_string = value.as_f64().unwrap().to_string();
    let float_string_bytes = float_string.as_bytes();
    file.write(float_string_bytes);
    if !float_string_bytes.contains(&('.' as u8))
    {
        file.write(b".0");
    }
}

fn write_field_name_to_struct(struct_file: &mut File, field_name: &str)
{
    struct_file.write(field_name.as_bytes());
    struct_file.write(b": ");
}

fn write_serde_float_to_struct(struct_file: &mut File, field_name: &str,
    value: &serde_json::value::Value)
{
    write_field_name_to_struct(struct_file, field_name);
    write_serde_float_as_char_bytes(struct_file, value);
    struct_file.write(b", ");
}

fn write_serde_point_to_struct(struct_file: &mut File, field_name: &str,
    value: &serde_json::value::Value)
{
    write_field_name_to_struct(struct_file, field_name);
    let array = value.as_array().unwrap();
    struct_file.write(b"Point{x: ");
    write_serde_float_as_char_bytes(struct_file, &array[0]);
    struct_file.write(b", y: ");
    write_serde_float_as_char_bytes(struct_file, &array[1]);
    struct_file.write(b"}, ");
}

fn main()
{
    let mut constants_file = File::create("src/constants.rs").unwrap();
    constants_file.write(b"static BLACK: COLORREF = ");
    constants_file.write(RGB(0, 0, 0).to_string().as_bytes());
    constants_file.write(b";\n");
    constants_file.write(b"static RED: COLORREF = ");
    constants_file.write(RGB(255, 0, 0).to_string().as_bytes());
    constants_file.write(b";\n");
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
    constants_file.write(create_dialog_template_constant(b"ADD_CLEF_DIALOG_TEMPLATE".to_vec(),
        DS_CENTER, 0, 0, 160, 100, wide_char_string("Add Clef"), vec![&mut add_clef_dialog_ok,
        &mut add_clef_dialog_cancel, &mut add_clef_dialog_shape, &mut add_clef_dialog_octave,
        &mut add_clef_dialog_g_clef, &mut add_clef_dialog_c_clef, &mut add_clef_dialog_f_clef,
        &mut add_clef_dialog_unpitched_clef, &mut add_clef_dialog_15ma, &mut add_clef_dialog_8va,
        &mut add_clef_dialog_none, &mut add_clef_dialog_8vb,
        &mut add_clef_dialog_15mb]).as_slice()).unwrap();
    let bravura_metadata_file =
        File::open("bravura_metadata.json").expect("Failed to open bravura_metadata.json");    
    let bravura_metadata: serde_json::Value = 
        serde_json::from_reader(bravura_metadata_file).unwrap();
    let engraving_defaults = &bravura_metadata["engravingDefaults"];
    let glyphs_with_anchors = &bravura_metadata["glyphsWithAnchors"];
    let black_notehead_anchors = &glyphs_with_anchors["noteheadBlack"];
    let half_notehead_anchors = &glyphs_with_anchors["noteheadHalf"]; 
    constants_file.write(
        b"static BRAVURA_METADATA: FontMetadata = FontMetadata{").unwrap();
    write_serde_float_to_struct(&mut constants_file, "beam_spacing",
        &engraving_defaults["beamSpacing"]);
    write_serde_float_to_struct(&mut constants_file, "beam_thickness",
        &engraving_defaults["beamThickness"]);
    write_serde_float_to_struct(&mut constants_file, "leger_line_thickness",
        &engraving_defaults["legerLineThickness"]);
    write_serde_float_to_struct(&mut constants_file, "leger_line_extension",
        &engraving_defaults["legerLineExtension"]);
    write_serde_float_to_struct(&mut constants_file, "staff_line_thickness",
        &engraving_defaults["staffLineThickness"]);
    write_serde_float_to_struct(&mut constants_file, "stem_thickness",
        &engraving_defaults["stemThickness"]);
    write_serde_point_to_struct(&mut constants_file, "black_notehead_stem_up_se",
        &black_notehead_anchors["stemUpSE"]);
    write_serde_point_to_struct(&mut constants_file, "black_notehead_stem_down_nw",
        &black_notehead_anchors["stemDownNW"]);
    write_serde_point_to_struct(&mut constants_file, "half_notehead_stem_up_se",
        &half_notehead_anchors["stemUpSE"]);
    write_serde_point_to_struct(&mut constants_file, "half_notehead_stem_down_nw",
        &half_notehead_anchors["stemDownNW"]);
    constants_file.write(b"};\n");
}