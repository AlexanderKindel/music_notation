#include "declarations.h"
#include "respace_slices.c"
#include "viewport.c"

//Coordinates have two properties, zoom and translation, which are marked on identifiers using a
//pair of prefixes. "t" means "translated," "z" means "zoomed," and either of these properties is
//negated by beginning the prefix with "u". For example, tuz_x refers to a coordinate that is
//translated but unzoomed.

//Distances are also identified with a zoom prefix, as are things that are associated with a size,
//like fonts. Coordinates involved in the context of distance and size calculations may also omit
//the translation prefix since distances are invariant under translation.

//Any coordinate that is to be mapped to the screen must be both translated and zoomed. Generally,
//coordinates that are stored as part of the project state are stored untranslated and unzoomed.
//Translation is to be applied first, then zooming.

int32_t float_round(float a)
{
    return a + 0.5;
}

float get_zoom_factor(int8_t zoom_exponent)
{
    float out = 1.0;
    float base;
    if (zoom_exponent < 0)
    {
        zoom_exponent = -zoom_exponent;
        base = 1.0 / 1.1;
    }
    else
    {
        base = 1.1;
    }
    if (zoom_exponent)
    {
        while (true)
        {
            if (zoom_exponent % 2)
            {
                out *= base;
            }
            zoom_exponent = zoom_exponent >> 1;
            if (!zoom_exponent)
            {
                break;
            }
            base *= base;
        }
    }
    return out;
}

int32_t zoom_coordinate(float tuz_coordinate, float zoom_factor)
{
    return float_round(zoom_factor * tuz_coordinate);
}

int32_t unzoom_coordinate(float tz_coordinate, float zoom_factor)
{
    return float_round(tz_coordinate / zoom_factor);
}

void get_work_region_rect(HWND main_window_handle, struct Project*project, RECT*out)
{
    GetClientRect(main_window_handle, out);
    RECT control_tabs_rect;
    GetWindowRect(project->control_tabs_handle, &control_tabs_rect);
    MapWindowPoints(GetDesktopWindow(), main_window_handle, (POINT*)&control_tabs_rect.right, 1);
    out->top = control_tabs_rect.bottom;
}

void invalidate_work_region(HWND main_window_handle, struct Project*project)
{
    RECT work_region_rect;
    get_work_region_rect(main_window_handle, project, &work_region_rect);
    InvalidateRect(main_window_handle, &work_region_rect, FALSE);
}

HFONT get_staff_font(float staff_space_height, float staff_height_multiple)
{
    return CreateFontW(float_round(-4.0 * staff_height_multiple * staff_space_height), 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, L"Bravura");
}

void get_staff_font_set(struct FontSet*out, float staff_space_height)
{
    out->full_size = get_staff_font(staff_space_height, 1.0);
    out->two_thirds_size = get_staff_font(staff_space_height, 2.0 / 3.0);
}

void release_font_set(struct FontSet*font_set)
{
    DeleteObject(font_set->full_size);
    DeleteObject(font_set->two_thirds_size);
}

int32_t get_character_width(HDC device_context, HFONT font, uint32_t codepoint)
{
    HFONT old_font = (HFONT)SelectObject(device_context, font);
    ABC abc_array;
    GetCharABCWidthsW(device_context, codepoint, codepoint, &abc_array);
    SelectObject(device_context, old_font);
    return abc_array.abcB;
}

int32_t get_string_width(HDC device_context, HFONT font, wchar_t*string, uint8_t string_length)
{
    HFONT old_font = (HFONT)SelectObject(device_context, font);
    SIZE size;
    GetTextExtentPoint32W(device_context, string, string_length, &size);
    SelectObject(device_context, old_font);
    return size.cx;
}

void draw_character(HDC device_context, HFONT z_font, float tuz_x, float tuz_y, float zoom_factor,
    uint16_t codepoint)
{
    HFONT old_font = (HFONT)SelectObject(device_context, z_font);
    TextOutW(device_context, zoom_coordinate(tuz_x, zoom_factor),
        zoom_coordinate(tuz_y, zoom_factor), (LPCWSTR)&codepoint, 1);
    SelectObject(device_context, old_font);
}

struct VerticalInterval get_tz_horizontal_line_vertical_bounds(float tuz_vertical_center,
    float uz_thickness, float zoom_factor)
{
    float tuz_bottom = tuz_vertical_center + uz_thickness / 2.0;
    int32_t tz_top = zoom_coordinate(tuz_bottom - uz_thickness, zoom_factor);
    int32_t tz_bottom = zoom_coordinate(tuz_bottom, zoom_factor);
    if (tz_top == tz_bottom)
    {
        tz_top -= 1;
    }
    return (struct VerticalInterval) { tz_top, tz_bottom };
}

