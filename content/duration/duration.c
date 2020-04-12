#include "declarations.h"

void get_whole_notes_long(struct Duration*duration, struct Rational*out, struct Stack*out_stack)
{
    if (duration->log2 == 1)
    {
        if (duration->augmentation_dot_count)
        {
            struct Duration halved_duration;
            halved_duration.augmentation_dot_count = duration->augmentation_dot_count;
            halved_duration.log2 = 0;
            get_whole_notes_long(&halved_duration, out, out_stack);
            out->denominator->value[0] = out->denominator->value[0] >> 1;
        }
        else
        {
            out->numerator = initialize_stack_integer(out_stack, 2);
            out->denominator = initialize_stack_integer(out_stack, 1);
        }
    }
    else
    {
        uint32_t numerator = 1;
        uint32_t denominator = 1 << duration->augmentation_dot_count - duration->log2;
        if (duration->augmentation_dot_count)
        {
            uint32_t digit = 1;
            while (digit < denominator)
            {
                digit = digit << 1;
                numerator |= digit;
            }
        }
        out->numerator = initialize_stack_integer(out_stack, numerator);
        out->denominator = initialize_stack_integer(out_stack, denominator);
    }
}

struct Object*overwrite_range(struct ObjectIter*range_start, struct Project*project,
    struct Rational*whole_notes_after_current_slice, struct SliceIter*slice_iter,
    uint32_t range_end_address, uint32_t staff_index, bool is_merge)
{
    void*stack_a_savepoint = project->stack_a.cursor;
    while (true)
    {
        struct Slice*previous_slice = slice_iter->slice;
        increment_page_element_iter(&slice_iter->base, &project->page_pool, sizeof(struct Slice));
        if (!slice_iter->slice)
        {
            free_rational_from_persistent_memory(project, &previous_slice->whole_notes_long);
            copy_rational_to_persistent_memory(project, whole_notes_after_current_slice,
                &previous_slice->whole_notes_long);
            insert_slice_before_iter(slice_iter, project);
            slice_iter->slice->whole_notes_long.numerator =
                initialize_pool_integer(INTEGER_POOL(project), 0);
            slice_iter->slice->whole_notes_long.denominator =
                initialize_pool_integer(INTEGER_POOL(project), 1);
            break;
        }
        if (SLICE_IS_RHYTHMIC(previous_slice))
        {
            int8_t comparison = compare_rationals(whole_notes_after_current_slice,
                &previous_slice->whole_notes_long, &project->stack_a);
            if (comparison < 0)
            {
                insert_slice_before_iter(slice_iter, project);
                subtract_rationals(&slice_iter->slice->whole_notes_long,
                    &previous_slice->whole_notes_long, whole_notes_after_current_slice,
                    &project->stack_a, &project->stack_b);
                copy_rational_to_persistent_memory(project, &slice_iter->slice->whole_notes_long,
                    &slice_iter->slice->whole_notes_long);
                free_rational_from_persistent_memory(project, &previous_slice->whole_notes_long);
                copy_rational_to_persistent_memory(project, whole_notes_after_current_slice,
                    &previous_slice->whole_notes_long);
                slice_iter->slice->rod_intersection_count =
                    previous_slice->rod_intersection_count - 1;
                uint32_t node_index = previous_slice->first_object_address_node_index;
                while (node_index)
                {
                    slice_iter->slice->rod_intersection_count += 1;
                    node_index = ((struct AddressNode*)
                        resolve_pool_index(ADDRESS_NODE_POOL(project), node_index))->index_of_next;
                }
                break;
            }
            if (!comparison)
            {
                if (is_merge)
                {
                    slice_iter->slice->rod_intersection_count -= 1;
                }
                break;
            }
            subtract_rationals(whole_notes_after_current_slice, whole_notes_after_current_slice,
                &previous_slice->whole_notes_long, &project->stack_a, &project->stack_b);
        }
    }
    project->stack_a.cursor = stack_a_savepoint;
    while (true)
    {
        if (!range_start->object || range_start->object->address == range_end_address)
        {
            insert_slice_object_before_iter(range_start, project, slice_iter->slice->address,
                staff_index);
            break;
        }
        if (range_start->object->object_type == OBJECT_BARLINE)
        {
            increment_page_element_iter(&range_start->base, &project->page_pool,
                sizeof(struct Object));
        }
        else
        {
            if (range_start->object->slice_address != slice_iter->slice->address)
            {
                remove_object_from_slice(range_start, project);
                add_object_to_slice(range_start, project, slice_iter->slice->address, staff_index);
            }
            break;
        }
    }
    struct Object*out = range_start->object;
    increment_page_element_iter(&range_start->base, &project->page_pool, sizeof(struct Object));
    return out;
}

