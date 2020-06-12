#include "declarations.h"

struct RodAddress
{
    uint32_t rod_index_in_log;
    uint32_t rod_log_index;
};

struct SliceRodLog
{
    struct Slice*slice;
    struct SliceRodLog*log_of_previous_joined_slice;
    int32_t*uz_rod_lengths;
};

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

int32_t uz_get_duration_width(struct Rational*duration, struct Project*project,
    float uz_staff_space_height)
{
    void*stack_a_savepoint = project->stack_a.cursor;
    struct Division division;
    divide_integers(&division, duration->numerator, duration->denominator, &project->stack_a,
        &project->stack_b);
    float float_whole_notes_long = integer_to_float(division.quotient);
    float place_value = 0.5;
    while (division.remainder->value_count)
    {
        divide_integers(&division, double_integer(division.remainder, &project->stack_a),
            duration->denominator, &project->stack_a, &project->stack_b);
        float_whole_notes_long += place_value * integer_to_float(division.quotient);
        place_value /= 2.0;
        if (place_value == 0.0)
        {
            project->stack_a.cursor = stack_a_savepoint;
            break;
        }
    }
    return float_round(uz_staff_space_height * 1.5 * log2f(24.25 * float_whole_notes_long + 1.0));
}

void add_rod_log(struct Project*project, struct SliceIter*slice_iter, uint32_t*log_count)
{
    slice_iter->slice->needs_respacing = false;
    struct SliceRodLog*rod_log = extend_array(&project->stack_a, sizeof(struct SliceRodLog));
    rod_log->slice = slice_iter->slice;
    rod_log->log_of_previous_joined_slice = 0;
    slice_iter->slice->index_in_slice_range = *log_count;
    size_t rod_array_size = *log_count * sizeof(uint32_t);
    rod_log->uz_rod_lengths = extend_array(&project->stack_b, rod_array_size);
    memset(rod_log->uz_rod_lengths, 0, rod_array_size);
    *log_count += 1;
}

struct SliceRodLog*get_canonical_join_group_log(struct SliceRodLog*rod_logs, uint32_t rod_log_index)
{
    struct SliceRodLog*log = rod_logs + rod_log_index;
    while (log->log_of_previous_joined_slice)
    {
        log = log->log_of_previous_joined_slice;
    }
    return log;
}

