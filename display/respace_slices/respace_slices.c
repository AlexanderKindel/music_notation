#include "declarations.h"

int32_t uz_get_default_distance_from_object_origin_to_slice(float uz_staff_space_height,
    struct Object*object)
{
    if (object->object_type == OBJECT_DURATION && object->duration.is_pitched &&
        object->duration.log2 == 1)
    {
        return float_round(uz_staff_space_height *
            BRAVURA_METADATA.uz_double_whole_notehead_x_offset);
    }
    return 0;
}

int32_t reset_distance_from_previous_slice(HDC device_context, struct Project*project,
    struct Slice*slice)
{
    slice->needs_respacing = false;
    struct SliceIter slice_iter;
    initialize_page_element_iter(&slice_iter.base, slice, sizeof(struct Slice));
    decrement_page_element_iter(&slice_iter.base, &project->page_pool, sizeof(struct Slice));
    if (!slice_iter.slice)
    {
        return 0;
    }
    int32_t uz_distance_from_previous_slice = 0;
    if (slice->whole_notes_long.denominator)
    {
        struct SliceIter previous_duration_slice_iter = slice_iter;
        while (previous_duration_slice_iter.slice)
        {
            if (previous_duration_slice_iter.slice->whole_notes_long.denominator)
            {
                void*stack_a_savepoint = project->stack_a.cursor;
                struct Rational min_whole_notes_long_of_slice_duration =
                { &(struct Integer) { 1, 2 }, &(struct Integer) { 1, 1 } };
                float uz_max_staff_scale_at_min_duration = 0.0;
                uint32_t index_of_next_node =
                    previous_duration_slice_iter.slice->first_object_address_node_index;
                while (index_of_next_node)
                {
                    struct AddressNode*node =
                        resolve_pool_index(&ADDRESS_NODE_POOL(project), index_of_next_node);
                    struct Rational whole_notes_long_of_slice_duration;
                    get_whole_notes_long(&((struct Object*)resolve_address(project,
                        node->address.object_address))->duration,
                        &whole_notes_long_of_slice_duration, &project->stack_a);
                    if (compare_rationals(&whole_notes_long_of_slice_duration,
                        &min_whole_notes_long_of_slice_duration, &project->stack_a) <= 0)
                    {
                        min_whole_notes_long_of_slice_duration = whole_notes_long_of_slice_duration;
                        float uz_scale = ((struct StaffScale*)resolve_address(project,
                            ((struct Staff*)resolve_pool_index(&project->staff_pool,
                                node->address.staff_index))->scale_address))->value;
                        uz_max_staff_scale_at_min_duration =
                            MAX(uz_max_staff_scale_at_min_duration, uz_scale);
                    }
                    index_of_next_node = node->index_of_next;
                }
                project->stack_a.cursor = stack_a_savepoint;
                struct Division division;
                divide_integers(&division,
                    previous_duration_slice_iter.slice->whole_notes_long.numerator,
                    previous_duration_slice_iter.slice->whole_notes_long.denominator,
                    &project->stack_a, &project->stack_b);
                float whole_notes_long_float = integer_to_float(division.quotient);
                float place_value = 0.5;
                while (division.remainder->value_count)
                {
                    divide_integers(&division,
                        double_integer(division.remainder, &project->stack_a),
                        previous_duration_slice_iter.slice->whole_notes_long.denominator,
                        &project->stack_a, &project->stack_b);
                    whole_notes_long_float += place_value * integer_to_float(division.quotient);
                    place_value /= 2.0;
                    if (place_value == 0.0)
                    {
                        break;
                    }
                }
                uz_distance_from_previous_slice = float_round(UZ_WHOLE_NOTE_WIDTH *
                    uz_max_staff_scale_at_min_duration * project->uz_default_staff_space_height *
                    powf(DURATION_RATIO, log2f(whole_notes_long_float)));
                project->stack_a.cursor = stack_a_savepoint;
                break;
            }
            decrement_page_element_iter(&previous_duration_slice_iter.base, &project->page_pool,
                sizeof(struct Slice));
        }
    }
    uint32_t index_of_next_node = slice->first_object_address_node_index;
    while (index_of_next_node)
    {
        struct AddressNode*node =
            resolve_pool_index(&ADDRESS_NODE_POOL(project), index_of_next_node);
        struct Staff*staff = resolve_pool_index(&project->staff_pool, node->address.staff_index);
        float uz_space_height = project->uz_default_staff_space_height *
            ((struct StaffScale*)resolve_address(project, staff->scale_address))->value;
        struct FontSet uz_font_set;
        get_staff_font_set(&uz_font_set, uz_space_height);
        struct Object*object = resolve_address(project, node->address.object_address);
        int32_t uz_range_width =
            uz_get_default_distance_from_object_origin_to_slice(uz_space_height, object);
        object->uz_distance_to_next_slice = uz_range_width;
        struct ObjectIter object_iter;
        initialize_page_element_iter(&object_iter.base, object, sizeof(struct Object));
        while (true)
        {
            decrement_page_element_iter(&object_iter.base, &project->page_pool,
                sizeof(struct Object));
            if (object_iter.object->slice_address == STAFF_START_SLICE_ADDRESS)
            {
                uz_range_width += float_round(uz_space_height);
                break;
            }
            switch (object_iter.object->object_type)
            {
            case OBJECT_ACCIDENTAL:
                uz_range_width += get_character_width(device_context, uz_font_set.full_size,
                    get_accidental_codepoint(((struct Object*)resolve_address(project,
                        object_iter.object->accidental_note_address))->
                        duration.pitch.pitch.accidental)) +
                    float_round(uz_space_height * UZ_DISTANCE_BETWEEN_ACCIDENTAL_AND_NOTE);
                break;
            case OBJECT_BARLINE:
                uz_range_width += float_round(project->uz_default_staff_space_height *
                    (BRAVURA_METADATA.uz_thin_barline_thickness + 1.0));
                break;
            case OBJECT_CLEF:
            {
                float spacer = 1.0;
                HFONT uz_font;
                if (object_is_header(object_iter.object))
                {
                    uz_font = uz_font_set.full_size;
                    switch (object->object_type)
                    {
                    case OBJECT_ACCIDENTAL:
                    {
                        struct ObjectIter accidental_iter;
                        initialize_page_element_iter(&accidental_iter.base, object,
                            sizeof(struct Object));
                        increment_page_element_iter(&accidental_iter.base, &project->page_pool,
                            sizeof(struct Object));
                        if (accidental_iter.object->object_type != OBJECT_ACCIDENTAL)
                        {
                            spacer = 1.5;
                        }
                        break;
                    }
                    case OBJECT_DURATION:
                        spacer = 2.5;
                    }
                }
                else
                {
                    uz_font = uz_font_set.two_thirds_size;
                }
                uz_range_width += float_round(uz_space_height * spacer) +
                    get_character_width(device_context, uz_font,
                        object_iter.object->clef.codepoint);
                break;
            }
            case OBJECT_DURATION:
            {
                float spacer;
                if (object->object_type == OBJECT_DURATION)
                {
                    spacer = 0.0;
                }
                else
                {
                    spacer = 1.0;
                }
                uz_range_width += float_round(uz_space_height *
                    (object_iter.object->duration.augmentation_dot_count *
                        UZ_DISTANCE_BETWEEN_AUGMENTATION_DOTS + spacer)) +
                    object_iter.object->duration.augmentation_dot_count *
                    get_character_width(device_context, uz_font_set.full_size, 0xe1e7) +
                    get_character_width(device_context, uz_font_set.full_size,
                        get_duration_codepoint(&object_iter.object->duration));
                break;
            }
            case OBJECT_KEY_SIG:
            {
                float spacer;
                switch (object->object_type)
                {
                case OBJECT_ACCIDENTAL:
                {
                    struct ObjectIter accidental_iter;
                    initialize_page_element_iter(&accidental_iter.base, object,
                        sizeof(struct Object));
                    increment_page_element_iter(&accidental_iter.base, &project->page_pool,
                        sizeof(struct Object));
                    if (accidental_iter.object->object_type != OBJECT_ACCIDENTAL)
                    {
                        spacer = 1.0;
                    }
                    else
                    {
                        spacer = 1.5;
                    }
                    break;
                }
                case OBJECT_CLEF:
                    spacer = 2.0;
                    break;
                case OBJECT_DURATION:
                    if (object_is_header(object_iter.object))
                    {
                        spacer = 2.5;
                    }
                    else
                    {
                        spacer = 2.0;
                    }
                    break;
                case OBJECT_KEY_SIG:
                    spacer = 2.0;
                    break;
                default:
                    spacer = 1.0;
                }
                uz_range_width += float_round(uz_space_height * spacer);
                for (uint8_t i = 0; i < object_iter.object->key_sig.accidental_count; ++i)
                {
                    uz_range_width += get_character_width(device_context, uz_font_set.full_size,
                        get_accidental_codepoint(object_iter.object->
                            key_sig.accidentals[i].accidental));
                }
                break;
            }
            case OBJECT_TIME_SIG:
            {
                float spacer;
                switch (object->object_type)
                {
                case OBJECT_ACCIDENTAL:
                case OBJECT_BARLINE:
                case OBJECT_NONE:
                    spacer = 1.0;
                    break;
                default:
                    spacer = 2.0;
                }
                struct TimeSigStrings strings;
                time_sig_to_strings(&strings, object_iter.object->time_sig);
                uz_range_width += float_round(uz_space_height * spacer) +
                    (MAX(get_string_width(device_context, uz_font_set.full_size,
                        strings.numerator_string, strings.numerator_string_length),
                        get_string_width(device_context, uz_font_set.full_size,
                            strings.denominator_string, strings.denominator_string_length)));
            }
            };
            object = object_iter.object;
            if (object->slice_address)
            {
                uz_range_width -=
                    uz_get_default_distance_from_object_origin_to_slice(uz_space_height, object);
                initialize_page_element_iter(&slice_iter.base,
                    resolve_address(project, object->slice_address), sizeof(struct Slice));
                while (true)
                {
                    increment_page_element_iter(&slice_iter.base, &project->page_pool,
                        sizeof(struct Slice));
                    if (slice_iter.slice == slice)
                    {
                        break;
                    }
                    uz_range_width -= slice_iter.slice->uz_distance_from_previous_slice;
                }
                break;
            }
            object->uz_distance_to_next_slice = uz_range_width;
        }
        uz_distance_from_previous_slice = MAX(uz_distance_from_previous_slice, uz_range_width);
        release_font_set(&uz_font_set);
        index_of_next_node = node->index_of_next;
    }
    int32_t delta = uz_distance_from_previous_slice - slice->uz_distance_from_previous_slice;
    project->utuz_last_slice_x += delta;
    slice->uz_distance_from_previous_slice = uz_distance_from_previous_slice;
    return delta;
}

