#include "shared_declarations.h"

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
        "    struct StaffScale staff_scales[MAX_STAFF_SCALE_COUNT];\n"
        "    struct Pool page_pool;\n"
        "    struct Pool staff_pool;\n"
        "    struct Pool other_pools[%u];\n"
        "    struct Stack stack_a;\n"
        "    struct Stack stack_b;\n"
        "    struct Selection selection;\n"
        "    struct Page*slices;\n"
        "    struct StaffObjectAddress ghost_cursor_address;\n"
        "    HWND control_tabs_handle;\n"
        "    HWND staff_tab_handle;\n"
        "    HWND add_staff_button_handle;\n"
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
        "};\n\n#endif",
        pool_count);
    fclose(meta_declarations);
    return 0;
}