void draw_horizontal_line(HDC device_context, int32_t tuz_left_edge, int32_t tuz_right_edge,
    float tuz_vertical_center, float uz_thickness, float zoom_factor)
{
    struct VerticalInterval tz_vertical_bounds =
        get_tz_horizontal_line_vertical_bounds(tuz_vertical_center, uz_thickness, zoom_factor);
    Rectangle(device_context, zoom_coordinate(tuz_left_edge, zoom_factor),
        tz_vertical_bounds.top, zoom_coordinate(tuz_right_edge, zoom_factor),
        tz_vertical_bounds.bottom);
}

void draw_object(struct FontSet*z_font_set, HDC device_context, int8_t*staff_middle_pitch,
    struct Object*object, struct Project*project, struct Staff*staff, float uz_staff_space_height,
    float zoom_factor, int32_t tuz_staff_middle_y, int32_t tuz_x)
{
    if (object->is_hidden)
    {
        return;
    }
    switch (object->object_type)
    {
    case OBJECT_ACCIDENTAL:
    {
        struct Pitch note_pitch = ((struct Object*)resolve_address(project,
            object->accidental_note_address))->duration.pitch.pitch;
        draw_character(device_context, z_font_set->full_size, tuz_x,
            get_tuz_y_of_staff_relative_step(tuz_staff_middle_y, uz_staff_space_height,
                staff->line_count,
                note_pitch.steps_above_c4 - *staff_middle_pitch + staff->line_count - 1),
            zoom_factor, get_accidental_codepoint(note_pitch.accidental));
        return;
    }
    case OBJECT_BARLINE:
    {
        struct VerticalInterval tz_vertical_bounds =
            get_tz_staff_vertical_bounds(uz_staff_space_height, zoom_factor, tuz_staff_middle_y,
                staff->line_count);
        Rectangle(device_context, zoom_coordinate(tuz_x, zoom_factor), tz_vertical_bounds.top,
            zoom_coordinate(tuz_x +
                project->uz_default_staff_space_height * BRAVURA_METADATA.uz_thin_barline_thickness,
                zoom_factor),
            tz_vertical_bounds.bottom);
        return;
    }
    case OBJECT_CLEF:
    {
        *staff_middle_pitch = get_staff_middle_pitch(&object->clef);
        HFONT font;
        if (object->slice_address == HEADER_CLEF_SLICE_ADDRESS)
        {
            font = z_font_set->full_size;
        }
        else
        {
            font = z_font_set->two_thirds_size;
        }
        draw_character(device_context, font, tuz_x,
            get_tuz_y_of_staff_relative_step(tuz_staff_middle_y, uz_staff_space_height,
                staff->line_count,
                staff->line_count - 1 + object->clef.steps_of_baseline_above_staff_middle),
            zoom_factor, object->clef.codepoint);
        return;
    }
    case OBJECT_DURATION:
    {
        uint16_t duration_codepoint = get_duration_codepoint(&object->duration);
        HFONT uz_font = get_staff_font(uz_staff_space_height, 1.0);
        int32_t tuz_duration_right_edge;
        float tuz_duration_y;
        float tuz_augmentation_dot_y;
        if (object->duration.is_pitched)
        {
            int8_t steps_above_bottom_line = object->duration.pitch.pitch.steps_above_c4 -
                get_staff_bottom_line_pitch(staff->line_count, *staff_middle_pitch);
            tuz_duration_y = get_tuz_y_of_staff_relative_step(tuz_staff_middle_y,
                uz_staff_space_height, staff->line_count, steps_above_bottom_line);
            if (steps_above_bottom_line % 2)
            {
                tuz_augmentation_dot_y = tuz_duration_y;
            }
            else
            {
                tuz_augmentation_dot_y = get_tuz_y_of_staff_relative_step(tuz_staff_middle_y,
                    uz_staff_space_height, staff->line_count, steps_above_bottom_line + 1);
            }
            if (object->duration.log2 < 0)
            {
                float tuz_stem_left_edge;
                float tuz_stem_right_edge;
                float tuz_stem_bottom;
                float tuz_stem_top;
                uint8_t space_count = staff->line_count - 1;
                if (space_count > steps_above_bottom_line)
                {
                    tuz_stem_top = get_tuz_y_of_staff_relative_step(tuz_staff_middle_y,
                        uz_staff_space_height, staff->line_count,
                        max(steps_above_bottom_line + 7, space_count));
                    if (object->duration.log2 == -1)
                    {
                        tuz_stem_right_edge = tuz_x +
                            uz_staff_space_height * BRAVURA_METADATA.uz_half_notehead_stem_up_se.x;
                        tuz_stem_left_edge = tuz_stem_right_edge -
                            uz_staff_space_height * BRAVURA_METADATA.uz_stem_thickness;
                        tuz_stem_bottom = tuz_duration_y -
                            uz_staff_space_height * BRAVURA_METADATA.uz_half_notehead_stem_up_se.y;
                    }
                    else
                    {
                        tuz_stem_right_edge = tuz_x +
                            uz_staff_space_height * BRAVURA_METADATA.uz_black_notehead_stem_up_se.x;
                        tuz_stem_left_edge = tuz_stem_right_edge -
                            uz_staff_space_height * BRAVURA_METADATA.uz_stem_thickness;
                        tuz_stem_bottom = tuz_duration_y -
                            uz_staff_space_height * BRAVURA_METADATA.uz_black_notehead_stem_up_se.y;
                        if (object->duration.log2 == -3)
                        {
                            draw_character(device_context, z_font_set->full_size,
                                tuz_stem_left_edge, tuz_stem_top, zoom_factor, 0xe240);
                        }
                        else if (object->duration.log2 < -3)
                        {
                            draw_character(device_context, z_font_set->full_size,
                                tuz_stem_left_edge, tuz_stem_top, zoom_factor, 0xe242);
                            float uz_flag_spacing = uz_staff_space_height *
                                (BRAVURA_METADATA.uz_beam_spacing +
                                    BRAVURA_METADATA.uz_beam_thickness);
                            for (int8_t i = 0; i < -object->duration.log2 - 4; ++i)
                            {
                                tuz_stem_top -= uz_flag_spacing;
                                draw_character(device_context, z_font_set->full_size,
                                    tuz_stem_left_edge, tuz_stem_top, zoom_factor, 0xe250);
                            }
                        }
                    }
                }
                else
                {
                    tuz_stem_bottom = get_tuz_y_of_staff_relative_step(tuz_staff_middle_y,
                        uz_staff_space_height, staff->line_count,
                        min(steps_above_bottom_line - 7, space_count));
                    if (object->duration.log2 == -1)
                    {
                        tuz_stem_left_edge = tuz_x + uz_staff_space_height *
                            BRAVURA_METADATA.uz_half_notehead_stem_down_nw.x;
                        tuz_stem_top = tuz_duration_y - uz_staff_space_height *
                            BRAVURA_METADATA.uz_half_notehead_stem_down_nw.y;
                    }
                    else
                    {
                        tuz_stem_left_edge = tuz_x + uz_staff_space_height *
                            BRAVURA_METADATA.uz_black_notehead_stem_down_nw.x;
                        tuz_stem_top = tuz_duration_y - uz_staff_space_height *
                            BRAVURA_METADATA.uz_black_notehead_stem_down_nw.y;
                        if (object->duration.log2 == -3)
                        {
                            draw_character(device_context, z_font_set->full_size,
                                tuz_stem_left_edge, tuz_stem_bottom, zoom_factor, 0xe241);
                        }
                        else if (object->duration.log2 < -3)
                        {
                            draw_character(device_context, z_font_set->full_size,
                                tuz_stem_left_edge, tuz_stem_bottom, zoom_factor, 0xe243);
                            float uz_flag_spacing = uz_staff_space_height *
                                (BRAVURA_METADATA.uz_beam_spacing +
                                    BRAVURA_METADATA.uz_beam_thickness);
                            for (uint8_t i = 0; i < -object->duration.log2 - 4; ++i)
                            {
                                tuz_stem_bottom += uz_flag_spacing;
                                draw_character(device_context, z_font_set->full_size,
                                    tuz_stem_left_edge, tuz_stem_bottom, zoom_factor, 0xe251);
                            }
                        }
                    }
                    tuz_stem_right_edge = tuz_stem_left_edge +
                        uz_staff_space_height * BRAVURA_METADATA.uz_stem_thickness;
                }
                Rectangle(device_context, zoom_coordinate(tuz_stem_left_edge, zoom_factor),
                    zoom_coordinate(tuz_stem_top, zoom_factor),
                    zoom_coordinate(tuz_stem_right_edge, zoom_factor),
                    zoom_coordinate(tuz_stem_bottom, zoom_factor));
            }
            tuz_duration_right_edge =
                tuz_x + get_character_width(device_context, uz_font, duration_codepoint);
            float uz_leger_extension =
                uz_staff_space_height * BRAVURA_METADATA.uz_leger_line_extension;
            float uz_leger_thickness =
                uz_staff_space_height * BRAVURA_METADATA.uz_leger_line_thickness;
            float tuz_leger_left_edge = tuz_x - uz_leger_extension;
            float tuz_leger_right_edge = tuz_duration_right_edge + uz_leger_extension;
            if (steps_above_bottom_line < -1)
            {
                for (int8_t i = steps_above_bottom_line / 2; i < 0; ++i)
                {
                    draw_horizontal_line(device_context, tuz_leger_left_edge, tuz_leger_right_edge,
                        get_tuz_y_of_staff_relative_step(tuz_staff_middle_y, uz_staff_space_height,
                            staff->line_count, 2 * i),
                        uz_leger_thickness, zoom_factor);
                }
            }
            else if (steps_above_bottom_line >= 2 * staff->line_count)
            {
                for (int8_t i = staff->line_count; i <= steps_above_bottom_line / 2; ++i)
                {
                    draw_horizontal_line(device_context, tuz_leger_left_edge, tuz_leger_right_edge,
                        get_tuz_y_of_staff_relative_step(tuz_staff_middle_y, uz_staff_space_height,
                            staff->line_count, 2 * i),
                        uz_leger_thickness, zoom_factor);
                }
            }
        }
        else
        {
            uint8_t spaces_above_bottom_line;
            if (object->duration.log2)
            {
                spaces_above_bottom_line = (staff->line_count + 1) / 2 - 1;
            }
            else
            {
                if (staff->line_count == 1)
                {
                    spaces_above_bottom_line = 0;
                }
                else
                {
                    spaces_above_bottom_line = (staff->line_count + 1) / 2;
                }
            }
            tuz_duration_right_edge =
                tuz_x + get_character_width(device_context, uz_font, duration_codepoint);
            tuz_duration_y = get_tuz_y_of_staff_relative_step(tuz_staff_middle_y,
                uz_staff_space_height, staff->line_count, 2 * spaces_above_bottom_line);
            tuz_augmentation_dot_y = get_tuz_y_of_staff_relative_step(tuz_staff_middle_y,
                uz_staff_space_height, staff->line_count, 2 * spaces_above_bottom_line + 1);
        }
        float uz_dot_separation = uz_staff_space_height * UZ_DISTANCE_BETWEEN_AUGMENTATION_DOTS;
        float tuz_next_dot_left_edge = tuz_duration_right_edge + uz_dot_separation;
        float uz_dot_offset =
            uz_dot_separation + get_character_width(device_context, uz_font, 0xe1e7);
        draw_character(device_context, z_font_set->full_size, tuz_x, tuz_duration_y, zoom_factor,
            duration_codepoint);
        for (uint8_t i = 0; i < object->duration.augmentation_dot_count; ++i)
        {
            draw_character(device_context, z_font_set->full_size, tuz_next_dot_left_edge,
                tuz_augmentation_dot_y, zoom_factor, 0xe1e7);
            tuz_next_dot_left_edge += uz_dot_offset;
        }
        DeleteObject(uz_font);
        return;
    }
    case OBJECT_KEY_SIG:
    {
        int8_t middle_line_letter_name = *staff_middle_pitch % 7;
        int8_t floor = object->key_sig.floors[middle_line_letter_name];
        int32_t tuz_accidental_x = tuz_x;
        for (uint8_t i = 0; i < object->key_sig.accidental_count; ++i)
        {
            if (!object->key_sig.accidentals[i].accidental)
            {
                break;
            }
            int8_t steps_above_middle_line =
                object->key_sig.accidentals[i].letter_name - middle_line_letter_name;
            if (steps_above_middle_line < floor)
            {
                steps_above_middle_line += 7;
            }
            else if (steps_above_middle_line >= floor + 7)
            {
                steps_above_middle_line -= 7;
            }
            uint32_t accidental_codepoint =
                get_accidental_codepoint(object->key_sig.accidentals[i].accidental);
            draw_character(device_context, z_font_set->full_size, tuz_accidental_x,
                get_tuz_y_of_staff_relative_step(tuz_staff_middle_y, uz_staff_space_height,
                    staff->line_count, steps_above_middle_line + staff->line_count - 1),
                zoom_factor, accidental_codepoint);
            tuz_accidental_x += get_character_width(device_context, z_font_set->full_size,
                accidental_codepoint);
        }
        return;
    }
    case OBJECT_TIME_SIG:
    {
        struct TimeSigStrings strings;
        time_sig_to_strings(&strings, object->time_sig);
        int32_t z_numerator_width = get_string_width(device_context, z_font_set->full_size,
            strings.numerator_string, strings.numerator_string_length);
        int32_t z_denominator_width = get_string_width(device_context, z_font_set->full_size,
            strings.denominator_string, strings.denominator_string_length);
        int32_t z_numerator_x_offset;
        int32_t z_denominator_x_offset;
        if (z_numerator_width > z_denominator_width)
        {
            z_numerator_x_offset = 0;
            z_denominator_x_offset = (z_numerator_width - z_denominator_width) / 2;
        }
        else
        {
            z_numerator_x_offset = (z_denominator_width - z_numerator_width) / 2;
            z_denominator_x_offset = 0;
        }
        int32_t tz_x = zoom_coordinate(tuz_x, zoom_factor);
        HFONT old_font = (HFONT)SelectObject(device_context, z_font_set->full_size);
        TextOutW(device_context, tz_x + z_numerator_x_offset,
            zoom_coordinate(tuz_staff_middle_y - uz_staff_space_height, zoom_factor),
            strings.numerator_string, strings.numerator_string_length);
        TextOutW(device_context, tz_x + z_denominator_x_offset,
            zoom_coordinate(tuz_staff_middle_y + uz_staff_space_height, zoom_factor),
            strings.denominator_string, strings.denominator_string_length);
        SelectObject(device_context, old_font);
    }
    }
}

