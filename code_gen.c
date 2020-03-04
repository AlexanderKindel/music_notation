#include "shared_declarations.h"

struct Control
{
    wchar_t*class;
    wchar_t*text;
    DWORD style;
    WORD id;
};

void write_string(bool*dword_aligned, FILE*file, wchar_t*string)
{
    wchar_t*next_char = string;
    while (*next_char)
    {
        fprintf(file, "%u, ", *next_char);
        ++next_char;
    }
    fputs("0, ", file);
    *dword_aligned = ((size_t)(next_char - string) % 2 == 0) ^ *dword_aligned;
}

void write_control(bool*dword_aligned, FILE*file, struct Control*control)
{
    if (*dword_aligned)
    {
        *dword_aligned = false;
    }
    else
    {
        fputs("0, ", file);
    }
    fprintf(file, "%u, %u, 0, 0, 0, 0, 0, 0, %u, ", *(wchar_t*)&control->style,
        *(((wchar_t*)&control->style) + 1), control->id);
    if (control->class[0] == 0xffff)
    {
        fprintf(file, "0xffff, %u, ", control->class[1]);
    }
    else
    {
        write_string(dword_aligned, file, control->class);
    }
    write_string(dword_aligned, file, control->text);
    *dword_aligned = !*dword_aligned;
    fputs("0", file);
}

void write_template(FILE*file, char*template_name, wchar_t*dialog_name, WORD control_count,
    struct Control*controls)
{
    fprintf(file, "struct DialogTemplate %s = { 0, 0, %u, 0, 0, 0, 0, 0, 0, ", template_name,
        control_count);
    bool dword_aligned = false;
    write_string(&dword_aligned, file, dialog_name);
    for (WORD i = 0; i < control_count - 1; ++i)
    {
        write_control(&dword_aligned, file, controls + i);
        fputs(", ", file);
    }
    write_control(&dword_aligned, file, controls + control_count - 1);
    fputs(" };\n\n", file);
}

#define WRITE_TEMPLATE(file, template_name, dialog_name, controls)\
write_template(file, template_name, dialog_name, sizeof(controls) / sizeof(struct Control), controls)

