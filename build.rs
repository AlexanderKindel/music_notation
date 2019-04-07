extern crate serde_json;
extern crate winapi;

#[path="src/shared.rs"] mod shared;

use shared::*;
use std::fs::File;
use std::io::Write;
use std::ptr::null_mut;
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
    top_edge: u16, width: u16, height: u16, title: Vec<u16>, mut font_info: Vec<u8>,
    controls: Vec<&mut Vec<u8>>) -> Vec<u8>
{
    let mut template: Vec<u8> = vec![1, 0, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0];
    add_u32(&mut template, style | DS_SETFONT);
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
    template.append(&mut font_info);
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
    let font_info =
    unsafe
    {
        let mut metrics: NONCLIENTMETRICSW = std::mem::uninitialized();
        metrics.cbSize = std::mem::size_of::<NONCLIENTMETRICSW>() as u32;
        SystemParametersInfoW(SPI_GETNONCLIENTMETRICS, metrics.cbSize,
            &mut metrics as *mut _ as *mut winapi::ctypes::c_void, 0);
        let mut font_info: Vec<u8> = vec![];
        if metrics.lfMessageFont.lfHeight < 0
        {
            metrics.lfMessageFont.lfHeight = ((-metrics.lfMessageFont.lfHeight as i64 * 72) /
                GetDeviceCaps(GetDC(null_mut()), LOGPIXELSY) as i64) as i32;
        }
        add_u16(&mut font_info, metrics.lfMessageFont.lfHeight as u16);
        add_u16(&mut font_info, metrics.lfMessageFont.lfWeight as u16);
        font_info.push(metrics.lfMessageFont.lfItalic);
        font_info.push(metrics.lfMessageFont.lfCharSet);
        let mut char_index = 0;
        loop
        {
            let character = metrics.lfMessageFont.lfFaceName[char_index];
            add_u16(&mut font_info, character);
            if character == 0
            {
                break;
            }
            char_index += 1;
        }
        font_info
    };
    let mut constants_file = File::create("src/constants.rs").unwrap();
    constants_file.write(b"#[repr(align(32))]\nstruct Template<A>\n{\n    data: A\n}\n\n");
    write_constant(&mut constants_file, "DURATION_RATIO: f32",
        (WHOLE_NOTE_WIDTH / 2.0).sqrt().sqrt());
    write_constant(&mut constants_file, "BLACK: COLORREF", RGB(0, 0, 0));
    write_constant(&mut constants_file, "RED: COLORREF", RGB(255, 0, 0));
    write_constant(&mut constants_file, "WHITE: COLORREF", RGB(255, 255, 255));
    let button_string = wide_char_string("button");
    let cancel_string = wide_char_string("Cancel");
    let edit_string = wide_char_string("edit");
    let empty_string = wide_char_string("");
    let ok_string = wide_char_string("OK");
    let static_string = wide_char_string("static");
    let mut add_staff_cancel = create_dialog_control_template(BS_PUSHBUTTON | WS_VISIBLE, 80, 65,
        30, 10, IDCANCEL as u32, &button_string, &cancel_string);
    let mut add_staff_ok = create_dialog_control_template(BS_PUSHBUTTON | WS_VISIBLE, 50, 65, 30,
        10, IDOK as u32, &button_string, &ok_string);
    let mut add_staff_line_count_label = create_dialog_control_template(SS_LEFT | WS_CHILD |
        WS_VISIBLE, 5, 5, 40, 10, 0, &static_string, &wide_char_string("Line count:"));
    let mut add_staff_line_count_display = create_dialog_control_template(WS_BORDER | WS_CHILD |
        WS_VISIBLE, 45, 5, 20, 10, IDC_ADD_STAFF_LINE_COUNT_DISPLAY as u32, &static_string,
        &wide_char_string("5"));
    let mut add_staff_line_count_spin = create_dialog_control_template(UDS_ALIGNRIGHT |
        UDS_AUTOBUDDY | UDS_SETBUDDYINT | WS_CHILD | WS_VISIBLE, 0, 0, 0, 0,
        IDC_ADD_STAFF_LINE_COUNT_SPIN as u32, &wide_char_string(UPDOWN_CLASS), &vec![]);
    let mut add_staff_scale_label = create_dialog_control_template(SS_LEFT | WS_CHILD | WS_VISIBLE,
        5, 25, 60, 10, 0, &static_string, &wide_char_string("Scale:"));
    let mut add_staff_scale_list = create_dialog_control_template(CBS_DROPDOWNLIST |
        CBS_HASSTRINGS | WS_CHILD | WS_VISIBLE, 5, 35, 70, 100, IDC_ADD_STAFF_SCALE_LIST as u32,
        &wide_char_string("COMBOBOX"), &empty_string);
    let mut add_staff_add_scale = create_dialog_control_template(BS_PUSHBUTTON | WS_VISIBLE, 85, 25,
        75, 10, IDC_ADD_STAFF_ADD_SCALE as u32, &button_string, &wide_char_string("Add new scale"));
    let mut add_staff_edit_scale = create_dialog_control_template(BS_PUSHBUTTON | WS_VISIBLE, 85,
        35, 75, 10, IDC_ADD_STAFF_EDIT_SCALE as u32, &button_string,
        &wide_char_string("Edit selected scale"));
    let mut add_staff_remove_scale = create_dialog_control_template(BS_PUSHBUTTON | WS_VISIBLE, 85,
        45, 75, 10, IDC_ADD_STAFF_REMOVE_SCALE as u32, &button_string,
        &wide_char_string("Remove selected scale"));
    constants_file.write(create_dialog_template_constant(b"ADD_STAFF_DIALOG_TEMPLATE".to_vec(),
        DS_CENTER, 0, 0, 165, 80, wide_char_string("Add Staff"), font_info.clone(),
        vec![&mut add_staff_cancel, &mut add_staff_ok, &mut add_staff_line_count_label,
        &mut add_staff_line_count_display, &mut add_staff_line_count_spin,
        &mut add_staff_scale_label, &mut add_staff_add_scale, &mut add_staff_edit_scale,
        &mut add_staff_remove_scale, &mut add_staff_scale_list]).as_slice()).unwrap();
    let mut edit_staff_scale_cancel = create_dialog_control_template(BS_PUSHBUTTON | WS_VISIBLE, 35,
        55, 30, 10, IDCANCEL as u32, &button_string, &cancel_string);
    let mut edit_staff_scale_ok = create_dialog_control_template(BS_PUSHBUTTON | WS_VISIBLE, 5, 55,
        30, 10, IDOK as u32, &button_string, &ok_string);
    let mut edit_staff_scale_name_label = create_dialog_control_template(SS_LEFT | WS_CHILD |
        WS_VISIBLE, 5, 5, 60, 10, 0, &static_string, &wide_char_string("Name:"));
    let mut edit_staff_scale_name_edit = create_dialog_control_template(WS_BORDER | WS_CHILD |
        WS_VISIBLE, 5, 15, 60, 10, IDC_EDIT_STAFF_SCALE_NAME as u32, &edit_string, &empty_string);
    let mut edit_staff_scale_value_label = create_dialog_control_template(SS_LEFT | WS_CHILD |
        WS_VISIBLE, 5, 25, 60, 10, 0, &static_string, &wide_char_string("Value:"));
    let mut edit_staff_scale_value_edit = create_dialog_control_template(WS_BORDER | WS_CHILD |
        WS_VISIBLE, 5, 35, 60, 10, IDC_EDIT_STAFF_SCALE_VALUE as u32, &edit_string, &empty_string);
    constants_file.write(create_dialog_template_constant(
        b"EDIT_STAFF_SCALE_DIALOG_TEMPLATE".to_vec(), DS_CENTER, 0, 0, 70, 70,
        wide_char_string("Edit Staff Scale"), font_info.clone(), vec![&mut edit_staff_scale_cancel,
        &mut edit_staff_scale_ok, &mut edit_staff_scale_name_label,
        &mut edit_staff_scale_value_label, &mut edit_staff_scale_name_edit,
        &mut edit_staff_scale_value_edit]).as_slice()).unwrap();
    let mut remap_staff_scale_cancel = create_dialog_control_template(BS_PUSHBUTTON | WS_VISIBLE,
        60, 70, 30, 10, IDCANCEL as u32, &button_string, &cancel_string);
    let mut remap_staff_scale_ok = create_dialog_control_template(BS_PUSHBUTTON | WS_VISIBLE, 30,
        70, 30, 10, IDOK as u32, &button_string, &ok_string);
    let mut remap_staff_scale_text = create_dialog_control_template(SS_LEFT | WS_CHILD | WS_VISIBLE,
        5, 5, 115, 35, 0, &static_string, &wide_char_string("One or more existing \
        staves use the scale marked for deletion. Choose a new scale for these staves."));
    let mut remap_staff_scale_scale_list = create_dialog_control_template(CBS_DROPDOWNLIST |
        CBS_HASSTRINGS | WS_CHILD | WS_VISIBLE, 5, 40, 110, 100, IDC_REMAP_STAFF_SCALE_LIST as u32,
        &wide_char_string("COMBOBOX"), &empty_string);
    constants_file.write(create_dialog_template_constant(
        b"REMAP_STAFF_SCALE_DIALOG_TEMPLATE".to_vec(), DS_CENTER, 0, 0, 125, 85,
        wide_char_string("Remap Staff Scale"), font_info.clone(),
        vec![&mut remap_staff_scale_cancel, &mut remap_staff_scale_ok, &mut remap_staff_scale_text,
        &mut remap_staff_scale_scale_list]).as_slice()).unwrap();
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
    write_serde_float_to_struct(&mut constants_file, "leger_line_extension",
        &engraving_defaults["legerLineExtension"]);
    write_serde_float_to_struct(&mut constants_file, "leger_line_thickness",
        &engraving_defaults["legerLineThickness"]);
    write_serde_float_to_struct(&mut constants_file, "staff_line_thickness",
        &engraving_defaults["staffLineThickness"]);
    write_serde_float_to_struct(&mut constants_file, "stem_thickness",
        &engraving_defaults["stemThickness"]);
    write_serde_float_to_struct(&mut constants_file, "thin_barline_thickness",
        &engraving_defaults["thinBarlineThickness"]);
    constants_file.write(b"};\n");
}

fn write_constant<T: ToString>(file: &mut File, name_and_type: &str, value: T)
{
    file.write(b"static ");
    file.write(name_and_type.as_bytes());
    file.write(b" = ");
    file.write(value.to_string().as_bytes());
    file.write(b";\n");
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