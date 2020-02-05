#include "declarations.h"
#include "memory.c"
#include "rational.c"

__declspec(noreturn) void crash(char*message)
{
    puts(message);
    abort();
}

int8_t clamped_add(int8_t augend, uint8_t addend)
{
    if (augend > INT8_MAX - addend)
    {
        return INT8_MAX;
    }
    return augend + addend;
}

int8_t clamped_subtract(int8_t minuend, uint8_t subtrahend)
{
    if (minuend < INT8_MIN + subtrahend)
    {
        return INT8_MIN;
    }
    return minuend - subtrahend;
}

void insert_sliceless_object_before_iter(struct ObjectIter*iter, struct Project*project)
{
    insert_page_element_before_iter(&iter->base, project, sizeof(struct Object));
    iter->object->slice_address = 0;
    struct ObjectIter iter_copy = *iter;
    get_next_slice_right_of_iter(&iter_copy, project)->needs_respacing = true;
}

void add_object_to_slice(struct ObjectIter*iter, struct Project*project, uint32_t slice_address,
    uint32_t staff_index)
{
    iter->object->slice_address = slice_address;
    struct AddressNode*node = allocate_pool_slot(&ADDRESS_NODE_POOL(project));
    node->address.object_address = iter->object->address;
    node->address.staff_index = staff_index;
    struct Slice*slice = resolve_address(project, slice_address);
    node->index_of_next = slice->first_object_address_node_index;
    slice->first_object_address_node_index =
        get_element_index_in_pool(&ADDRESS_NODE_POOL(project), node);
    slice->needs_respacing = true;
    struct SliceIter slice_iter;
    initialize_page_element_iter(&slice_iter.base, slice, sizeof(struct Slice));
    increment_page_element_iter(&slice_iter.base, &project->page_pool, sizeof(struct Slice));
    if (slice_iter.slice)
    {
        slice_iter.slice->needs_respacing = true;
    }
}

void insert_slice_object_before_iter(struct ObjectIter*iter, struct Project*project,
    uint32_t slice_address, uint32_t staff_index)
{
    insert_page_element_before_iter(&iter->base, project, sizeof(struct Object));
    add_object_to_slice(iter, project, slice_address, staff_index);
}

void remove_object_from_slice(struct ObjectIter*iter, struct Project*project)
{
    if (iter->object->slice_address)
    {
        struct SliceIter slice_iter;
        initialize_page_element_iter(&slice_iter.base,
            resolve_address(project, iter->object->slice_address), sizeof(struct Slice));
        uint32_t*index_of_node = &slice_iter.slice->first_object_address_node_index;
        while (true)
        {
            struct AddressNode*node =
                resolve_pool_index(&ADDRESS_NODE_POOL(project), *index_of_node);
            if (node->address.object_address == iter->object->address)
            {
                *index_of_node = node->index_of_next;
                free_pool_slot(&ADDRESS_NODE_POOL(project), node);
                break;
            }
            *index_of_node = node->index_of_next;
        }
        if (slice_iter.slice->first_object_address_node_index)
        {
            slice_iter.slice->needs_respacing = true;
            increment_page_element_iter(&slice_iter.base, &project->page_pool,
                sizeof(struct Slice));
        }
        else
        {
            struct SliceIter previous_duration_slice_iter = slice_iter;
            while (true)
            {
                decrement_page_element_iter(&previous_duration_slice_iter.base, &project->page_pool,
                    sizeof(struct Object));
                if (!previous_duration_slice_iter.slice)
                {
                    break;
                }
                if (previous_duration_slice_iter.slice->whole_notes_long.denominator)
                {
                    void*stack_a_savepoint = project->stack_a.cursor;
                    struct Rational old_whole_notes_long =
                        previous_duration_slice_iter.slice->whole_notes_long;
                    add_rationals(&previous_duration_slice_iter.slice->whole_notes_long,
                        &old_whole_notes_long, &slice_iter.slice->whole_notes_long,
                        &project->stack_a, &project->stack_b);
                    free_rational_from_persistent_memory(project, &old_whole_notes_long);
                    copy_rational_to_persistent_memory(project,
                        &previous_duration_slice_iter.slice->whole_notes_long,
                        &previous_duration_slice_iter.slice->whole_notes_long);
                    project->stack_a.cursor = stack_a_savepoint;
                    break;
                }
            }
            free_rational_from_persistent_memory(project, &slice_iter.slice->whole_notes_long);
            remove_page_element_at_iter(&slice_iter.base, project, sizeof(struct Slice));
        }
        if (slice_iter.slice)
        {
            slice_iter.slice->needs_respacing = true;
        }
    }
    else
    {
        struct ObjectIter iter_copy = *iter;
        get_next_slice_right_of_iter(&iter_copy, project)->needs_respacing = true;
    }
}

