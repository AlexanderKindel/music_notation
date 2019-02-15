extern crate serde_json;
extern crate winapi;

#[path="src/shared.rs"] mod shared;

use shared::*;
use std::fs::File;
use std::io::Write;
use winapi::shared::minwindef::*;
use winapi::um::commctrl::*;
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
    let mut template: Vec<u8> = vec![];
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
    template_string.append(&mut b": Template<[u8; ".to_vec());
    template_string.append(&mut template.len().to_string().as_bytes().to_vec());
    template_string.append(&mut b"]> = Template{data: [".to_vec());
    for entry in template
    {
        template_string.append(&mut entry.to_string().as_bytes().to_vec());
        template_string.append(&mut b", ".to_vec());
    }
    template_string.pop();
    template_string.append(&mut b"]};\n".to_vec());
    template_string
}

fn main()
{
    let mut constants_file = File::create("src/constants.rs").unwrap();
    constants_file.write(b"#[repr(align(32))]\nstruct Template<A>\n{\n    data: A\n}\n\n");
    constants_file.write(b"static BLACK: COLORREF = ");
    constants_file.write(RGB(0, 0, 0).to_string().as_bytes());
    constants_file.write(b";\n");
    constants_file.write(b"static RED: COLORREF = ");
    constants_file.write(RGB(255, 0, 0).to_string().as_bytes());
    constants_file.write(b";\n");
    constants_file.write(b"static WHITE: COLORREF = ");
    constants_file.write(RGB(255, 255, 255).to_string().as_bytes());
    constants_file.write(b";\n");
    let button_string = wide_char_string("button");
    let cancel_string = wide_char_string("Cancel");
    let edit_string = wide_char_string("edit");
    let empty_string = wide_char_string("");
    let ok_string = wide_char_string("OK");
    let static_string = wide_char_string("static");
    let mut add_clef_dialog_ok = create_dialog_control_template(BS_PUSHBUTTON | WS_VISIBLE, 45, 65,
        30, 10, IDOK as u32, &button_string, &ok_string);
    let mut add_clef_dialog_cancel = create_dialog_control_template(BS_PUSHBUTTON | WS_VISIBLE, 75,
        65, 30, 10, IDCANCEL as u32, &button_string, &cancel_string);
    let mut add_clef_dialog_shape = create_dialog_control_template(SS_LEFT | WS_CHILD | WS_VISIBLE,
        5, 5, 40, 10, 0, &static_string, &wide_char_string("Clef shape:"));
    let mut add_clef_dialog_octave = create_dialog_control_template(SS_LEFT | WS_CHILD | WS_VISIBLE,
        75, 5, 70, 10, 0, &static_string, &wide_char_string("Octave transposition:"));
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
    let mut add_staff_dialog_cancel = create_dialog_control_template(BS_PUSHBUTTON | WS_VISIBLE, 87,
        65, 30, 10, IDCANCEL as u32, &button_string, &cancel_string);
    let mut add_staff_dialog_ok = create_dialog_control_template(BS_PUSHBUTTON | WS_VISIBLE, 57, 65,
        30, 10, IDOK as u32, &button_string, &ok_string);
    let mut add_staff_dialog_line_count_label = create_dialog_control_template(SS_LEFT | WS_CHILD |
        WS_VISIBLE, 5, 5, 40, 10, 0, &static_string, &wide_char_string("Line count:"));
    let mut add_staff_dialog_line_count_display = create_dialog_control_template(WS_BORDER |
        WS_CHILD | WS_VISIBLE, 45, 5, 20, 10, 0, &static_string, &wide_char_string("5"));
    let mut add_staff_dialog_line_count_spin = create_dialog_control_template(UDS_ALIGNRIGHT |
        UDS_AUTOBUDDY | UDS_SETBUDDYINT | WS_CHILD | WS_VISIBLE, 0, 0, 0, 0,
        IDC_ADD_STAFF_LINE_COUNT as u32, &wide_char_string(UPDOWN_CLASS), &vec![]);
    let mut add_staff_dialog_scale_label = create_dialog_control_template(SS_LEFT | WS_CHILD |
        WS_VISIBLE, 5, 25, 60, 10, 0, &static_string, &wide_char_string("Scale:"));
    let mut add_staff_dialog_scale_list =
        create_dialog_control_template(CBS_DROPDOWNLIST | CBS_HASSTRINGS | WS_CHILD | WS_VISIBLE, 5,
        35, 80, 100, IDC_ADD_STAFF_SCALE_LIST as u32, &wide_char_string("COMBOBOX"), &empty_string);
    let mut add_staff_dialog_add_scale = create_dialog_control_template(BS_PUSHBUTTON | WS_VISIBLE,
        90, 25, 80, 10, IDC_ADD_STAFF_ADD_SCALE as u32, &button_string,
        &wide_char_string("Add new scale"));
    let mut add_staff_dialog_edit_scale = create_dialog_control_template(BS_PUSHBUTTON |
        WS_VISIBLE, 90, 35, 80, 10, IDC_ADD_STAFF_EDIT_SCALE as u32, &button_string,
        &wide_char_string("Edit selected scale"));
    let mut add_staff_dialog_remove_scale = create_dialog_control_template(BS_PUSHBUTTON |
        WS_VISIBLE, 90, 45, 80, 10, IDC_ADD_STAFF_REMOVE_SCALE as u32, &button_string,
        &wide_char_string("Remove selected scale"));
    constants_file.write(create_dialog_template_constant(b"ADD_STAFF_DIALOG_TEMPLATE".to_vec(),
        DS_CENTER, 0, 0, 185, 100, wide_char_string("Add Staff"),
        vec![&mut add_staff_dialog_cancel, &mut add_staff_dialog_ok,
        &mut add_staff_dialog_line_count_label, &mut add_staff_dialog_line_count_display,
        &mut add_staff_dialog_line_count_spin, &mut add_staff_dialog_scale_label,
        &mut add_staff_dialog_add_scale, &mut add_staff_dialog_edit_scale,
        &mut add_staff_dialog_remove_scale, &mut add_staff_dialog_scale_list]).as_slice()).unwrap();
    let mut edit_staff_scale_dialog_cancel = create_dialog_control_template(BS_PUSHBUTTON |
        WS_VISIBLE, 35, 55, 30, 10, IDCANCEL as u32, &button_string, &cancel_string);
    let mut edit_staff_scale_dialog_ok = create_dialog_control_template(BS_PUSHBUTTON | WS_VISIBLE,
        5, 55, 30, 10, IDOK as u32, &button_string, &ok_string);
    let mut edit_staff_scale_dialog_name_label = create_dialog_control_template(SS_LEFT | WS_CHILD |
        WS_VISIBLE, 5, 5, 60, 10, 0, &static_string, &wide_char_string("Name:"));
    let mut edit_staff_scale_dialog_name_edit = create_dialog_control_template(WS_BORDER |
        WS_CHILD | WS_VISIBLE, 5, 15, 60, 10, IDC_EDIT_STAFF_SCALE_NAME as u32, &edit_string,
        &empty_string);
    let mut edit_staff_scale_dialog_value_label = create_dialog_control_template(SS_LEFT |
        WS_CHILD | WS_VISIBLE, 5, 25, 60, 10, 0, &static_string, &wide_char_string("Value:"));
    let mut edit_staff_scale_dialog_value_edit = create_dialog_control_template(WS_BORDER |
        WS_CHILD | WS_VISIBLE, 5, 35, 60, 10, IDC_EDIT_STAFF_SCALE_VALUE as u32, &edit_string,
        &empty_string);
    constants_file.write(create_dialog_template_constant(
        b"EDIT_STAFF_SCALE_DIALOG_TEMPLATE".to_vec(), DS_CENTER, 0, 0, 80, 90,
        wide_char_string("Edit Staff Scale"), vec![&mut edit_staff_scale_dialog_cancel,
        &mut edit_staff_scale_dialog_ok, &mut edit_staff_scale_dialog_name_label,
        &mut edit_staff_scale_dialog_value_label, &mut edit_staff_scale_dialog_name_edit,
        &mut edit_staff_scale_dialog_value_edit]).as_slice()).unwrap();
    let mut remap_staff_scale_dialog_cancel = create_dialog_control_template(BS_PUSHBUTTON |
        WS_VISIBLE, 60, 70, 30, 10, IDCANCEL as u32, &button_string, &cancel_string);
    let mut remap_staff_scale_dialog_ok = create_dialog_control_template(BS_PUSHBUTTON | WS_VISIBLE,
        30, 70, 30, 10, IDOK as u32, &button_string, &ok_string);
    let mut remap_staff_scale_dialog_text = create_dialog_control_template(SS_LEFT | WS_CHILD |
        WS_VISIBLE, 5, 5, 115, 35, 0, &static_string, &wide_char_string("One or more existing \
        staves use the scale marked for deletion. Choose a new scale for these staves."));
    let mut remap_staff_scale_dialog_scale_list =
        create_dialog_control_template(CBS_DROPDOWNLIST | CBS_HASSTRINGS | WS_CHILD | WS_VISIBLE, 5,
        40, 110, 100, IDC_REMAP_STAFF_SCALE_LIST as u32, &wide_char_string("COMBOBOX"),
        &empty_string);
    constants_file.write(create_dialog_template_constant(
        b"REMAP_STAFF_SCALE_DIALOG_TEMPLATE".to_vec(), DS_CENTER, 0, 0, 130, 105,
        wide_char_string("Remap Staff Scale"), vec![&mut remap_staff_scale_dialog_cancel,
        &mut remap_staff_scale_dialog_ok, &mut remap_staff_scale_dialog_text,
        &mut remap_staff_scale_dialog_scale_list]).as_slice()).unwrap();
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
    write_serde_point_to_struct(&mut constants_file, "black_notehead_stem_up_se",
        &black_notehead_anchors["stemUpSE"]);
    write_serde_point_to_struct(&mut constants_file, "black_notehead_stem_down_nw",
        &black_notehead_anchors["stemDownNW"]);
    write_serde_point_to_struct(&mut constants_file, "half_notehead_stem_up_se",
        &half_notehead_anchors["stemUpSE"]);
    write_serde_point_to_struct(&mut constants_file, "half_notehead_stem_down_nw",
        &half_notehead_anchors["stemDownNW"]);
    write_serde_float_to_struct(&mut constants_file, "beam_spacing",
        &engraving_defaults["beamSpacing"]);
    write_serde_float_to_struct(&mut constants_file, "beam_thickness",
        &engraving_defaults["beamThickness"]);
    write_field_name_to_struct(&mut constants_file, "double_whole_notehead_x_offset");
    write_serde_float_as_char_bytes(&mut constants_file,
        &glyphs_with_anchors["noteheadDoubleWhole"]["noteheadOrigin"].as_array().unwrap()[0]);
    constants_file.write(b", ");
    write_serde_float_to_struct(&mut constants_file, "leger_line_thickness",
        &engraving_defaults["legerLineThickness"]);
    write_serde_float_to_struct(&mut constants_file, "leger_line_extension",
        &engraving_defaults["legerLineExtension"]);
    write_serde_float_to_struct(&mut constants_file, "staff_line_thickness",
        &engraving_defaults["staffLineThickness"]);
    write_serde_float_to_struct(&mut constants_file, "stem_thickness",
        &engraving_defaults["stemThickness"]);    
    constants_file.write(b"};\n");
}

fn write_field_name_to_struct(struct_file: &mut File, field_name: &str)
{
    struct_file.write(field_name.as_bytes());
    struct_file.write(b": ");
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