void draw_object_with_selection(struct FontSet*z_font_set, HDC device_context,
    int8_t*staff_middle_pitch, struct Object*object, struct Project*project, struct Staff*staff,
    float uz_staff_space_height, float zoom_factor, int32_t tuz_staff_middle_y, int32_t tuz_x)
{
    if (object->is_selected)
    {
        SetTextColor(device_context, RED);
        draw_object(z_font_set, device_context, staff_middle_pitch, object, project, staff,
            uz_staff_space_height, zoom_factor, tuz_staff_middle_y, tuz_x);
        SetTextColor(device_context, BLACK);
    }
    else
    {
        draw_object(z_font_set, device_context, staff_middle_pitch, object, project, staff,
            uz_staff_space_height, zoom_factor, tuz_staff_middle_y, tuz_x);
    }
    if (project->selection.selection_type == SELECTION_CURSOR &&
        project->selection.address.object_address == object->address)
    {
        SaveDC(device_context);
        SelectObject(device_context, RED_PEN);
        SelectObject(device_context, RED_BRUSH);
        int8_t steps_of_floor_above_bottom_line = project->selection.range_floor -
            get_staff_bottom_line_pitch(staff->line_count, *staff_middle_pitch);
        float tuz_range_indicator_bottom = get_tuz_y_of_staff_relative_step(tuz_staff_middle_y,
            uz_staff_space_height, staff->line_count, steps_of_floor_above_bottom_line);
        float tuz_range_indicator_top = get_tuz_y_of_staff_relative_step(tuz_staff_middle_y,
            uz_staff_space_height, staff->line_count, steps_of_floor_above_bottom_line + 6);
        float tuz_range_indicator_right_edge = tuz_x + uz_staff_space_height;
        float uz_line_thickness = uz_staff_space_height * BRAVURA_METADATA.uz_staff_line_thickness;
        draw_horizontal_line(device_context, tuz_x, tuz_range_indicator_right_edge,
            tuz_range_indicator_bottom, uz_line_thickness, zoom_factor);
        draw_horizontal_line(device_context, tuz_x, tuz_range_indicator_right_edge,
            tuz_range_indicator_top, uz_line_thickness, zoom_factor);
        int32_t tuz_leger_left_edge = tuz_x - uz_staff_space_height;
        int32_t tuz_cursor_bottom;
        if (steps_of_floor_above_bottom_line < 0)
        {
            for (int8_t i = steps_of_floor_above_bottom_line / 2; i < 0; ++i)
            {
                draw_horizontal_line(device_context, tuz_leger_left_edge, tuz_x,
                    get_tuz_y_of_staff_relative_step(tuz_staff_middle_y, uz_staff_space_height,
                        staff->line_count, 2 * i),
                    uz_line_thickness, zoom_factor);
            }
            tuz_cursor_bottom = tuz_range_indicator_bottom;
        }
        else
        {
            tuz_cursor_bottom = get_tuz_y_of_staff_relative_step(tuz_staff_middle_y,
                uz_staff_space_height, staff->line_count, 0);
        }
        int8_t steps_of_ceiling_above_bottom_line = steps_of_floor_above_bottom_line + 6;
        float tuz_cursor_top;
        uint8_t twice_space_count = 2 * (staff->line_count - 1);
        if (steps_of_ceiling_above_bottom_line > twice_space_count)
        {
            for (uint8_t i = staff->line_count; i <= steps_of_ceiling_above_bottom_line / 2; ++i)
            {
                draw_horizontal_line(device_context, tuz_leger_left_edge, tuz_x,
                    get_tuz_y_of_staff_relative_step(tuz_staff_middle_y, uz_staff_space_height,
                        staff->line_count, 2 * i),
                    uz_line_thickness, zoom_factor);
            }
            tuz_cursor_top = tuz_range_indicator_top;
        }
        else
        {
            tuz_cursor_top = get_tuz_y_of_staff_relative_step(tuz_staff_middle_y,
                uz_staff_space_height, staff->line_count, twice_space_count);
        }
        float tz_cursor_left_edge = zoom_coordinate(tuz_x, zoom_factor);
        Rectangle(device_context, tz_cursor_left_edge, zoom_coordinate(tuz_cursor_top, zoom_factor),
            tz_cursor_left_edge + 1, zoom_coordinate(tuz_cursor_bottom, zoom_factor));
        RestoreDC(device_context, -1);
    }
    else if (project->ghost_cursor_address.object_address == object->address)
    {
        SaveDC(device_context);
        SelectObject(device_context, GRAY_PEN);
        SelectObject(device_context, (HGDIOBJ)GRAY_BRUSH);
        struct VerticalInterval tz_vertical_bounds =
            get_tz_staff_vertical_bounds(project->uz_default_staff_space_height *
                ((struct StaffScale*)resolve_address(project, staff->scale_address))->value,
                zoom_factor, tuz_staff_middle_y, staff->line_count);
        int32_t tz_left_edge = zoom_coordinate(tuz_x, zoom_factor);
        Rectangle(device_context, tz_left_edge, tz_vertical_bounds.top, tz_left_edge + 1,
            tz_vertical_bounds.bottom);
        RestoreDC(device_context, -1);
    }
}