void remove_object_at_iter(struct ObjectIter*iter, struct Project*project)
{
    if (project->ghost_cursor_address.object_address == iter->object->address)
    {
        project->ghost_cursor_address.staff_index = 0;
    }
    remove_object_from_slice(iter, project);
    remove_page_element_at_iter(&iter->base, project, sizeof(struct Object));
}

void remove_object_tree_at_iter(struct ObjectIter*iter, struct Project*project)
{
    switch (iter->object->object_type)
    {
    case OBJECT_ACCIDENTAL:
    case OBJECT_BARLINE:
        return;
    case OBJECT_CLEF:
    {
        struct Staff*staff =
            resolve_pool_index(&project->staff_pool, project->selection.address.staff_index);
        if (staff->address_of_clef_beyond_leftmost_slice_to_draw == iter->object->address)
        {
            struct ObjectIter previous_clef_iter = *iter;
            while (true)
            {
                decrement_page_element_iter(&previous_clef_iter.base, &project->page_pool,
                    sizeof(struct Object));
                if (previous_clef_iter.object->object_type == OBJECT_CLEF)
                {
                    staff->address_of_clef_beyond_leftmost_slice_to_draw =
                        previous_clef_iter.object->address;
                    break;
                }
            }
        }
        remove_object_at_iter(iter, project);
        break;
    }
    case OBJECT_DURATION:
    {
        if (iter->object->duration.is_pitched)
        {
            uint32_t note_address = iter->object->address;
            if (iter->object->duration.pitch.accidental_object_address)
            {
                iter->object->is_valid_cursor_position = true;
                struct ObjectIter accidental_iter;
                initialize_page_element_iter(&accidental_iter.base,
                    resolve_address(project,
                        iter->object->duration.pitch.accidental_object_address),
                    sizeof(struct Object));
                remove_object_at_iter(&accidental_iter, project);
                initialize_page_element_iter(&iter->base, resolve_address(project, note_address),
                    sizeof(struct Object));
            }
            reset_accidental_displays_from_previous_key_sig(iter->object, project);
            initialize_page_element_iter(&iter->base, resolve_address(project, note_address),
                sizeof(struct Object));
        }
        remove_object_at_iter(iter, project);
        break;
    }
    case OBJECT_KEY_SIG:
    {
        remove_object_at_iter(iter, project);
        reset_accidental_displays_from_previous_key_sig(iter->object, project);
        break;
    }
    case OBJECT_TIME_SIG:
        remove_object_at_iter(iter, project);
    }
}

void delete_object(struct Object*object, struct Project*project)
{
    object->is_selected = false;
    project->selection.selection_type = SELECTION_CURSOR;
    set_cursor_to_next_valid_state(project);
    switch (object->object_type)
    {
    case OBJECT_ACCIDENTAL:
    case OBJECT_BARLINE:
        return;
    case OBJECT_CLEF:
    case OBJECT_KEY_SIG:
    case OBJECT_TIME_SIG:
    {
        if (object_is_header(object))
        {
            return;
        }
        struct ObjectIter iter;
        initialize_page_element_iter(&iter.base, object, sizeof(struct Object));
        remove_object_tree_at_iter(&iter, project);
        return;
    }
    case OBJECT_DURATION:
        if (object->duration.is_pitched)
        {
            object->duration.is_pitched = false;
            uint32_t note_address = object->address;
            if (object->duration.pitch.accidental_object_address)
            {
                object->is_valid_cursor_position = true;
                struct ObjectIter accidental_iter;
                initialize_page_element_iter(&accidental_iter.base,
                    resolve_address(project, object->duration.pitch.accidental_object_address),
                    sizeof(struct Object));
                remove_object_at_iter(&accidental_iter, project);
                object = resolve_address(project, note_address);
            }
            reset_accidental_displays_from_previous_key_sig(object, project);
        }
    }
}

