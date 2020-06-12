#include "declarations.h"

int8_t get_staff_middle_pitch_at_viewport_left_edge(struct Project*project, struct Staff*staff)
{
    return get_staff_middle_pitch(&((struct Object*)resolve_address(project,
        staff->address_of_clef_beyond_leftmost_slice_to_draw))->clef);
}

void reset_viewport_offset_x(HWND main_window_handle, struct Project*project,
    int32_t uz_new_offset_x)
{
    if (uz_new_offset_x > project->utuz_last_slice_x)
    {
        uz_new_offset_x = project->utuz_last_slice_x;
    }
    else
    {
        RECT client_rect;
        GetClientRect(main_window_handle, &client_rect);
        int32_t uz_minimum_allowed_offset =
            ((struct Slice*)resolve_page_index(project->slices, 1, sizeof(struct Slice)))->
                uz_distance_from_previous_slice -
            zoom_coordinate(client_rect.right, 1.0 / get_zoom_factor(project->zoom_exponent));
        if (uz_new_offset_x < uz_minimum_allowed_offset)
        {
            uz_new_offset_x = uz_minimum_allowed_offset;
        }
    }
    struct PositionedSliceIter new_leftmost_slice_to_draw_iter;
    initialize_slice_iter_to_ut_leftmost_to_draw(&new_leftmost_slice_to_draw_iter, project);
    HDC device_context = GetDC(main_window_handle);
    if (uz_new_offset_x < project->uz_viewport_offset.x)
    {
        while (true)
        {
            while (uz_new_offset_x < new_leftmost_slice_to_draw_iter.uz_slice_x)
            {
                struct PositionedSliceIter current_slice_iter = new_leftmost_slice_to_draw_iter;
                decrement_slice_iter(&project->page_pool, &new_leftmost_slice_to_draw_iter);
                if (!new_leftmost_slice_to_draw_iter.iter.slice)
                {
                    new_leftmost_slice_to_draw_iter = current_slice_iter;
                    goto new_leftmost_slice_to_draw_found;
                }
                if (new_leftmost_slice_to_draw_iter.iter.slice->needs_respacing)
                {
                    uz_new_offset_x += respace_slice_range(device_context,
                        &new_leftmost_slice_to_draw_iter, project);
                }
            }
            struct PositionedSliceIter respacing_iter = new_leftmost_slice_to_draw_iter;
            while (true)
            {
                if (!respacing_iter.iter.slice->rod_intersection_count)
                {
                    goto new_leftmost_slice_to_draw_found;
                }
                decrement_slice_iter(&project->page_pool, &respacing_iter);
                if (respacing_iter.iter.slice->needs_respacing)
                {
                    uz_new_offset_x +=
                        respace_slice_range(device_context, &respacing_iter, project);
                    new_leftmost_slice_to_draw_iter = respacing_iter;
                    break;
                }
            }
        }
    new_leftmost_slice_to_draw_found:
        for (struct Staff*staff = resolve_pool_index(&project->staff_pool, 1);
            staff < project->staff_pool.cursor; ++staff)
        {
            if (staff->is_on_free_list ||
                staff->address_of_clef_beyond_leftmost_slice_to_draw == HEADER_CLEF_SLICE_ADDRESS)
            {
                continue;
            }
            struct ObjectIter iter;
            initialize_page_element_iter(&iter.base,
                get_leftmost_staff_object_to_draw(&new_leftmost_slice_to_draw_iter, project,
                    get_element_index_in_pool(&project->staff_pool, staff)),
                sizeof(struct Object));
            do
            {
                if (iter.object->object_type == OBJECT_CLEF)
                {
                    staff->address_of_clef_beyond_leftmost_slice_to_draw = iter.object->address;
                    break;
                }
                decrement_page_element_iter(&iter.base, &project->page_pool, sizeof(struct Object));
            } while (iter.object);
        }
    }
    else
    {
        while (true)
        {
            struct PositionedSliceIter current_slice_iter = new_leftmost_slice_to_draw_iter;
            increment_slice_iter(&project->page_pool, &new_leftmost_slice_to_draw_iter);
            if (!new_leftmost_slice_to_draw_iter.iter.slice ||
                new_leftmost_slice_to_draw_iter.uz_slice_x > uz_new_offset_x)
            {
                new_leftmost_slice_to_draw_iter = current_slice_iter;
                break;
            }
        }
        for (struct Staff*staff = resolve_pool_index(&project->staff_pool, 1);
            staff < project->staff_pool.cursor; ++staff)
        {
            if (staff->is_on_free_list)
            {
                continue;
            }
            struct ObjectIter iter;
            initialize_page_element_iter(&iter.base,
                get_leftmost_staff_object_to_draw(&new_leftmost_slice_to_draw_iter, project,
                    get_element_index_in_pool(&project->staff_pool, staff)),
                sizeof(struct Object));
            struct SliceIter slice_iter = new_leftmost_slice_to_draw_iter.iter;
            while (true)
            {
                if (iter.object->object_type == OBJECT_CLEF)
                {
                    staff->address_of_clef_beyond_leftmost_slice_to_draw = iter.object->address;
                    break;
                }
                decrement_page_element_iter(&iter.base, &project->page_pool, sizeof(struct Object));
                if (!iter.object)
                {
                    break;
                }
                if (iter.object->slice_address)
                {
                    do
                    {
                        decrement_page_element_iter(&slice_iter.base, &project->page_pool,
                            sizeof(struct Slice));
                        if (slice_iter.slice->address == project->address_of_leftmost_slice_to_draw)
                        {
                            goto break_both_loops;
                        }
                    } while (slice_iter.slice->address != iter.object->slice_address);
                }
            }
        break_both_loops:;
        }
    }
    ReleaseDC(main_window_handle, device_context);
    project->utuz_x_of_slice_beyond_leftmost_to_draw =
        new_leftmost_slice_to_draw_iter.uz_slice_x -
        new_leftmost_slice_to_draw_iter.iter.slice->uz_distance_from_previous_slice;
    project->address_of_leftmost_slice_to_draw =
        new_leftmost_slice_to_draw_iter.iter.slice->address;
    project->uz_viewport_offset.x = uz_new_offset_x;
}