uint32_t get_address_of_clicked_staff_object(HDC back_buffer_device_context, struct Project*project,
    struct Staff*staff, float zoom_factor, int32_t staff_middle_y, int32_t tz_mouse_x,
    int32_t tz_mouse_y)
{
    float uz_space_height = project->uz_default_staff_space_height *
        ((struct StaffScale*)resolve_address(project, staff->scale_address))->value;
    struct FontSet z_font_set;
    get_staff_font_set(&z_font_set, zoom_factor * uz_space_height);
    int8_t staff_middle_pitch = get_staff_middle_pitch_at_viewport_left_edge(project, staff);
    struct PositionedSliceIter slice_iter;
    initialize_slice_iter_to_t_leftmost_to_draw(&slice_iter, project);
    uint32_t staff_index = get_element_index_in_pool(&project->staff_pool, staff);
    struct ObjectIter object_iter;
    initialize_page_element_iter(&object_iter.base,
        get_leftmost_staff_object_to_draw(&slice_iter, project, staff_index),
        sizeof(struct Object));
    draw_object(&z_font_set, back_buffer_device_context, &staff_middle_pitch,
        object_iter.object, project, staff, uz_space_height, zoom_factor, staff_middle_y,
        slice_iter.uz_slice_x - object_iter.object->uz_distance_to_next_slice);
    if (GetPixel(back_buffer_device_context, tz_mouse_x, tz_mouse_y) == WHITE)
    {
        release_font_set(&z_font_set);
        return object_iter.object->address;
    }
    while (true)
    {
        increment_slice_iter(&project->page_pool, &slice_iter);
        if (!slice_iter.iter.slice)
        {
            break;
        }
        uint32_t node_index = slice_iter.iter.slice->first_object_address_node_index;
        while (node_index)
        {
            struct AddressNode*node = resolve_pool_index(ADDRESS_NODE_POOL(project), node_index);
            if (node->address.staff_index == staff_index)
            {
                while (true)
                {
                    increment_page_element_iter(&object_iter.base, &project->page_pool,
                        sizeof(struct Object));
                    if (!object_iter.object)
                    {
                        return 0;
                    }
                    int32_t tuz_object_x =
                        slice_iter.uz_slice_x - object_iter.object->uz_distance_to_next_slice;
                    if (tz_mouse_x < zoom_coordinate(tuz_object_x, zoom_factor))
                    {
                        release_font_set(&z_font_set);
                        return 0;
                    }
                    draw_object(&z_font_set, back_buffer_device_context, &staff_middle_pitch,
                        object_iter.object, project, staff, uz_space_height, zoom_factor,
                        staff_middle_y, tuz_object_x);
                    if (GetPixel(back_buffer_device_context, tz_mouse_x, tz_mouse_y) == WHITE)
                    {
                        release_font_set(&z_font_set);
                        return object_iter.object->address;
                    }
                    if (object_iter.object->address == node->address.object_address)
                    {
                        break;
                    }
                }
                break;
            }
            node_index = node->index_of_next;
        }
        if (tz_mouse_x < zoom_coordinate(slice_iter.uz_slice_x, zoom_factor))
        {
            release_font_set(&z_font_set);
            return 0;
        }
    }
    release_font_set(&z_font_set);
    return 0;
}