int32_t respace_slice_range_left_of_iter(HDC device_context, struct Project*project,
    struct SliceIter*iter)
{
    int32_t out = 0;
    if (iter->slice->needs_respacing)
    {
        struct SliceIter iter_copy = *iter;
        decrement_page_element_iter(&iter_copy.base, &project->page_pool, sizeof(struct Slice));
        if (iter_copy.slice)
        {
            out += respace_slice_range_left_of_iter(device_context, project, &iter_copy);
        }
        out += reset_distance_from_previous_slice(device_context, project, iter->slice);
    }
    return out;
}

void respace_onscreen_slices(HWND main_window_handle, struct Project*project)
{
    struct PositionedSliceIter iter;
    initialize_slice_iter_to_t_leftmost_to_draw(&iter, project);
    while (iter.iter.slice->needs_respacing &&
        iter.iter.slice->address != STAFF_START_SLICE_ADDRESS)
    {
        decrement_slice_iter(&project->page_pool, &iter);
    }
    RECT tz_work_region_rect;
    get_work_region_rect(main_window_handle, project, &tz_work_region_rect);
    int32_t tuz_work_region_right_edge =
        unzoom_coordinate(tz_work_region_rect.right, get_zoom_factor(project->zoom_exponent));
    HDC device_context = GetDC(main_window_handle);
    while (iter.iter.slice && iter.uz_slice_x < tuz_work_region_right_edge)
    {
        if (iter.iter.slice->needs_respacing)
        {
            reset_distance_from_previous_slice(device_context, project, iter.iter.slice);
        }
        increment_slice_iter(&project->page_pool, &iter);
    }
    ReleaseDC(main_window_handle, device_context);
}