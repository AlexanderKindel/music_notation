#include "declarations.h"
#include "memory.c"
#include "rational.c"
#include "duration.c"

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

void add_object_to_slice(struct ObjectIter*iter, struct Project*project, struct Slice*slice,
    uint32_t staff_index)
{
    iter->object->slice_address = slice->address;
    struct AddressNode*node = allocate_pool_slot(ADDRESS_NODE_POOL(project));
    node->address.object_address = iter->object->address;
    node->address.staff_index = staff_index;
    node->index_of_next = slice->first_object_address_node_index;
    slice->first_object_address_node_index =
        get_element_index_in_pool(ADDRESS_NODE_POOL(project), node);
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
    struct Slice*slice, uint32_t staff_index)
{
    insert_page_element_before_iter(&iter->base, project, sizeof(struct Object));
    add_object_to_slice(iter, project, slice, staff_index);
}

void remove_object_from_slice(struct ObjectIter*iter, struct Project*project)
{
    struct SliceIter slice_iter;
    initialize_page_element_iter(&slice_iter.base,
        resolve_address(project, iter->object->slice_address), sizeof(struct Slice));
    uint32_t*index_of_node = &slice_iter.slice->first_object_address_node_index;
    while (true)
    {
        struct AddressNode*node = resolve_pool_index(ADDRESS_NODE_POOL(project), *index_of_node);
        if (node->address.object_address == iter->object->address)
        {
            *index_of_node = node->index_of_next;
            free_pool_slot(ADDRESS_NODE_POOL(project), node);
            break;
        }
        *index_of_node = node->index_of_next;
    }
    if (slice_iter.slice->first_object_address_node_index)
    {
        slice_iter.slice->needs_respacing = true;
        increment_page_element_iter(&slice_iter.base, &project->page_pool, sizeof(struct Slice));
    }
    else
    {
        struct SliceIter previous_duration_slice_iter = slice_iter;
        while (true)
        {
            decrement_page_element_iter(&previous_duration_slice_iter.base, &project->page_pool,
                sizeof(struct Slice));
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

void remove_object_at_iter(struct ObjectIter*iter, struct Project*project)
{
    if (project->ghost_cursor_address.object_address == iter->object->address)
    {
        project->ghost_cursor_address.staff_index = 0;
    }
    if (iter->object->address)
    {
        remove_object_from_slice(iter, project);
    }
    else
    {
        struct ObjectIter iter_copy = *iter;
        get_next_slice_right_of_iter(&iter_copy, project)->needs_respacing = true;
    }
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
                    iter->object->is_hidden = false;
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

void get_key_sig(struct KeySig*out, struct Project*project)
{
    uint8_t accidental_type;
    uint_fast8_t stride;
    uint_fast8_t next_letter_name;
    if (SendMessageW(project->flats_handle, BM_GETCHECK, 0, 0) == BST_CHECKED)
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
            struct AddressNode*node = resolve_pool_index(ADDRESS_NODE_POOL(project), node_index);
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
    iter->slice->rod_intersection_count = 0;
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