uint16_t get_accidental_codepoint(uint8_t accidental)
{
    switch (accidental)
    {
    case DOUBLE_FLAT:
        return 0xe264;
    case FLAT:
        return 0xe260;
    case NATURAL:
        return 0xe261;
    case SHARP:
        return 0xe262;
    case DOUBLE_SHARP:
        return 0xe263;
    }
    crash("Accidental unrecognized.");
}

uint16_t get_duration_codepoint(struct Duration*duration)
{
    if (duration->is_pitched)
    {
        switch (duration->log2)
        {
        case 1:
            return 0xe0a0;
        case 0:
            return 0xe0a2;
        case -1:
            return 0xe0a3;
        default:
            return 0xe0a4;
        }
    }
    return 0xe4e3 - duration->log2;
}

void time_sig_to_strings(struct TimeSigStrings*out, struct TimeSig time_sig)
{
    out->numerator_string = out->buffer;
    out->denominator_string = out->buffer + 4;
    out->numerator_string_length =
        integer_to_wchar_string(&out->numerator_string, time_sig.numerator, 0xe080, 4);
    out->denominator_string_length =
        integer_to_wchar_string(&out->denominator_string, time_sig.denominator, 0xe080, 4);
}

void draw_staff(HDC device_context, struct Project*project, int32_t tuz_staff_middle_y,
    int32_t tuz_update_region_right_edge, uint32_t staff_index)
{
    struct Staff*staff = resolve_pool_index(&project->staff_pool, staff_index);
    float uz_space_height = project->uz_default_staff_space_height *
        ((struct StaffScale*)resolve_address(project, staff->scale_address))->value;
    float zoom_factor = get_zoom_factor(project->zoom_exponent);
    float uz_staff_line_thickness = uz_space_height * BRAVURA_METADATA.uz_staff_line_thickness;
    struct PositionedSliceIter slice_iter;
    initialize_slice_iter_to_t_leftmost_to_draw(&slice_iter, project);
    int32_t tz_staff_left_edge = zoom_coordinate(slice_iter.uz_slice_x, zoom_factor);
    for (uint8_t line_index = 0; line_index < staff->line_count; ++line_index)
    {
        struct VerticalInterval tz_staff_line_vertical_bounds =
            get_tz_horizontal_line_vertical_bounds(
                get_tuz_y_of_staff_relative_step(tuz_staff_middle_y, uz_space_height,
                    staff->line_count, 2 * line_index),
                uz_staff_line_thickness, zoom_factor);
        Rectangle(device_context, tz_staff_left_edge, tz_staff_line_vertical_bounds.top,
            tuz_update_region_right_edge, tz_staff_line_vertical_bounds.bottom);
    }
    struct FontSet z_font_set;
    get_staff_font_set(&z_font_set, zoom_factor * uz_space_height);
    int8_t staff_middle_pitch = get_staff_middle_pitch_at_viewport_left_edge(project, staff);
    struct ObjectIter object_iter;
    initialize_page_element_iter(&object_iter.base,
        get_leftmost_staff_object_to_draw(&slice_iter, project, staff_index),
        sizeof(struct Object));
    draw_object_with_selection(&z_font_set, device_context, &staff_middle_pitch,
        object_iter.object, project, staff, uz_space_height, zoom_factor, tuz_staff_middle_y,
        slice_iter.uz_slice_x - object_iter.object->uz_distance_to_next_slice);
    while (true)
    {
        increment_slice_iter(&project->page_pool, &slice_iter);
        if (!slice_iter.iter.slice)
        {
            break;
        }
        uint32_t node_index = slice_iter.iter.slice->first_object_address_node_index;
        while (node_index)
        {
            struct AddressNode*node = resolve_pool_index(ADDRESS_NODE_POOL(project), node_index);
            if (node->address.staff_index == staff_index)
            {
                while (true)
                {
                    increment_page_element_iter(&object_iter.base, &project->page_pool,
                        sizeof(struct Object));
                    int32_t tuz_object_x =
                        slice_iter.uz_slice_x - object_iter.object->uz_distance_to_next_slice;
                    if (zoom_coordinate(tuz_object_x, zoom_factor) >= tuz_update_region_right_edge)
                    {
                        release_font_set(&z_font_set);
                        return;
                    }
                    draw_object_with_selection(&z_font_set, device_context, &staff_middle_pitch,
                        object_iter.object, project, staff, uz_space_height, zoom_factor,
                        tuz_staff_middle_y, tuz_object_x);
                    if (object_iter.object->address == node->address.object_address)
                    {
                        break;
                    }
                }
                break;
            }
            node_index = node->index_of_next;
        }
        if (tuz_update_region_right_edge < zoom_coordinate(slice_iter.uz_slice_x, zoom_factor))
        {
            release_font_set(&z_font_set);
            return;
        }
    }
    release_font_set(&z_font_set);
}