void overwrite_with_duration(struct Duration*duration, struct ObjectIter*iter,
    struct Project*project, uint32_t staff_index)
{
    void*stack_a_savepoint = project->stack_a.cursor;
    struct Rational previous_duration_whole_notes_long;
    get_whole_notes_long(duration, &previous_duration_whole_notes_long, &project->stack_a);
    struct Rational whole_notes_left_to_overwrite = previous_duration_whole_notes_long;
    struct SliceIter duration_slice_iter;
    while (true)
    {
        if (iter->object->slice_address)
        {
            struct Slice*slice = resolve_address(project, iter->object->slice_address);
            if (SLICE_IS_RHYTHMIC(slice))
            {
                slice->needs_respacing = true;
                initialize_page_element_iter(&duration_slice_iter.base, slice,
                    sizeof(struct Slice));
                break;
            }
        }
        if (iter->object->object_type == OBJECT_BARLINE)
        {
            increment_page_element_iter(&iter->base, &project->page_pool, sizeof(struct Object));
        }
        else
        {
            remove_object_at_iter(iter, project);
        }
    }
    iter->object->object_type = OBJECT_DURATION;
    iter->object->duration = *duration;
    struct ObjectIter range_to_remove_start = *iter;
    increment_page_element_iter(&range_to_remove_start.base, &project->page_pool,
        sizeof(struct Object));
    struct ObjectIter range_to_remove_end = range_to_remove_start;
    struct SliceIter duration_end_slice_iter = duration_slice_iter;
    while (true)
    {
        if (!range_to_remove_end.object)
        {
            struct Object*final_staff_object = overwrite_range(&range_to_remove_start, project,
                &whole_notes_left_to_overwrite, &duration_end_slice_iter, 0, staff_index, false);
            final_staff_object->object_type = OBJECT_NONE;
            final_staff_object->is_selected = false;
            final_staff_object->is_valid_cursor_position = true;
            while (range_to_remove_start.object)
            {
                if (range_to_remove_start.object->object_type == OBJECT_BARLINE)
                {
                    increment_page_element_iter(&range_to_remove_start.base, &project->page_pool,
                        sizeof(struct Object));
                }
                else
                {
                    remove_object_at_iter(&range_to_remove_start, project);
                }
            }
            project->stack_a.cursor = stack_a_savepoint;
            return;
        }
        if (range_to_remove_end.object->slice_address)
        {
            struct Slice*staff_slice =
                resolve_address(project, range_to_remove_end.object->slice_address);
            while (duration_end_slice_iter.slice != staff_slice)
            {
                if (SLICE_IS_RHYTHMIC(duration_end_slice_iter.slice))
                {
                    if (compare_rationals(&whole_notes_left_to_overwrite,
                        &duration_end_slice_iter.slice->whole_notes_long, &project->stack_a) > 0)
                    {
                        subtract_rationals(&whole_notes_left_to_overwrite,
                            &whole_notes_left_to_overwrite,
                            &duration_end_slice_iter.slice->whole_notes_long, &project->stack_a,
                            &project->stack_b);
                    }
                    else
                    {
                        subtract_rationals(&whole_notes_left_to_overwrite,
                            &duration_end_slice_iter.slice->whole_notes_long,
                            &whole_notes_left_to_overwrite, &project->stack_a, &project->stack_b);
                        while (true)
                        {
                            increment_page_element_iter(&duration_end_slice_iter.base,
                                &project->page_pool, sizeof(struct Slice));
                            if (duration_end_slice_iter.slice == staff_slice)
                            {
                                goto overwrite_end_found;
                            }
                            add_rationals(&whole_notes_left_to_overwrite,
                                &whole_notes_left_to_overwrite,
                                &duration_end_slice_iter.slice->whole_notes_long, &project->stack_a,
                                &project->stack_b);
                        }
                    }
                }
                increment_page_element_iter(&duration_end_slice_iter.base, &project->page_pool,
                    sizeof(struct Slice));
            }
            if (SLICE_IS_RHYTHMIC(staff_slice))
            {
                staff_slice->rod_intersection_count += 1;
            }
        }
        increment_page_element_iter(&range_to_remove_end.base, &project->page_pool,
            sizeof(struct Object));
    }
overwrite_end_found:
    uint32_t range_to_remove_end_address = range_to_remove_end.object->address;
    int8_t rest_duration_log2 = 0;
    struct Rational rest_whole_notes_long =
    { &(struct Integer) { 1, 1 }, &(struct Integer) { 1, 1 } };
    while (whole_notes_left_to_overwrite.denominator->value_count)
    {
        struct Division division;
        divide_integers(&division, whole_notes_left_to_overwrite.numerator,
            whole_notes_left_to_overwrite.denominator, &project->stack_a, &project->stack_b);
        halve_integer_in_place(whole_notes_left_to_overwrite.denominator);
        if (division.quotient->value_count)
        {
            whole_notes_left_to_overwrite.numerator = division.remainder;
            struct Object*rest = overwrite_range(&range_to_remove_start, project,
                &previous_duration_whole_notes_long, &duration_slice_iter,
                range_to_remove_end_address, staff_index, true);
            rest->duration.is_pitched = false;
            rest->duration.augmentation_dot_count = 0;
            rest->duration.log2 = rest_duration_log2;
            rest->object_type = OBJECT_DURATION;
            rest->is_selected = false;
            rest->is_valid_cursor_position = true;
            previous_duration_whole_notes_long = rest_whole_notes_long;
        }
        rest_duration_log2 -= 1;
        rest_whole_notes_long.denominator =
            double_integer(rest_whole_notes_long.denominator, &project->stack_a);
    }
    while (range_to_remove_start.object->address != range_to_remove_end_address)
    {
        if (range_to_remove_start.object->object_type == OBJECT_BARLINE)
        {
            increment_page_element_iter(&range_to_remove_start.base, &project->page_pool,
                sizeof(struct Object));
        }
        else
        {
            remove_object_at_iter(&range_to_remove_start, project);
        }
    }
    project->stack_a.cursor = stack_a_savepoint;
}