void reset_viewport_offset_y(HWND main_window_handle, struct Project*project,
    int32_t uz_new_offset_y)
{
    if (!project->topmost_staff_index)
    {
        project->uz_viewport_offset.y = 0;
        return;
    }
    float inverse_zoom_factor = 1.0 / get_zoom_factor(project->zoom_exponent);
    int32_t uz_maximum_allowed_offset = project->utuz_bottom_staff_y -
        zoom_coordinate(CONTROL_TABS_HEIGHT, inverse_zoom_factor);
    if (uz_new_offset_y > uz_maximum_allowed_offset)
    {
        uz_new_offset_y = uz_maximum_allowed_offset;
    }
    else
    {
        RECT client_rect;
        GetClientRect(main_window_handle, &client_rect);
        int32_t uz_minimum_allowed_offset = ((struct Staff*)resolve_pool_index(&project->staff_pool,
            project->topmost_staff_index))->uz_distance_from_staff_above -
            zoom_coordinate(client_rect.bottom, inverse_zoom_factor);
        if (uz_new_offset_y < uz_minimum_allowed_offset)
        {
            uz_new_offset_y = uz_minimum_allowed_offset;
        }
    }
    project->uz_viewport_offset.y = uz_new_offset_y;
    struct Staff*highest_visible_staff =
        resolve_pool_index(&project->staff_pool, project->highest_visible_staff_index);
    int32_t utuz_highest_visible_staff_y = project->utuz_y_of_staff_above_highest_visible +
        highest_visible_staff->uz_distance_from_staff_above;
    if (uz_new_offset_y < utuz_highest_visible_staff_y)
    {
        while (highest_visible_staff->index_of_staff_above)
        {
            highest_visible_staff = resolve_pool_index(&project->staff_pool,
                highest_visible_staff->index_of_staff_above);
            utuz_highest_visible_staff_y -= highest_visible_staff->uz_distance_from_staff_above;
            project->highest_visible_staff_index = highest_visible_staff->index_of_staff_above;
            if (utuz_highest_visible_staff_y <= uz_new_offset_y)
            {
                break;
            }
        }
        project->utuz_y_of_staff_above_highest_visible =
            utuz_highest_visible_staff_y - highest_visible_staff->uz_distance_from_staff_above;
    }
    else
    {
        while (highest_visible_staff->index_of_staff_below)
        {
            uint32_t next_staff_down_index = highest_visible_staff->index_of_staff_below;
            highest_visible_staff = resolve_pool_index(&project->staff_pool, next_staff_down_index);
            int32_t utuz_next_staff_down_y =
                utuz_highest_visible_staff_y + highest_visible_staff->uz_distance_from_staff_above;
            if (utuz_next_staff_down_y <= uz_new_offset_y)
            {
                project->utuz_y_of_staff_above_highest_visible = utuz_highest_visible_staff_y;
                project->highest_visible_staff_index = next_staff_down_index;
                utuz_highest_visible_staff_y = utuz_next_staff_down_y;
            }
            else
            {
                break;
            }
        }
    }
}