bool object_is_header(struct Object*object)
{
    if (object->slice_address)
    {
        if (object->slice_address >= HEADER_CLEF_SLICE_ADDRESS &&
            object->slice_address <= HEADER_TIME_SIG_SLICE_ADDRESS)
        {
            return true;
        }
    }
    return false;
}

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

struct Object*overwrite_range_object(struct ObjectIter*range_start, struct Project*project,
    struct Rational*whole_notes_after_current_slice, struct SliceIter*slice_iter,
    uint32_t range_end_address, uint32_t staff_index)
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
                initialize_pool_integer(&INTEGER_POOL(project), 0);
            slice_iter->slice->whole_notes_long.denominator =
                initialize_pool_integer(&INTEGER_POOL(project), 1);
            break;
        }
        if (previous_slice->whole_notes_long.denominator)
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
                break;
            }
            if (!comparison)
            {
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
    struct Rational remaining_whole_notes_long = previous_duration_whole_notes_long;
    struct SliceIter duration_slice_iter;
    while (true)
    {
        if (iter->object->slice_address)
        {
            struct Slice*slice = resolve_address(project, iter->object->slice_address);
            if (slice->whole_notes_long.denominator)
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
            struct Object*final_staff_object = overwrite_range_object(&range_to_remove_start,
                project, &remaining_whole_notes_long, &duration_end_slice_iter, 0, staff_index);
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
            return;
        }
        if (range_to_remove_end.object->slice_address)
        {
            struct Slice*staff_slice =
                resolve_address(project, range_to_remove_end.object->slice_address);
            while (duration_end_slice_iter.slice != staff_slice)
            {
                if (duration_end_slice_iter.slice->whole_notes_long.denominator)
                {
                    if (compare_rationals(&remaining_whole_notes_long,
                        &duration_end_slice_iter.slice->whole_notes_long, &project->stack_a) > 0)
                    {
                        subtract_rationals(&remaining_whole_notes_long,
                            &remaining_whole_notes_long,
                            &duration_end_slice_iter.slice->whole_notes_long, &project->stack_a,
                            &project->stack_b);
                    }
                    else
                    {
                        subtract_rationals(&remaining_whole_notes_long,
                            &duration_end_slice_iter.slice->whole_notes_long,
                            &remaining_whole_notes_long, &project->stack_a, &project->stack_b);
                        while (true)
                        {
                            increment_page_element_iter(&duration_end_slice_iter.base,
                                &project->page_pool, sizeof(struct Slice));
                            if (duration_end_slice_iter.slice == staff_slice)
                            {
                                goto duration_end_found;
                            }
                            add_rationals(&remaining_whole_notes_long, &remaining_whole_notes_long,
                                &duration_end_slice_iter.slice->whole_notes_long, &project->stack_a,
                                &project->stack_b);
                        }
                    }
                }
                increment_page_element_iter(&duration_end_slice_iter.base, &project->page_pool,
                    sizeof(struct Slice));
            }
        }
        increment_page_element_iter(&range_to_remove_end.base, &project->page_pool,
            sizeof(struct Object));
    }
duration_end_found:
    uint32_t range_to_remove_end_address = range_to_remove_end.object->address;
    int8_t rest_duration_log2 = 0;
    struct Rational rest_whole_notes_long =
    { &(struct Integer) { 1, 1 }, &(struct Integer) { 1, 1 } };
    while (remaining_whole_notes_long.denominator->value_count)
    {
        struct Division division;
        divide_integers(&division, remaining_whole_notes_long.numerator,
            remaining_whole_notes_long.denominator, &project->stack_a, &project->stack_b);
        halve_integer_in_place(remaining_whole_notes_long.denominator);
        if (division.quotient->value_count)
        {
            remaining_whole_notes_long.numerator = division.remainder;
            struct Object*rest = overwrite_range_object(&range_to_remove_start, project,
                &previous_duration_whole_notes_long, &duration_slice_iter,
                range_to_remove_end_address, staff_index);
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

void get_letter_name_accidentals_from_key_sig(struct KeySig*key_sig, uint8_t*out)
{
    for (uint_fast8_t i = 0; i < key_sig->accidental_count; ++i)
    {
        struct KeySigAccidental key_sig_accidental = key_sig->accidentals[i];
        out[key_sig_accidental.letter_name] = key_sig_accidental.accidental;
    }
}

struct PitchNode
{
    struct PitchNode*next_pitch;
    struct Pitch pitch;
};

void reset_accidental_displays(struct ObjectIter*iter, struct Project*project,
    uint8_t*key_sig_accidentals)
{
    void*stack_a_savepoint = project->stack_a.cursor;
    struct PitchNode*note_pitches[7] = { 0 };
    do
    {
        switch (iter->object->object_type)
        {
        case OBJECT_DURATION:
            if (iter->object->duration.is_pitched)
            {
                int8_t letter_name =
                    pitch_to_letter_name(iter->object->duration.pitch.pitch.steps_above_c4);
                bool show_accidental;
                struct PitchNode**scale_degree_pitches = note_pitches + letter_name;
                struct PitchNode*scale_degree_pitch = *scale_degree_pitches;
                while (true)
                {
                    if (!scale_degree_pitch)
                    {
                        show_accidental = key_sig_accidentals[letter_name] !=
                            iter->object->duration.pitch.pitch.accidental;
                        struct PitchNode*new_node = allocate_stack_slot(&project->stack_a,
                            sizeof(struct PitchNode), _alignof(struct PitchNode));
                        new_node->next_pitch = *scale_degree_pitches;
                        *scale_degree_pitches = new_node;
                        new_node->pitch = iter->object->duration.pitch.pitch;
                        break;
                    }
                    if (scale_degree_pitch->pitch.steps_above_c4 ==
                        iter->object->duration.pitch.pitch.steps_above_c4)
                    {
                        show_accidental = scale_degree_pitch->pitch.accidental !=
                            iter->object->duration.pitch.pitch.accidental;
                        scale_degree_pitch->pitch = iter->object->duration.pitch.pitch;
                        break;
                    }
                    if (scale_degree_pitch->pitch.accidental !=
                        iter->object->duration.pitch.pitch.accidental)
                    {
                        show_accidental = true;
                        struct PitchNode*new_node = allocate_stack_slot(&project->stack_a,
                            sizeof(struct PitchNode), _alignof(struct PitchNode));
                        new_node->next_pitch = *scale_degree_pitches;
                        *scale_degree_pitches = new_node;
                        new_node->pitch = iter->object->duration.pitch.pitch;
                        break;
                    }
                    scale_degree_pitch = scale_degree_pitch->next_pitch;
                }
                if (iter->object->duration.pitch.accidental_object_address)
                {
                    if (!show_accidental)
                    {
                        uint32_t note_address = iter->object->address;
                        struct ObjectIter accidental_iter;
                        initialize_page_element_iter(&accidental_iter.base,
                            resolve_address(project,
                                iter->object->duration.pitch.accidental_object_address),
                            sizeof(struct Object));
                        remove_object_at_iter(&accidental_iter, project);
                        initialize_page_element_iter(&iter->base,
                            resolve_address(project, note_address), sizeof(struct Object));
                        iter->object->duration.pitch.accidental_object_address = 0;
                        iter->object->is_valid_cursor_position = true;
                    }
                }
                else if (show_accidental)
                {
                    uint32_t note_address = iter->object->address;
                    insert_sliceless_object_before_iter(iter, project);
                    iter->object->accidental_note_address = note_address;
                    iter->object->object_type = OBJECT_ACCIDENTAL;
                    iter->object->is_selected = false;
                    iter->object->is_valid_cursor_position = true;
                    struct Object*note = resolve_address(project, note_address);
                    note->is_valid_cursor_position = false;
                    note->duration.pitch.accidental_object_address = iter->object->address;
                    initialize_page_element_iter(&iter->base, note, sizeof(struct Object));
                }
            }
            break;
        case OBJECT_KEY_SIG:
            project->stack_a.cursor = stack_a_savepoint;
            return;
        }
        increment_page_element_iter(&iter->base, &project->page_pool, sizeof(struct Object));
    } while (iter->object);
    project->stack_a.cursor = stack_a_savepoint;
}

void reset_accidental_displays_from_previous_key_sig(struct Object*object, struct Project*project)
{
    uint8_t key_sig_accidentals[] =
    { NATURAL, NATURAL, NATURAL, NATURAL, NATURAL, NATURAL, NATURAL };
    struct ObjectIter previous_key_sig_iter;
    initialize_page_element_iter(&previous_key_sig_iter.base, object, sizeof(struct Object));
    struct ObjectIter leftmost_object_to_reset_iter;
    while (true)
    {
        leftmost_object_to_reset_iter = previous_key_sig_iter;
        decrement_page_element_iter(&previous_key_sig_iter.base, &project->page_pool,
            sizeof(struct Object));
        if (!previous_key_sig_iter.object)
        {
            break;
        }
        if (previous_key_sig_iter.object->object_type == OBJECT_KEY_SIG)
        {
            get_letter_name_accidentals_from_key_sig(&previous_key_sig_iter.object->key_sig,
                key_sig_accidentals);
            break;
        }
    }
    reset_accidental_displays(&leftmost_object_to_reset_iter, project, key_sig_accidentals);
}

void get_key_sig(struct KeySig*out, bool is_flats)
{
    uint8_t accidental_type;
    uint_fast8_t stride;
    uint_fast8_t next_letter_name;
    if (is_flats)
    {
        memcpy(out->floors, (int8_t[]) { -4, -5, -4, -5, -1, -2, -3 }, sizeof(out->floors));
        accidental_type = FLAT;
        stride = 3;
        next_letter_name = LETTER_NAME_B;
    }
    else
    {
        memcpy(out->floors, (int8_t[]) { -2, -3, -4, -5, -1, -2, -1 }, sizeof(out->floors));
        accidental_type = SHARP;
        stride = 4;
        next_letter_name = LETTER_NAME_F;
    }
    for (uint_fast8_t i = 0; i < out->accidental_count; ++i)
    {
        out->accidentals[i] = (struct KeySigAccidental) { accidental_type, next_letter_name };
        next_letter_name = (next_letter_name + stride) % 7;
    }
}

void cancel_selection(HWND main_window_handle)
{
    struct Project*project = (struct Project*)GetWindowLongPtrW(main_window_handle, GWLP_USERDATA);
    if (project->selection.selection_type == SELECTION_NONE)
    {
        return;
    }
    if (project->selection.selection_type == SELECTION_OBJECT)
    {
        ((struct Object*)resolve_address(project, project->selection.address.object_address))->
            is_selected = false;
    }
    enable_add_header_object_buttons(project, FALSE);
    project->selection.selection_type = SELECTION_NONE;
}

void set_cursor_to_next_valid_state(struct Project*project)
{
    struct ObjectIter iter;
    initialize_page_element_iter(&iter.base,
        resolve_address(project, project->selection.address.object_address), sizeof(struct Object));
    struct Object*object_right_of_cursor = iter.object;
    int8_t new_range_floor = project->selection.range_floor;
    while (true)
    {
        switch (object_right_of_cursor->object_type)
        {
        case OBJECT_CLEF:
            project->selection.range_floor =
                get_staff_middle_pitch(&object_right_of_cursor->clef) - 3;
            break;
        case OBJECT_DURATION:
            if (object_right_of_cursor->duration.is_pitched)
            {
                project->selection.range_floor = clamped_subtract(object_right_of_cursor->
                    duration.pitch.pitch.steps_above_c4, 3);
            }
        }
        increment_page_element_iter(&iter.base, &project->page_pool, sizeof(struct Object));
        if (!iter.object)
        {
            break;
        }
        object_right_of_cursor = iter.object;
        if (object_right_of_cursor->is_valid_cursor_position)
        {
            break;
        }
    }
    project->selection.address.object_address = object_right_of_cursor->address;
    enable_add_header_object_buttons(project, TRUE);
}

struct Slice*get_next_slice_right_of_iter(struct ObjectIter*iter, struct Project*project)
{
    while (!iter->object->slice_address)
    {
        increment_page_element_iter(&iter->base, &project->page_pool, sizeof(struct Object));
    }
    return resolve_address(project, iter->object->slice_address);
}

struct Slice*get_next_slice_right_of_object(struct Object*object, struct Project*project)
{
    struct ObjectIter iter;
    initialize_page_element_iter(&iter.base, object, sizeof(struct Object));
    return get_next_slice_right_of_iter(&iter, project);
}

void initialize_slice_iter_to_ut_leftmost_to_draw(struct PositionedSliceIter*out,
    struct Project*project)
{
    initialize_page_element_iter((struct BaseIter*)out,
        resolve_address(project, project->address_of_leftmost_slice_to_draw), sizeof(struct Slice));
    out->uz_slice_x = project->utuz_x_of_slice_beyond_leftmost_to_draw +
        out->iter.slice->uz_distance_from_previous_slice;
}

void initialize_slice_iter_to_t_leftmost_to_draw(struct PositionedSliceIter*out,
    struct Project*project)
{
    initialize_slice_iter_to_ut_leftmost_to_draw(out, project);
    out->uz_slice_x -= project->uz_viewport_offset.x;
}

void increment_slice_iter(struct Pool*page_pool, struct PositionedSliceIter*iter)
{
    increment_page_element_iter((struct BaseIter*)iter, page_pool, sizeof(struct Slice));
    if (iter->iter.slice)
    {
        iter->uz_slice_x += iter->iter.slice->uz_distance_from_previous_slice;
    }
}

void decrement_slice_iter(struct Pool*page_pool, struct PositionedSliceIter*iter)
{
    iter->uz_slice_x -= iter->iter.slice->uz_distance_from_previous_slice;
    decrement_page_element_iter(&iter->iter.base, page_pool, sizeof(struct Slice));
}

struct Object*get_nth_object_on_staff(struct Project*project, uint32_t staff_index, uint8_t n)
{
    return (struct Object*)((struct Page*)resolve_pool_index(&project->page_pool,
        ((struct Staff*)resolve_pool_index(&project->staff_pool,
            staff_index))->object_page_index))->bytes + n;
}

struct Object*get_leftmost_staff_object_to_draw(struct PositionedSliceIter*iter,
    struct Project*project, uint32_t staff_index)
{
    while (true)
    {
        uint32_t node_index = iter->iter.slice->first_object_address_node_index;
        while (node_index)
        {
            struct AddressNode*node =
                resolve_pool_index(&ADDRESS_NODE_POOL(project), node_index);
            if (node->address.staff_index == staff_index)
            {
                return resolve_address(project, node->address.object_address);
            }
            node_index = node->index_of_next;
        }
        decrement_slice_iter(&project->page_pool, iter);
    }
}

void insert_slice_before_iter(struct SliceIter*iter, struct Project*project)
{
    insert_page_element_before_iter(&iter->base, project, sizeof(struct Slice));
    iter->slice->uz_distance_from_previous_slice = 0;
    iter->slice->first_object_address_node_index = 0;
    iter->slice->needs_respacing = false;
}

int8_t get_staff_middle_pitch(struct Clef*clef)
{
    int8_t baseline_pitch;
    switch (clef->codepoint)
    {
    case 0xe050:
    case 0xe069:
        baseline_pitch = 4;
        break;
    case 0xe051:
        baseline_pitch = -10;
        break;
    case 0xe052:
        baseline_pitch = -3;
        break;
    case 0xe053:
        baseline_pitch = 11;
        break;
    case 0xe054:
        baseline_pitch = 18;
        break;
    case 0xe05c:
        baseline_pitch = 0;
        break;
    case 0xe05d:
        baseline_pitch = -7;
        break;
    case 0xe062:
        baseline_pitch = -4;
        break;
    case 0xe063:
        baseline_pitch = -18;
        break;
    case 0xe064:
        baseline_pitch = -11;
        break;
    case 0xe065:
        baseline_pitch = 3;
        break;
    case 0xe066:
        baseline_pitch = 10;
    }
    return baseline_pitch - clef->steps_of_baseline_above_staff_middle;
}

int8_t get_staff_bottom_line_pitch(uint8_t line_count, int8_t middle_pitch)
{
    return middle_pitch - line_count + 1;
}