int8_t pitch_to_letter_name(int8_t pitch)
{
    int8_t out = pitch % 7;
    if (out < 0)
    {
        out = -out;
    }
    return out;
}

struct DisplayedAccidental get_default_accidental(struct Object*note, struct Project*project)
{
    struct DisplayedAccidental out = { NATURAL, false };
    void*stack_a_savepoint = project->stack_a.cursor;
    struct Pitch*pitch_in_other_octaves = start_array(&project->stack_a, _alignof(struct Pitch));
    uint_fast8_t pitch_in_other_octaves_count = 0;
    int8_t letter_name = pitch_to_letter_name(note->duration.pitch.pitch.steps_above_c4);
    struct ObjectIter iter;
    initialize_page_element_iter(&iter.base, note, sizeof(struct Object));
    while (true)
    {
        decrement_page_element_iter(&iter.base, &project->page_pool, sizeof(struct Object));
        if (!iter.object)
        {
            break;
        }
        switch (iter.object->object_type)
        {
        case OBJECT_DURATION:
            if (iter.object->duration.is_pitched)
            {
                if (iter.object->duration.pitch.pitch.steps_above_c4 ==
                    note->duration.pitch.pitch.steps_above_c4)
                {
                    out.accidental = iter.object->duration.pitch.pitch.accidental;
                    goto accidental_finalized;
                }
                else if (iter.object->duration.pitch.pitch.steps_above_c4 % 7 == letter_name)
                {
                    uint_fast8_t pitch_index = 0;
                    while (true)
                    {
                        if (pitch_index == pitch_in_other_octaves_count)
                        {
                            *(struct Pitch*)extend_array(&project->stack_a, sizeof(struct Pitch)) =
                                iter.object->duration.pitch.pitch;
                            ++pitch_in_other_octaves_count;
                            break;
                        }
                        if (iter.object->duration.pitch.pitch.steps_above_c4 ==
                            pitch_in_other_octaves[pitch_index].steps_above_c4)
                        {
                            break;
                        }
                        ++pitch_index;
                    }
                }
            }
            break;
        case OBJECT_KEY_SIG:
            for (uint_fast8_t i = 0; i < iter.object->key_sig.accidental_count; ++i)
            {
                if (iter.object->key_sig.accidentals[i].letter_name == letter_name)
                {
                    out.accidental = iter.object->key_sig.accidentals[i].accidental;
                    break;
                }
            }
            goto accidental_finalized;
        }
    }
accidental_finalized:
    for (uint_fast8_t i = 0; i < pitch_in_other_octaves_count; ++i)
    {
        if (pitch_in_other_octaves[i].accidental != out.accidental)
        {
            out.is_visible = true;
            break;
        }
    }
    project->stack_a.cursor = stack_a_savepoint;
    return out;
}