int32_t respace_slice_range(HDC device_context, struct PositionedSliceIter*slice_iter,
    struct Project*project)
{
    if (slice_iter->iter.slice->address == STAFF_START_SLICE_ADDRESS)
    {
        return 0;
    }
    do
    {
        decrement_slice_iter(&project->page_pool, slice_iter);
    } while (slice_iter->iter.slice->rod_intersection_count);
    struct SliceIter leftmost_slice_in_range_iter = slice_iter->iter;
    int32_t uz_leftmost_slice_in_range_distance_from_previous_slice =
        leftmost_slice_in_range_iter.slice->uz_distance_from_previous_slice;
    int32_t uz_delta = 0;
    uint32_t spring_count = 0;
    void*stack_a_savepoint = project->stack_a.cursor;
    void*stack_b_savepoint = project->stack_b.cursor;
    struct SliceRodLog*rod_logs = start_array(&project->stack_a, _alignof(struct SliceRodLog));
    start_array(&project->stack_b, _alignof(uint32_t));
    do
    {
        add_rod_log(project, &slice_iter->iter, &spring_count);
        increment_page_element_iter(&slice_iter->iter.base, &project->page_pool,
            sizeof(struct Slice));
        uz_delta -= slice_iter->iter.slice->uz_distance_from_previous_slice;
    } while (slice_iter->iter.slice->rod_intersection_count);
    add_rod_log(project, &slice_iter->iter, &spring_count);
    spring_count -= 1;
    size_t matrix_size = spring_count * sizeof(float*);
    start_array(&project->stack_a, _alignof(float*));
    float**confirmed_matrix = extend_array(&project->stack_a, matrix_size);
    float**tentative_matrix = extend_array(&project->stack_a, matrix_size);
    size_t matrix_row_size = spring_count * sizeof(float);
    start_array(&project->stack_a, _alignof(float));
    float*confirmed_augmentation = extend_array(&project->stack_a, matrix_row_size);
    float*tentative_augmentation = extend_array(&project->stack_a, matrix_row_size);
    float*spring_length_solutions = extend_array(&project->stack_a, matrix_row_size);
    float max_staff_scale_in_slice = 0.0;
    uint32_t node_index = rod_logs[0].slice->first_object_address_node_index;
    while (node_index)
    {
        struct AddressNode*node =
            resolve_pool_index(ADDRESS_NODE_POOL(project), node_index);
        float staff_scale = ((struct StaffScale*)resolve_address(project,
            ((struct Staff*)resolve_pool_index(&project->staff_pool,
                node->address.staff_index))->scale_address))->value;
        max_staff_scale_in_slice = MAX(max_staff_scale_in_slice, staff_scale);
        node_index = node->index_of_next;
    }
    int32_t uz_slice_duration_rod_length =
        uz_get_duration_width(&rod_logs[0].slice->whole_notes_long, project,
            project->uz_default_staff_space_height * max_staff_scale_in_slice);
    for (uint32_t spring_index = 0; spring_index < spring_count; ++spring_index)
    {
        confirmed_matrix[spring_index] = extend_array(&project->stack_a, matrix_row_size);
        memset(confirmed_matrix[spring_index], 0, matrix_row_size);
        tentative_matrix[spring_index] = extend_array(&project->stack_a, matrix_row_size);
        struct SliceRodLog*rod_log = rod_logs + spring_index + 1;
        float uz_max_space_height_in_slice = 0.0;
        node_index = rod_log->slice->first_object_address_node_index;
        do
        {
            struct AddressNode*node = resolve_pool_index(ADDRESS_NODE_POOL(project), node_index);
            float uz_space_height = project->uz_default_staff_space_height *
                ((struct StaffScale*)resolve_address(project,
                    ((struct Staff*)resolve_pool_index(&project->staff_pool,
                        node->address.staff_index))->scale_address))->value;
            uz_max_space_height_in_slice = MAX(uz_max_space_height_in_slice, uz_space_height);
            struct FontSet uz_font_set;
            get_staff_font_set(&uz_font_set, uz_space_height);
            struct Object*object_after_iter =
                resolve_address(project, node->address.object_address);
            struct ObjectIter object_iter;
            initialize_page_element_iter(&object_iter.base, object_after_iter,
                sizeof(struct Object));
            int32_t uz_rod_length =
                uz_get_default_distance_from_object_origin_to_slice(uz_space_height,
                    object_iter.object);
            object_iter.object->uz_distance_to_next_slice = uz_rod_length;
            while (true)
            {
                decrement_page_element_iter(&object_iter.base, &project->page_pool,
                    sizeof(struct Object));
                switch (object_iter.object->object_type)
                {
                case OBJECT_ACCIDENTAL:
                    uz_rod_length += get_character_width(device_context, uz_font_set.full_size,
                        get_accidental_codepoint(((struct Object*)resolve_address(project,
                            object_iter.object->accidental_note_address))->
                            duration.pitch.pitch.accidental)) +
                        float_round(uz_space_height * UZ_DISTANCE_BETWEEN_ACCIDENTAL_AND_NOTE);
                    break;
                case OBJECT_BARLINE:
                    uz_rod_length += float_round(project->uz_default_staff_space_height *
                        (BRAVURA_METADATA.uz_thin_barline_thickness + 1.0));
                    break;
                case OBJECT_CLEF:
                {
                    HFONT uz_font;
                    if (object_is_header(object_iter.object))
                    {
                        uz_font = uz_font_set.full_size;
                    }
                    else
                    {
                        uz_font = uz_font_set.two_thirds_size;
                    }
                    uz_rod_length += float_round(uz_space_height) +
                        get_character_width(device_context, uz_font,
                            object_iter.object->clef.codepoint);
                    break;
                }
                case OBJECT_DURATION:
                {
                    float spacer;
                    if (object_after_iter->object_type == OBJECT_DURATION)
                    {
                        spacer = 0.0;
                    }
                    else
                    {
                        spacer = 1.0;
                    }
                    uz_rod_length += float_round(uz_space_height *
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
                    if (object_iter.object->key_sig.accidental_count)
                    {
                        float spacer;
                        switch (object_after_iter->object_type)
                        {
                        case OBJECT_ACCIDENTAL:
                        {
                            struct ObjectIter accidental_iter;
                            initialize_page_element_iter(&accidental_iter.base, object_after_iter,
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
                        uz_rod_length += float_round(uz_space_height * spacer);
                        for (uint8_t i = 0; i < object_iter.object->key_sig.accidental_count; ++i)
                        {
                            uz_rod_length += get_character_width(device_context,
                                uz_font_set.full_size, get_accidental_codepoint(
                                    object_iter.object->key_sig.accidentals[i].accidental));
                        }
                    }
                    break;
                }
                case OBJECT_NONE:
                {
                    uz_rod_length += float_round(uz_space_height);
                    break;
                }
                case OBJECT_TIME_SIG:
                {
                    float spacer;
                    switch (object_after_iter->object_type)
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
                    uz_rod_length += float_round(uz_space_height * spacer) +
                        (MAX(get_string_width(device_context, uz_font_set.full_size,
                            strings.numerator_string, strings.numerator_string_length),
                            get_string_width(device_context, uz_font_set.full_size,
                                strings.denominator_string, strings.denominator_string_length)));
                }
                };
                if (object_iter.object->slice_address)
                {
                    void*local_stack_b_savepoint = project->stack_b.cursor;
                    uz_rod_length -=
                        uz_get_default_distance_from_object_origin_to_slice(uz_space_height,
                            object_iter.object);
                    struct Rational whole_notes_long_of_slice_duration =
                    { &(struct Integer) { 0 }, &(struct Integer) { 1, 1 } };
                    struct SliceIter staff_slice_iter;
                    initialize_page_element_iter(&staff_slice_iter.base, rod_log->slice,
                        sizeof(struct Slice));
                    do
                    {
                        decrement_page_element_iter(&staff_slice_iter.base, &project->page_pool,
                            sizeof(struct Slice));
                        add_rationals(&whole_notes_long_of_slice_duration,
                            &whole_notes_long_of_slice_duration,
                            &staff_slice_iter.slice->whole_notes_long, &project->stack_b,
                            &project->stack_a);
                    } while (staff_slice_iter.slice->address != object_iter.object->slice_address);
                    int32_t*rod_length_in_log =
                        rod_log->uz_rod_lengths + staff_slice_iter.slice->index_in_slice_range;
                    *rod_length_in_log = MAX(*rod_length_in_log, uz_rod_length);
                    uz_rod_length = uz_get_duration_width(&whole_notes_long_of_slice_duration,
                        project, uz_space_height);
                    *rod_length_in_log = MAX(*rod_length_in_log, uz_rod_length);
                    project->stack_b.cursor = local_stack_b_savepoint;
                    goto logging_for_staff_finished;
                }
                object_after_iter = object_iter.object;
                object_after_iter->uz_distance_to_next_slice = uz_rod_length;
            }
        logging_for_staff_finished:
            release_font_set(&uz_font_set);
            node_index = node->index_of_next;
        } while (node_index);
        int32_t*uz_rod_length_in_log = rod_log->uz_rod_lengths + spring_index;
        if (uz_slice_duration_rod_length)
        {
            *uz_rod_length_in_log = MAX(*uz_rod_length_in_log, uz_slice_duration_rod_length);
            confirmed_matrix[spring_index][spring_index] =
                1.0 / (float)uz_slice_duration_rod_length;
            confirmed_augmentation[spring_index] = 0.0;
        }
        else
        {
            confirmed_matrix[spring_index][spring_index] = 1.0;
            confirmed_augmentation[spring_index] = *uz_rod_length_in_log;
            *uz_rod_length_in_log = 0;
            rod_log->log_of_previous_joined_slice = rod_log - 1;
        }
        uz_slice_duration_rod_length = uz_get_duration_width(&rod_log->slice->whole_notes_long,
            project, uz_max_space_height_in_slice);
    }
    uint32_t first_spring_index = 0;
    do
    {
        if (rod_logs[first_spring_index + 1].slice->whole_notes_long.numerator->value_count)
        {
            uint32_t spring_index = first_spring_index;
            uint32_t next_spring_index = first_spring_index;
            while (true)
            {
                do
                {
                    next_spring_index += 1;
                    if (next_spring_index == spring_count)
                    {
                        confirmed_matrix[spring_index][first_spring_index] =
                            -confirmed_matrix[first_spring_index][first_spring_index];
                        goto matrix_finalized;
                    }
                } while (!rod_logs[first_spring_index + 1].
                    slice->whole_notes_long.numerator->value_count);
                confirmed_matrix[spring_index][next_spring_index] =
                    -confirmed_matrix[next_spring_index][next_spring_index];
                spring_index = next_spring_index;
            }
        }
        first_spring_index += 1;
    } while (first_spring_index < spring_count);
matrix_finalized:
    struct RodAddress*unactivated_rods =
        start_array(&project->stack_a, _alignof(struct RodAddress));
    uint32_t unactivated_rod_count = 0;
    for (uint32_t log_index = 1; log_index <= spring_count; ++log_index)
    {
        struct SliceRodLog*log = rod_logs + log_index;
        for (uint32_t rod_index = 0; rod_index < log_index; ++rod_index)
        {
            if (log->uz_rod_lengths[rod_index])
            {
                struct RodAddress*address =
                    extend_array(&project->stack_a, sizeof(struct RodAddress));
                address->rod_index_in_log = rod_index;
                address->rod_log_index = log_index;
                unactivated_rod_count += 1;
            }
        }
    }
    if (unactivated_rod_count)
    {
        do
        {
            float uz_max_slice_range_width_at_which_rod_actives = 0.0;
            uint32_t address_index_of_rod_to_activate;
            uint32_t address_rod_index = 0;
            while (address_rod_index < unactivated_rod_count)
            {
                struct RodAddress address = unactivated_rods[address_rod_index];
                memcpy(tentative_augmentation, confirmed_augmentation, matrix_row_size);
                tentative_augmentation[0] =
                    rod_logs[address.rod_log_index].uz_rod_lengths[address.rod_index_in_log];
                memset(tentative_matrix[0], 0, matrix_row_size);
                for (uint32_t i = address.rod_index_in_log; i < address.rod_log_index; ++i)
                {
                    tentative_matrix[0][i] = 1.0;
                }
                for (uint32_t i = 1; i < spring_count; ++i)
                {
                    memcpy(tentative_matrix[i], confirmed_matrix[i], matrix_row_size);
                }
                for (uint32_t i = 0; i < spring_count; ++i)
                {
                    uint32_t j = i;
                    while (!tentative_matrix[i][i])
                    {
                        j += 1;
                        if (j == spring_count)
                        {
                            goto rod_cannot_be_activated;
                        }
                        SWAP(tentative_matrix[i], tentative_matrix[j], float*);
                        SWAP(tentative_augmentation[i], tentative_augmentation[j], float);
                    }
                    for (uint32_t k = i + 1; k < spring_count; ++k)
                    {
                        float scalar = tentative_matrix[k][i] / tentative_matrix[i][i];
                        for (uint32_t l = i; l < spring_count; ++l)
                        {
                            tentative_matrix[k][l] =
                                tentative_matrix[k][l] - tentative_matrix[i][l] * scalar;
                        }
                        tentative_augmentation[k] =
                            tentative_augmentation[k] - tentative_augmentation[i] * scalar;
                    }
                }
                float uz_slice_range_width_at_which_rod_actives = 0.0;
                for (uint32_t i = spring_count; i-- > 0;)
                {
                    tentative_augmentation[i] = tentative_augmentation[i] / tentative_matrix[i][i];
                    for (uint32_t j = 0; j <= i; ++j)
                    {
                        tentative_matrix[i][j] = tentative_matrix[i][j] / tentative_matrix[i][i];
                    }
                    for (uint32_t j = 0; j < i; ++j)
                    {
                        tentative_augmentation[j] = tentative_augmentation[j] -
                            tentative_augmentation[i] * tentative_matrix[j][i];
                    }
                    uz_slice_range_width_at_which_rod_actives += tentative_augmentation[i];
                }
                if (uz_slice_range_width_at_which_rod_actives >
                    uz_max_slice_range_width_at_which_rod_actives)
                {
                    address_index_of_rod_to_activate = address_rod_index;
                    uz_max_slice_range_width_at_which_rod_actives =
                        uz_slice_range_width_at_which_rod_actives;
                    SWAP(tentative_augmentation, spring_length_solutions, float*);
                }
            rod_cannot_be_activated:
                address_rod_index += 1;
            }
            if (uz_max_slice_range_width_at_which_rod_actives)
            {
                struct RodAddress address_of_rod_to_activate =
                    unactivated_rods[address_index_of_rod_to_activate];
                struct SliceRodLog*leftmost_join_group_log = get_canonical_join_group_log(rod_logs,
                    address_of_rod_to_activate.rod_index_in_log);
                struct SliceRodLog*rightmost_join_group_log = get_canonical_join_group_log(rod_logs,
                    address_of_rod_to_activate.rod_log_index);
                if (leftmost_join_group_log > rightmost_join_group_log)
                {
                    SWAP(leftmost_join_group_log, rightmost_join_group_log, struct SliceRodLog*);
                }
                rightmost_join_group_log->log_of_previous_joined_slice = leftmost_join_group_log;
                uint32_t row_to_add_to_index;
                if (leftmost_join_group_log == rod_logs)
                {
                    row_to_add_to_index = spring_count - 1;
                }
                else
                {
                    row_to_add_to_index = leftmost_join_group_log->slice->index_in_slice_range - 1;
                }
                uint32_t row_to_add_index =
                    rightmost_join_group_log->slice->index_in_slice_range - 1;
                if (row_to_add_index == row_to_add_to_index)
                {
                    break;
                }
                float*row_to_add_to = confirmed_matrix[row_to_add_to_index];
                float*row_to_add = confirmed_matrix[row_to_add_index];
                for (uint32_t i = 0; i < spring_count; ++i)
                {
                    row_to_add_to[i] += row_to_add[i];
                }
                memset(row_to_add, 0, matrix_row_size);
                for (uint32_t i = address_of_rod_to_activate.rod_index_in_log;
                    i < address_of_rod_to_activate.rod_log_index; ++i)
                {
                    row_to_add[i] = 1.0;
                }
                confirmed_augmentation[row_to_add_index] =
                    rod_logs[address_of_rod_to_activate.rod_log_index].
                    uz_rod_lengths[address_of_rod_to_activate.rod_index_in_log];
                unactivated_rod_count -= 1;
                unactivated_rods[address_index_of_rod_to_activate] =
                    unactivated_rods[unactivated_rod_count];
            }
            else
            {
                break;
            }
        } while (unactivated_rod_count);
    }
    else
    {
        spring_length_solutions = confirmed_augmentation;
    }
    for (uint32_t i = 0; i < spring_count; ++i)
    {
        int32_t uz_distance_from_previous_slice = float_round(spring_length_solutions[i]);
        rod_logs[i + 1].slice->uz_distance_from_previous_slice = uz_distance_from_previous_slice;
        slice_iter->uz_slice_x += uz_distance_from_previous_slice;
        uz_delta += uz_distance_from_previous_slice;
    }
    project->utuz_last_slice_x += uz_delta;
    leftmost_slice_in_range_iter.slice->uz_distance_from_previous_slice =
        uz_leftmost_slice_in_range_distance_from_previous_slice;
    project->stack_a.cursor = stack_a_savepoint;
    project->stack_b.cursor = stack_b_savepoint;
    return uz_delta;
}

void respace_onscreen_slices(HWND main_window_handle, struct Project*project)
{
    RECT tz_work_region_rect;
    get_work_region_rect(main_window_handle, project, &tz_work_region_rect);
    int32_t tuz_work_region_right_edge =
        unzoom_coordinate(tz_work_region_rect.right, get_zoom_factor(project->zoom_exponent));
    HDC device_context = GetDC(main_window_handle);
    struct PositionedSliceIter iter;
    initialize_slice_iter_to_t_leftmost_to_draw(&iter, project);
    while (iter.iter.slice && iter.uz_slice_x < tuz_work_region_right_edge)
    {
        if (iter.iter.slice->needs_respacing)
        {
            respace_slice_range(device_context, &iter, project);
        }
        increment_slice_iter(&project->page_pool, &iter);
    }
    ReleaseDC(main_window_handle, device_context);
}