int main()
{
    FILE*meta_declarations = fopen("generated_declarations.h", "w");
    fputs("#ifndef GENERATED_DECLARATIONS_H\n"
        "#define GENERATED_DECLARATIONS_H\n\n"
        "#include \"shared_declarations.h\"\n\n",
        meta_declarations);
    char*pool_element_types[5] = { "ADDRESS_NODE", "INTEGER", "POINTER", "SLICE" };
    uint32_t pool_element_sizes[5] = { sizeof(struct AddressNode), get_integer_size(1),
        sizeof(void*), sizeof(struct Slice), sizeof(struct Staff) };
    uint32_t pool_count = 0;
    for (uint32_t i = 0; i < 4; ++i)
    {
        uint32_t j = 0;
        while (true)
        {
            if (j == i)
            {
                ++pool_count;
                break;
            }
            if (pool_element_sizes[j] == pool_element_sizes[i])
            {
                pool_element_sizes[i] = 0;
                break;
            }
            ++j;
        }
        fprintf(meta_declarations, "#define %s_POOL(project) project->other_pools[%u]\n",
            pool_element_types[i], pool_count - 1);
    }
    fprintf(meta_declarations, "\nuint32_t pool_element_sizes[%u] = { ", pool_count);
    for (uint32_t i = 0; i < 4; ++i)
    {
        if (pool_element_sizes[i])
        {
            fprintf(meta_declarations, "%u, ", pool_element_sizes[i]);
        }
    }
    fprintf(meta_declarations, "};\n\nstruct Project\n"
        "{\n"
        "    struct Pool page_pool;\n"
        "    struct Pool staff_pool;\n"
        "    struct Pool other_pools[%u];\n"
        "    struct Stack stack_a;\n"
        "    struct Stack stack_b;\n"
        "    struct Selection selection;\n"
        "    struct Page*slices;\n"
        "    struct Page*staff_scales;\n"
        "    struct StaffObjectAddress ghost_cursor_address;\n"
        "    HWND control_tabs_handle;\n"
        "    HWND staff_tab_handle;\n"
        "    HWND add_staff_button_handle;\n"
        "    HWND edit_staff_scales_button_handle;\n"
        "    HWND clef_tab_handle;\n"
        "    HWND c_clef_handle;\n"
        "    HWND f_clef_handle;\n"
        "    HWND g_clef_handle;\n"
        "    HWND clef_15ma_handle;\n"
        "    HWND clef_8va_handle;\n"
        "    HWND clef_none_handle;\n"
        "    HWND clef_8vb_handle;\n"
        "    HWND clef_15mb_handle;\n"
        "    HWND add_clef_button_handle;\n"
        "    HWND key_sig_tab_handle;\n"
        "    HWND accidental_count_spin_handle;\n"
        "    HWND sharps_handle;\n"
        "    HWND flats_handle;\n"
        "    HWND add_key_sig_button_handle;\n"
        "    HWND time_sig_tab_handle;\n"
        "    HWND numerator_spin_handle;\n"
        "    HWND denominator_display_handle;\n"
        "    HWND denominator_spin_handle;\n"
        "    HWND add_time_sig_button_handle;\n"
        "    HWND note_tab_handle;\n"
        "    HWND duration_display_handle;\n"
        "    HWND duration_spin_handle;\n"
        "    HWND augmentation_dot_spin_handle;\n"
        "    HBITMAP main_window_back_buffer;\n"
        "    POINT uz_viewport_offset;\n"
        "    float uz_default_staff_space_height;\n"
        "    int32_t utuz_x_of_slice_beyond_leftmost_to_draw;\n"
        "    int32_t utuz_y_of_staff_above_highest_visible;\n"
        "    int32_t utuz_last_slice_x;\n"
        "    int32_t utuz_bottom_staff_y;\n"
        "    uint32_t topmost_staff_index;\n"
        "    uint32_t bottommost_staff_index;\n"
        "    uint32_t highest_visible_staff_index;\n"
        "    uint32_t address_of_leftmost_slice_to_draw;\n"
        "    int8_t zoom_exponent;\n"
        "};\n\n",
        pool_count);
    struct Control edit_staff_scales_controls[] =
    {
        {
            L"COMBOBOX", L"",
            CBS_DROPDOWNLIST | CBS_HASSTRINGS | WS_CHILD | WS_VISIBLE | WS_VSCROLL,
            IDC_EDIT_SCALES_SCALE_LIST
        },
        {
            (wchar_t[]) { 0xffff, 0x80 },
            EDIT_SCALES_ADD_SCALE_STRING, BS_PUSHBUTTON | WS_VISIBLE, IDC_EDIT_SCALES_ADD_SCALE
        },
        {
            (wchar_t[]) { 0xffff, 0x80 },
            EDIT_SCALES_EDIT_SCALE_STRING, BS_PUSHBUTTON | WS_VISIBLE, IDC_EDIT_SCALES_EDIT_SCALE
        },
        {
            (wchar_t[]) { 0xffff, 0x80 },
            EDIT_SCALES_REMOVE_SCALE_STRING, BS_PUSHBUTTON | WS_VISIBLE,
            IDC_EDIT_SCALES_REMOVE_SCALE
        },
        {
            (wchar_t[]) { 0xffff, 0x80 },
            OK_STRING, BS_PUSHBUTTON | WS_VISIBLE, IDOK
        },
        {
            (wchar_t[]) { 0xffff, 0x80 },
            CANCEL_STRING, BS_PUSHBUTTON | WS_VISIBLE, IDCANCEL
        }
    };
    WRITE_TEMPLATE(meta_declarations, "EDIT_STAFF_SCALES_DIALOG_TEMPLATE", L"Edit Staff Scales",
        edit_staff_scales_controls);
    struct Control add_staff_controls[] =
    {
        {
            (wchar_t[]) { 0xffff, 0x82 },
            ADD_STAFF_LINE_COUNT_LABEL_STRING, SS_SIMPLE | WS_CHILD | WS_VISIBLE,
            IDC_ADD_STAFF_LINE_COUNT_LABEL
        },
        {
            (wchar_t[]) { 0xffff, 0x82 },
            L"5", WS_BORDER | WS_CHILD | WS_VISIBLE, IDC_ADD_STAFF_LINE_COUNT_DISPLAY
        },
        {
            UPDOWN_CLASSW, L"",
            UDS_AUTOBUDDY | UDS_SETBUDDYINT | WS_CHILD | WS_VISIBLE, IDC_ADD_STAFF_LINE_COUNT_SPIN
        },
        {
            (wchar_t[]) { 0xffff, 0x82 },
            ADD_STAFF_SCALE_LABEL_STRING, SS_SIMPLE | WS_CHILD | WS_VISIBLE,
            IDC_ADD_STAFF_SCALE_LABEL
        },
        {
            L"COMBOBOX", L"",
            CBS_DROPDOWNLIST | CBS_HASSTRINGS | WS_CHILD | WS_VISIBLE | WS_VSCROLL,
            IDC_ADD_STAFF_SCALE_LIST
        },
        {
            (wchar_t[]) { 0xffff, 0x80 },
            EDIT_SCALES_STRING, BS_PUSHBUTTON | WS_VISIBLE, IDC_ADD_STAFF_EDIT_SCALES
        },
        {
            (wchar_t[]) { 0xffff, 0x80 },
            OK_STRING, BS_PUSHBUTTON | WS_VISIBLE, IDOK
        },
        {
            (wchar_t[]) { 0xffff, 0x80 },
            CANCEL_STRING, BS_PUSHBUTTON | WS_VISIBLE, IDCANCEL
        }
    };
    WRITE_TEMPLATE(meta_declarations, "ADD_STAFF_DIALOG_TEMPLATE", L"Add Staff",
        add_staff_controls);
    struct Control edit_staff_scale_controls[] =
    {
        {
            (wchar_t[]) { 0xffff, 0x82 },
            EDIT_STAFF_NAME_LABEL_STRING, SS_SIMPLE | WS_CHILD | WS_VISIBLE,
            IDC_EDIT_STAFF_SCALE_NAME_LABEL
        },
        {
            (wchar_t[]) { 0xffff, 0x81 },
            L"", WS_BORDER | WS_CHILD | WS_VISIBLE, IDC_EDIT_STAFF_SCALE_NAME
        },
        {
            (wchar_t[]) { 0xffff, 0x82 },
            EDIT_STAFF_VALUE_LABEL_STRING, SS_SIMPLE | WS_CHILD | WS_VISIBLE,
            IDC_EDIT_STAFF_SCALE_VALUE_LABEL
        },
        {
            (wchar_t[]) { 0xffff, 0x81 },
            L"", WS_BORDER | WS_CHILD | WS_VISIBLE, IDC_EDIT_STAFF_SCALE_VALUE
        },
        {
            (wchar_t[]) { 0xffff, 0x80 },
            OK_STRING, BS_PUSHBUTTON | WS_VISIBLE, IDOK
        }
    };
    WRITE_TEMPLATE(meta_declarations, "EDIT_STAFF_SCALE_DIALOG_TEMPLATE", L"Edit Staff Scale",
        edit_staff_scale_controls);
    struct Control remap_staff_scale_controls[] =
    {
        {
            (wchar_t[]) { 0xffff, 0x82 },
            L"One or more existing staves use the scale marked for deletion. Choose a new scale "
            "for these staves.", SS_LEFT | WS_CHILD | WS_VISIBLE, IDC_REMAP_STAFF_SCALE_MESSAGE
        },
        {
            L"COMBOBOX", L"", CBS_DROPDOWNLIST | CBS_HASSTRINGS | WS_CHILD | WS_VISIBLE,
            IDC_REMAP_STAFF_SCALE_LIST
        },
        {
            (wchar_t[]) { 0xffff, 0x80 },
            OK_STRING, BS_PUSHBUTTON | WS_VISIBLE, IDOK
        },
        {
            (wchar_t[]) { 0xffff, 0x80 },
            CANCEL_STRING, BS_PUSHBUTTON | WS_VISIBLE, IDCANCEL
        }
    };
    WRITE_TEMPLATE(meta_declarations, "REMAP_STAFF_SCALE_DIALOG_TEMPLATE", L"Remap Staff Scale",
        remap_staff_scale_controls);
    fputs("#endif", meta_declarations);
    fclose(meta_declarations);
    return 0;
}