float get_tuz_y_of_staff_relative_step(int32_t tuz_staff_middle_y, float uz_staff_space_height,
    uint8_t staff_line_count, int8_t steps_above_bottom_line)
{
    return tuz_staff_middle_y +
        ((staff_line_count - 1 - steps_above_bottom_line) * uz_staff_space_height) / 2.0;
}

struct VerticalInterval get_tz_staff_vertical_bounds(float uz_staff_space_height, float zoom_factor,
    int32_t tuz_staff_middle_y, uint8_t staff_line_count)
{
    float line_thickness = uz_staff_space_height * BRAVURA_METADATA.uz_staff_line_thickness;
    return (struct VerticalInterval) { get_tz_horizontal_line_vertical_bounds(
        get_tuz_y_of_staff_relative_step(tuz_staff_middle_y, uz_staff_space_height,
            staff_line_count, 2 * (staff_line_count - 1)),
        line_thickness, zoom_factor).top,
        get_tz_horizontal_line_vertical_bounds(
            get_tuz_y_of_staff_relative_step(tuz_staff_middle_y, uz_staff_space_height,
                staff_line_count, 0),
            line_thickness, zoom_factor).bottom };
}

struct StaffObjectAddress get_ghost_cursor_address(struct Project*project, int32_t tz_mouse_x,
    int32_t tz_mouse_y)
{
    struct StaffObjectAddress out;
    if (!project->highest_visible_staff_index)
    {
        NULL_STAFF_OBJECT_ADDRESS(out);
        return out;
    }
    out.staff_index = project->highest_visible_staff_index;
    float zoom_factor = get_zoom_factor(project->zoom_exponent);
    int32_t tuz_staff_middle_y =
        project->utuz_y_of_staff_above_highest_visible - project->uz_viewport_offset.y;
    do
    {
        struct Staff*staff = resolve_pool_index(&project->staff_pool, out.staff_index);
        tuz_staff_middle_y += staff->uz_distance_from_staff_above;
        struct VerticalInterval tz_staff_vertical_bounds =
            get_tz_staff_vertical_bounds(project->uz_default_staff_space_height *
                ((struct StaffScale*)resolve_address(project, staff->scale_address))->value,
                zoom_factor, tuz_staff_middle_y, staff->line_count);
        if (tz_staff_vertical_bounds.top > tz_mouse_y)
        {
            NULL_STAFF_OBJECT_ADDRESS(out);
            return out;
        }
        int32_t tuz_mouse_x = unzoom_coordinate(tz_mouse_x, zoom_factor);
        if (tz_mouse_y <= tz_staff_vertical_bounds.bottom)
        {
            struct PositionedSliceIter slice_iter;
            initialize_slice_iter_to_t_leftmost_to_draw(&slice_iter, project);
            struct Object*cursor_position =
                get_leftmost_staff_object_to_draw(&slice_iter, project, out.staff_index);
            struct ObjectIter object_iter;
            initialize_page_element_iter(&object_iter.base, cursor_position, sizeof(struct Object));
            do
            {
                uint32_t address_node_index =
                    slice_iter.iter.slice->first_object_address_node_index;
                while (address_node_index)
                {
                    struct AddressNode*node =
                        resolve_pool_index(ADDRESS_NODE_POOL(project), address_node_index);
                    if (node->address.staff_index == out.staff_index)
                    {
                        struct Object*staff_object =
                            resolve_address(project, node->address.object_address);
                        while (true)
                        {
                            if (tz_mouse_x <
                                slice_iter.uz_slice_x - staff_object->uz_distance_to_next_slice)
                            {
                                goto left_end_reached;
                            }
                            if (object_iter.object->is_valid_cursor_position)
                            {
                                cursor_position = object_iter.object;
                            }
                            if (staff_object == object_iter.object)
                            {
                                break;
                            }
                            increment_page_element_iter(&object_iter.base, &project->page_pool,
                                sizeof(struct Object));
                        }
                        break;
                    }
                    address_node_index = node->index_of_next;
                }
                increment_slice_iter(&project->page_pool, &slice_iter);
            } while (slice_iter.iter.slice);
        left_end_reached:
            initialize_page_element_iter(&object_iter.base, cursor_position, sizeof(struct Object));
            while (!object_iter.object->is_valid_cursor_position)
            {
                increment_page_element_iter(&object_iter.base, &project->page_pool,
                    sizeof(struct Object));
            }
            out.object_address = object_iter.object->address;
            return out;
        }
        out.staff_index = staff->index_of_staff_below;
    } while (out.staff_index);
    NULL_STAFF_OBJECT_ADDRESS(out);
    return out;
}