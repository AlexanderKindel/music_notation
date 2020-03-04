#include "declarations.h"
#include "clef_tab.c"
#include "staff_tab.c"

int32_t get_text_width(HDC device_context, wchar_t*text, size_t text_length)
{
    SIZE size;
    GetTextExtentPoint32W(device_context, text, text_length, &size);
    return size.cx;
}

void center_dialog(HWND dialog_handle, int window_width, int window_height)
{
    RECT desktop_rect;
    GetWindowRect(GetDesktopWindow(), &desktop_rect);
    RECT window_rect = { 0, 0, window_width, window_height };
    AdjustWindowRect(&window_rect, GetWindowLongW(dialog_handle, GWL_STYLE), 0);
    window_width = window_rect.right - window_rect.left;
    window_height = window_rect.bottom - window_rect.top;
    MoveWindow(dialog_handle, (desktop_rect.right - desktop_rect.left - window_width) / 2,
        (desktop_rect.bottom - desktop_rect.top - window_height) / 2, window_width,
        window_height, TRUE);
}

LRESULT key_sig_tab_proc(HWND window_handle, UINT message, WPARAM w_param, LPARAM l_param,
    UINT_PTR id_subclass, DWORD_PTR ref_data)
{
    switch (message)
    {
    case WM_COMMAND:
        if (HIWORD(w_param) == BN_CLICKED)
        {
            HWND main_window_handle = GetParent(GetParent(window_handle));
            SetFocus(main_window_handle);
            struct Project*project =
                (struct Project*)GetWindowLongPtrW(main_window_handle, GWLP_USERDATA);
            if (l_param == (LPARAM)project->add_key_sig_button_handle)
            {
                struct KeySig new_key_sig;
                new_key_sig.accidental_count =
                    SendMessageW(project->accidental_count_spin_handle, UDM_GETPOS32, 0, 0);
                if (!new_key_sig.accidental_count)
                {
                    struct ObjectIter previous_key_sig_iter;
                    initialize_page_element_iter(&previous_key_sig_iter.base,
                        resolve_address(project, project->selection.address.object_address),
                        sizeof(struct Object));
                    while (true)
                    {
                        decrement_page_element_iter(&previous_key_sig_iter.base,
                            &project->page_pool, sizeof(struct Object));
                        if (!previous_key_sig_iter.object)
                        {
                            return 0;
                        }
                        if (previous_key_sig_iter.object->object_type == OBJECT_KEY_SIG)
                        {
                            if (!previous_key_sig_iter.object->key_sig.accidental_count)
                            {
                                return 0;
                            }
                            memcpy(new_key_sig.floors, previous_key_sig_iter.object->key_sig.floors,
                                sizeof(new_key_sig.floors));
                            while (new_key_sig.accidental_count <
                                previous_key_sig_iter.object->key_sig.accidental_count)
                            {
                                new_key_sig.accidentals[new_key_sig.accidental_count].accidental =
                                    NATURAL;
                                new_key_sig.accidentals[new_key_sig.accidental_count].letter_name =
                                    previous_key_sig_iter.object->
                                    key_sig.accidentals[new_key_sig.accidental_count].letter_name;
                                ++new_key_sig.accidental_count;
                            }
                            break;
                        }
                    }
                }
                else
                {
                    get_key_sig(&new_key_sig,
                        SendMessageW(project->flats_handle, BM_GETCHECK, 0, 0) == BST_CHECKED);
                }
                struct ObjectIter iter;
                switch (project->selection.selection_type)
                {
                case SELECTION_CURSOR:
                {
                    initialize_page_element_iter(&iter.base,
                        resolve_address(project, project->selection.address.object_address),
                        sizeof(struct Object));
                    insert_sliceless_object_before_iter(&iter, project);
                    iter.object->key_sig = new_key_sig;
                    iter.object->object_type = OBJECT_KEY_SIG;
                    iter.object->is_selected = false;
                    iter.object->is_valid_cursor_position = true;
                    break;
                }
                case SELECTION_OBJECT:
                {
                    initialize_page_element_iter(&iter.base,
                        get_nth_object_on_staff(project, project->selection.address.staff_index, 2),
                        sizeof(struct Object));
                    if (iter.object->slice_address == HEADER_KEY_SIG_SLICE_ADDRESS)
                    {
                        iter.object->key_sig = new_key_sig;
                        ((struct Slice*)resolve_address(project, HEADER_TIME_SIG_SLICE_ADDRESS))->
                            needs_respacing = true;
                    }
                    else
                    {
                        insert_slice_object_before_iter(&iter, project,
                            HEADER_KEY_SIG_SLICE_ADDRESS, project->selection.address.staff_index);
                        iter.object->key_sig = new_key_sig;
                        iter.object->object_type = OBJECT_KEY_SIG;
                        iter.object->is_selected = false;
                        iter.object->is_valid_cursor_position = false;
                    }
                    cancel_selection(main_window_handle);
                    project->selection.selection_type = SELECTION_CURSOR;
                    project->selection.address.object_address = iter.object->address;
                }
                }
                project->selection.address.object_address = iter.object->address;
                uint8_t key_sig_accidentals[] =
                { NATURAL, NATURAL, NATURAL, NATURAL, NATURAL, NATURAL, NATURAL };
                get_letter_name_accidentals_from_key_sig(&iter.object->key_sig,
                    key_sig_accidentals);
                struct Object*new_key_sig_object = iter.object;
                increment_page_element_iter(&iter.base, &project->page_pool, sizeof(struct Object));
                reset_accidental_displays(&iter, project, key_sig_accidentals);
                if (iter.object)
                {
                    if (new_key_sig.accidentals[0].accidental == NATURAL)
                    {
                        if (iter.object->key_sig.accidentals[0].accidental == NATURAL)
                        {
                            remove_object_at_iter(&iter, project);
                        }
                    }
                    else if (iter.object->key_sig.accidentals[0].accidental == NATURAL)
                    {
                        memcpy(iter.object->key_sig.floors, new_key_sig_object->key_sig.floors,
                            sizeof(iter.object->key_sig.floors));
                        iter.object->key_sig.accidental_count =
                            new_key_sig_object->key_sig.accidental_count;
                        for (uint_fast8_t i = 0; iter.object->key_sig.accidental_count; ++i)
                        {
                            iter.object->key_sig.accidentals[i].accidental = NATURAL;
                            iter.object->key_sig.accidentals[i].letter_name =
                                new_key_sig_object->key_sig.accidentals[i].letter_name;
                        }
                    }
                }
                set_cursor_to_next_valid_state(project);
                invalidate_work_region(main_window_handle, project);
                return 0;
            }
        }
        break;
    case WM_NOTIFY:
        if (((LPNMHDR)l_param)->code == UDN_DELTAPOS)
        {
            struct Project*project = (struct Project*)GetWindowLongPtrW(
                GetParent(GetParent(window_handle)), GWLP_USERDATA);
            bool enable = ((LPNMUPDOWN)l_param)->iPos + ((LPNMUPDOWN)l_param)->iDelta > 0;
            EnableWindow(project->flats_handle, enable);
            EnableWindow(project->sharps_handle, enable);
            return 0;
        }
    }
    return DefWindowProcW(window_handle, message, w_param, l_param);
}

LRESULT note_tab_proc(HWND window_handle, UINT message, WPARAM w_param, LPARAM l_param,
    UINT_PTR id_subclass, DWORD_PTR ref_data)
{
    if (message == WM_NOTIFY)
    {
        LPNMHDR lpmhdr = (LPNMHDR)l_param;
        if (lpmhdr->code == UDN_DELTAPOS)
        {
            struct Project*project = (struct Project*)GetWindowLongPtrW(
                GetParent(GetParent(window_handle)), GWLP_USERDATA);
            LPNMUPDOWN lpnmud = (LPNMUPDOWN)l_param;
            if (lpmhdr->hwndFrom == project->duration_spin_handle)
            {
                int32_t new_position = lpnmud->iPos + lpnmud->iDelta;
                wchar_t buffer[7];
                wchar_t*new_text;
                if (new_position > MAX_LOG2_DURATION)
                {
                    SendMessageW(project->augmentation_dot_spin_handle, UDM_SETRANGE32, 0, 11);
                    new_text = L"double whole";
                }
                else if (new_position < MIN_LOG2_DURATION)
                {
                    SendMessageW(project->augmentation_dot_spin_handle, UDM_SETRANGE32, 0, 0);
                    SendMessageW(project->augmentation_dot_spin_handle, UDM_SETPOS32, 0, 0);
                    new_text = L"1024th";
                }
                else
                {
                    int32_t new_max_dot_count = new_position - MIN_LOG2_DURATION;
                    if (SendMessageW(project->augmentation_dot_spin_handle, UDM_GETPOS32, 0, 0) >
                        new_max_dot_count)
                    {
                        SendMessageW(project->augmentation_dot_spin_handle, UDM_SETPOS32, 0,
                            new_max_dot_count);
                    }
                    SendMessageW(project->augmentation_dot_spin_handle, UDM_SETRANGE32, 0,
                        new_max_dot_count);
                    switch (new_position)
                    {
                    case 1:
                        new_text = L"double whole";
                        break;
                    case 0:
                        new_text = L"whole";
                        break;
                    case -1:
                        new_text = L"half";
                        break;
                    case -2:
                        new_text = L"quarter";
                        break;
                    default:
                    {
                        new_text = buffer;
                        uint16_t denominator = 1 << -new_position;
                        integer_to_wchar_string(&new_text, denominator, L'0', 4);
                        if (denominator % 10 == 2)
                        {
                            memcpy(buffer + 4, L"nd", sizeof(L"nd"));
                        }
                        else
                        {
                            memcpy(buffer + 4, L"th", sizeof(L"th"));
                        }
                    }
                    }
                }
                SendMessageW(project->duration_display_handle, WM_SETTEXT, 0, (LPARAM)new_text);
                return 0;
            }
        }
    }
    return DefWindowProcW(window_handle, message, w_param, l_param);
}

void get_selected_time_sig(struct Project*project, struct TimeSig*out)
{
    out->numerator = SendMessageW(project->numerator_spin_handle, UDM_GETPOS32, 0, 0);
    out->denominator = 1 << -SendMessageW(project->denominator_spin_handle, UDM_GETPOS32, 0, 0);
}

LRESULT time_sig_tab_proc(HWND window_handle, UINT message, WPARAM w_param, LPARAM l_param,
    UINT_PTR id_subclass, DWORD_PTR ref_data)
{
    switch (message)
    {
    case WM_COMMAND:
        if (HIWORD(w_param) == BN_CLICKED)
        {
            HWND main_window_handle = GetParent(GetParent(window_handle));
            SetFocus(main_window_handle);
            struct Project*project =
                (struct Project*)GetWindowLongPtrW(main_window_handle, GWLP_USERDATA);
            if (l_param == (LPARAM)project->add_time_sig_button_handle)
            {
                struct ObjectIter time_sig;
                initialize_page_element_iter(&time_sig.base,
                    resolve_address(project, project->selection.address.object_address),
                    sizeof(struct Object));
                switch (project->selection.selection_type)
                {
                case SELECTION_CURSOR:
                {
                    insert_sliceless_object_before_iter(&time_sig, project);
                    get_selected_time_sig(project, &time_sig.object->time_sig);
                    time_sig.object->object_type = OBJECT_TIME_SIG;
                    time_sig.object->is_selected = false;
                    time_sig.object->is_valid_cursor_position = true;
                    break;
                }
                case SELECTION_OBJECT:
                {
                    project->selection.range_floor =
                        get_staff_middle_pitch(&get_nth_object_on_staff(project,
                            project->selection.address.staff_index, 1)->clef) - 3;
                    struct ObjectIter iter = time_sig;
                    while (true)
                    {
                        if (iter.object->slice_address == HEADER_TIME_SIG_SLICE_ADDRESS)
                        {
                            get_selected_time_sig(project, &time_sig.object->time_sig);
                            break;
                        }
                        else if (iter.object->slice_address > HEADER_TIME_SIG_SLICE_ADDRESS)
                        {
                            insert_slice_object_before_iter(&time_sig, project,
                                HEADER_TIME_SIG_SLICE_ADDRESS,
                                project->selection.address.staff_index);
                            get_selected_time_sig(project, &time_sig.object->time_sig);
                            time_sig.object->object_type = OBJECT_TIME_SIG;
                            time_sig.object->is_selected = false;
                            time_sig.object->is_valid_cursor_position = false;
                            break;
                        }
                        increment_page_element_iter(&iter.base, &project->page_pool,
                            sizeof(struct Object));
                    }
                    time_sig.object = iter.object;
                    cancel_selection(main_window_handle);
                    project->selection.selection_type = SELECTION_CURSOR;
                }
                }
                project->selection.address.object_address = time_sig.object->address;
                set_cursor_to_next_valid_state(project);
                invalidate_work_region(main_window_handle, project);
                return 0;
            }
        }
        break;
    case WM_NOTIFY:
    {
        NMHDR*notification_header = (NMHDR*)l_param;
        if (notification_header->code == UDN_DELTAPOS)
        {
            struct Project*project = (struct Project*)GetWindowLongPtrW(
                GetParent(GetParent(window_handle)), GWLP_USERDATA);
            NMUPDOWN*updown_notification = (NMUPDOWN*)l_param;
            int32_t new_position = updown_notification->iPos + updown_notification->iDelta;
            if (notification_header->hwndFrom == project->denominator_spin_handle)
            {
                if (new_position > 0)
                {
                    SendMessageW(project->denominator_display_handle, WM_SETTEXT, 0, (WPARAM)L"1");
                }
                else if (new_position < MIN_LOG2_DURATION)
                {
                    SendMessageW(project->denominator_display_handle, WM_SETTEXT, 0,
                        (WPARAM)L"1024");
                }
                else
                {
                    uint32_t denominator = 1 << -new_position;
                    wchar_t denominator_string[4];
                    denominator_string[3] = 0;
                    size_t character_index = 3;
                    while (denominator)
                    {
                        --character_index;
                        denominator_string[character_index] = denominator % 10 + L'0';
                        denominator /= 10;
                    }
                    SendMessageW(project->denominator_display_handle, WM_SETTEXT, 0,
                        (WPARAM)(denominator_string + character_index));
                }
                return 0;
            }
        }
    }
    }
    return DefWindowProcW(window_handle, message, w_param, l_param);
}

void enable_add_header_object_buttons(struct Project*project, BOOL enable)
{
    EnableWindow(project->add_clef_button_handle, enable);
    EnableWindow(project->add_key_sig_button_handle, enable);
    EnableWindow(project->add_time_sig_button_handle, enable);
}

LRESULT CALLBACK main_window_proc(HWND window_handle, UINT message, WPARAM w_param,
    LPARAM l_param)
{
    switch (message)
    {
    case WM_HSCROLL:
        SetFocus(window_handle);
        invalidate_work_region(window_handle,
            (struct Project*)GetWindowLongPtrW(window_handle, GWLP_USERDATA));
        return 0;
    case WM_KEYDOWN:
        if (65 <= w_param && w_param <= 71)
        {
            struct Project*project =
                (struct Project*)GetWindowLongPtrW(window_handle, GWLP_USERDATA);
            if (project->selection.selection_type != SELECTION_CURSOR)
            {
                return 0;
            }
            struct Duration duration;
            duration.is_pitched = true;
            int8_t octave4_cursor_range_floor = project->selection.range_floor % 7;
            int8_t octaves_of_range_floor_above_octave4 = project->selection.range_floor / 7;
            int8_t scale_degree = (w_param - 60) % 7;
            if (octave4_cursor_range_floor < 0)
            {
                octave4_cursor_range_floor += 7;
                octaves_of_range_floor_above_octave4 -= 1;
            }
            duration.pitch.pitch.steps_above_c4 =
                7 * octaves_of_range_floor_above_octave4 + scale_degree;
            if (octave4_cursor_range_floor > scale_degree)
            {
                duration.pitch.pitch.steps_above_c4 += 7;
            }
            duration.augmentation_dot_count =
                SendMessageW(project->augmentation_dot_spin_handle, UDM_GETPOS32, 0, 0);
            duration.log2 = SendMessageW(project->duration_spin_handle, UDM_GETPOS32, 0, 0);
            project->selection.range_floor =
                clamped_subtract(duration.pitch.pitch.steps_above_c4, 3);
            struct ObjectIter iter;
            initialize_page_element_iter(&iter.base,
                resolve_address(project, project->selection.address.object_address),
                sizeof(struct Object));
            overwrite_with_duration(&duration, &iter, project,
                project->selection.address.staff_index);
            struct DisplayedAccidental accidental = get_default_accidental(iter.object, project);
            iter.object->duration.pitch.pitch.accidental = accidental.accidental;
            if (accidental.is_visible)
            {
                iter.object->is_valid_cursor_position = false;
                uint32_t note_address = iter.object->address;
                insert_sliceless_object_before_iter(&iter, project);
                iter.object->accidental_note_address = note_address;
                iter.object->object_type = OBJECT_ACCIDENTAL;
                iter.object->is_selected = false;
                iter.object->is_valid_cursor_position = true;
                increment_page_element_iter(&iter.base, &project->page_pool, sizeof(struct Object));
            }
            else
            {
                iter.object->duration.pitch.accidental_object_address = 0;
            }
            increment_page_element_iter(&iter.base, &project->page_pool, sizeof(struct Object));
            project->selection.address.object_address = iter.object->address;
            reset_accidental_displays_from_previous_key_sig(iter.object, project);
            invalidate_work_region(window_handle, project);
            return 0;
        }
        switch (w_param)
        {
        case VK_BACK:
        {
            struct Project*project =
                (struct Project*)GetWindowLongPtrW(window_handle, GWLP_USERDATA);
            switch (project->selection.selection_type)
            {
            case SELECTION_CURSOR:
            {
                struct ObjectIter iter;
                initialize_page_element_iter(&iter.base,
                    resolve_address(project, project->selection.address.object_address),
                    sizeof(struct Object));
                decrement_page_element_iter(&iter.base, &project->page_pool, sizeof(struct Object));
                if (iter.object)
                {
                    delete_object(iter.object, project);
                    invalidate_work_region(window_handle, project);
                }
                return 0;
            }
            case SELECTION_NONE:
                return 0;
            case SELECTION_OBJECT:
            {
                delete_object(resolve_address(project, project->selection.address.object_address),
                    project);
                invalidate_work_region(window_handle, project);
            }
            }
            return 0;
        }
        case VK_DELETE:
        {
            struct Project*project =
                (struct Project*)GetWindowLongPtrW(window_handle, GWLP_USERDATA);
            if (project->selection.selection_type == SELECTION_OBJECT)
            {
                delete_object(resolve_address(project, project->selection.address.object_address),
                    project);
                invalidate_work_region(window_handle, project);
            }
            return 0;
        }
        case VK_DOWN:
        {
            struct Project*project =
                (struct Project*)GetWindowLongPtrW(window_handle, GWLP_USERDATA);
            switch (project->selection.selection_type)
            {
            case SELECTION_CURSOR:
                project->selection.range_floor =
                    clamped_subtract(project->selection.range_floor, 7);
                invalidate_work_region(window_handle, project);
                return 0;
            case SELECTION_OBJECT:
            {
                struct Object*object =
                    resolve_address(project, project->selection.address.object_address);
                switch (object->object_type)
                {
                case OBJECT_CLEF:
                {
                    int8_t new_baseline = object->clef.steps_of_baseline_above_staff_middle - 1;
                    if (new_baseline >
                        -(int8_t)((struct Staff*)resolve_pool_index(&project->staff_pool,
                            project->selection.address.staff_index))->line_count)
                    {
                        object->clef.steps_of_baseline_above_staff_middle = new_baseline;
                    }
                    invalidate_work_region(window_handle, project);
                    return 0;
                }
                case OBJECT_DURATION:
                {
                    if (object->duration.is_pitched)
                    {
                        if (HIBYTE(GetKeyState(VK_SHIFT)))
                        {
                            if (object->duration.pitch.pitch.accidental == DOUBLE_FLAT)
                            {
                                return 0;
                            }
                            --object->duration.pitch.pitch.accidental;
                        }
                        else
                        {
                            if (object->duration.pitch.pitch.steps_above_c4 > INT8_MIN)
                            {
                                --object->duration.pitch.pitch.steps_above_c4;
                            }
                            object->duration.pitch.pitch.accidental =
                                get_default_accidental(object, project).accidental;
                        }
                        get_next_slice_right_of_object(object, project)->needs_respacing = true;
                        reset_accidental_displays_from_previous_key_sig(object, project);
                        invalidate_work_region(window_handle, project);
                    }
                }
                }
            }
            }
            return 0;
        }
        case VK_ESCAPE:
            cancel_selection(window_handle);
            invalidate_work_region(window_handle,
                (struct Project*)GetWindowLongPtrW(window_handle, GWLP_USERDATA));
            return 0;
        case VK_LEFT:
        {
            struct Project*project =
                (struct Project*)GetWindowLongPtrW(window_handle, GWLP_USERDATA);
            if (project->selection.selection_type == SELECTION_CURSOR)
            {
                struct Staff*staff = resolve_pool_index(&project->staff_pool,
                    project->selection.address.staff_index);
                struct ObjectIter iter;
                initialize_page_element_iter(&iter.base,
                    resolve_address(project, project->selection.address.object_address),
                    sizeof(struct Object));
                int8_t new_range_floor = project->selection.range_floor;
                while (true)
                {
                    decrement_page_element_iter(&iter.base, &project->page_pool,
                        sizeof(struct Object));
                    if (!iter.object)
                    {
                        break;
                    }
                    if (iter.object->object_type == OBJECT_DURATION)
                    {
                        if (iter.object->duration.is_pitched)
                        {
                            new_range_floor = clamped_subtract(iter.object->
                                duration.pitch.pitch.steps_above_c4, 3);
                        }
                    }
                    if (iter.object->is_valid_cursor_position)
                    {
                        project->selection.address.object_address = iter.object->address;
                        project->selection.range_floor = new_range_floor;
                        invalidate_work_region(window_handle, project);
                        return 0;
                    }
                }
            }
            return 0;
        }
        case VK_RIGHT:
        {
            struct Project*project =
                (struct Project*)GetWindowLongPtrW(window_handle, GWLP_USERDATA);
            if (project->selection.selection_type == SELECTION_CURSOR)
            {
                set_cursor_to_next_valid_state(project);
                invalidate_work_region(window_handle, project);
            }
            return 0;
        }
        case VK_SPACE:
        {
            struct Project*project =
                (struct Project*)GetWindowLongPtrW(window_handle, GWLP_USERDATA);
            if (project->selection.selection_type != SELECTION_CURSOR)
            {
                return 0;
            }
            struct Duration duration;
            duration.is_pitched = false;
            duration.augmentation_dot_count =
                SendMessageW(project->augmentation_dot_spin_handle, UDM_GETPOS32, 0, 0);
            duration.log2 = SendMessageW(project->duration_spin_handle, UDM_GETPOS32, 0, 0);
            struct ObjectIter iter;
            initialize_page_element_iter(&iter.base,
                resolve_address(project, project->selection.address.object_address),
                sizeof(struct Object));
            overwrite_with_duration(&duration, &iter, project,
                project->selection.address.staff_index);
            increment_page_element_iter(&iter.base, &project->page_pool, sizeof(struct Object));
            project->selection.address.object_address = iter.object->address;
            reset_accidental_displays_from_previous_key_sig(iter.object, project);
            invalidate_work_region(window_handle, project);
            return 0;
        }
        case VK_UP:
        {
            struct Project*project =
                (struct Project*)GetWindowLongPtrW(window_handle, GWLP_USERDATA);
            switch (project->selection.selection_type)
            {
            case SELECTION_CURSOR:
                project->selection.range_floor =
                    clamped_add(project->selection.range_floor, 7);
                invalidate_work_region(window_handle, project);
                return 0;
            case SELECTION_OBJECT:
            {
                struct Object*object =
                    resolve_address(project, project->selection.address.object_address);
                switch (object->object_type)
                {
                case OBJECT_CLEF:
                {
                    int8_t new_baseline = object->clef.steps_of_baseline_above_staff_middle + 1;
                    if (new_baseline < ((struct Staff*)resolve_pool_index(&project->staff_pool,
                        project->selection.address.staff_index))->line_count)
                    {
                        object->clef.steps_of_baseline_above_staff_middle = new_baseline;
                    }
                    invalidate_work_region(window_handle, project);
                    return 0;
                }
                case OBJECT_DURATION:
                {
                    if (object->duration.is_pitched)
                    {
                        if (HIBYTE(GetKeyState(VK_SHIFT)))
                        {
                            if (object->duration.pitch.pitch.accidental == DOUBLE_SHARP)
                            {
                                return 0;
                            }
                            ++object->duration.pitch.pitch.accidental;
                        }
                        else
                        {
                            if (object->duration.pitch.pitch.steps_above_c4 < INT8_MAX)
                            {
                                ++object->duration.pitch.pitch.steps_above_c4;
                            }
                            object->duration.pitch.pitch.accidental =
                                get_default_accidental(object, project).accidental;
                        }
                        get_next_slice_right_of_object(object, project)->needs_respacing = true;
                        reset_accidental_displays_from_previous_key_sig(object, project);
                        invalidate_work_region(window_handle, project);
                    }
                }
                }
            }
            }
            return 0;
        }
        }
        break;
    case WM_LBUTTONDOWN:
    {
        struct Project*project = (struct Project*)GetWindowLongPtrW(window_handle, GWLP_USERDATA);
        float zoom_factor = get_zoom_factor(project->zoom_exponent);
        float tz_mouse_x = GET_X_LPARAM(l_param);
        float tz_mouse_y = GET_Y_LPARAM(l_param);
        HDC device_context = GetDC(window_handle);
        HDC back_buffer_device_context = CreateCompatibleDC(device_context);
        ReleaseDC(window_handle, device_context);
        SaveDC(back_buffer_device_context);
        SelectObject(back_buffer_device_context, project->main_window_back_buffer);
        SetBkMode(back_buffer_device_context, TRANSPARENT);
        SetTextAlign(back_buffer_device_context, TA_BASELINE);
        SetTextColor(back_buffer_device_context, WHITE);
        SelectObject(back_buffer_device_context, GetStockObject(WHITE_PEN));
        SelectObject(back_buffer_device_context, GetStockObject(WHITE_BRUSH));
        RECT client_rect;
        GetClientRect(window_handle, &client_rect);
        FillRect(back_buffer_device_context, &client_rect, GetStockObject(BLACK_BRUSH));
        int32_t tuz_staff_middle_y =
            project->utuz_y_of_staff_above_highest_visible - project->uz_viewport_offset.y;
        uint32_t staff_index = project->highest_visible_staff_index;
        while (staff_index)
        {
            struct Staff*staff = resolve_pool_index(&project->staff_pool, staff_index);
            tuz_staff_middle_y += staff->uz_distance_from_staff_above;
            uint32_t clicked_object_address =
                get_address_of_clicked_staff_object(back_buffer_device_context, project, staff,
                    zoom_factor, tuz_staff_middle_y, tz_mouse_x, tz_mouse_y);
            if (clicked_object_address)
            {
                cancel_selection(window_handle);
                struct Object*object = resolve_address(project, clicked_object_address);
                object->is_selected = true;
                if (object_is_header(object))
                {
                    enable_add_header_object_buttons(project, TRUE);
                }
                project->selection.selection_type = SELECTION_OBJECT;
                project->selection.address.staff_index = staff_index;
                project->selection.address.object_address = object->address;
                RestoreDC(back_buffer_device_context, -1);
                ReleaseDC(window_handle, back_buffer_device_context);
                invalidate_work_region(window_handle, project);
                return 0;
            }
            staff_index = staff->index_of_staff_below;
        }
        if (project->ghost_cursor_address.staff_index)
        {
            cancel_selection(window_handle);
            project->selection.selection_type = SELECTION_CURSOR;
            project->selection.address = project->ghost_cursor_address;

            struct ObjectIter iter;
            initialize_page_element_iter(&iter.base,
                resolve_address(project, project->ghost_cursor_address.object_address),
                sizeof(struct Object));
            while (true)
            {
                decrement_page_element_iter(&iter.base, &project->page_pool, sizeof(struct Object));
                switch (iter.object->object_type)
                {
                case OBJECT_CLEF:
                    project->selection.range_floor = get_staff_middle_pitch(&iter.object->clef);
                    goto range_floor_set;
                case OBJECT_DURATION:
                    if (iter.object->duration.is_pitched)
                    {
                        project->selection.range_floor =
                            iter.object->duration.pitch.pitch.steps_above_c4;
                        goto range_floor_set;
                    }
                }
            }
        range_floor_set:
            project->selection.range_floor -= 3;
            enable_add_header_object_buttons(project, TRUE);
            project->ghost_cursor_address.staff_index = 0;
            invalidate_work_region(window_handle, project);
        }
        RestoreDC(back_buffer_device_context, -1);
        ReleaseDC(window_handle, back_buffer_device_context);
        return 0;
    }
    case WM_MOUSEMOVE:
    {
        struct Project*project = (struct Project*)GetWindowLongPtrW(window_handle, GWLP_USERDATA);
        struct StaffObjectAddress ghost_cursor_address =
            get_ghost_cursor_address(project, GET_X_LPARAM(l_param), GET_Y_LPARAM(l_param));
        if (ghost_cursor_address.object_address)
        {
            if (!memcmp(&ghost_cursor_address, &project->ghost_cursor_address,
                sizeof(struct StaffObjectAddress)))
            {
                return 0;
            }
            project->ghost_cursor_address = ghost_cursor_address;
            invalidate_work_region(window_handle, project);
            return 0;
        }
        if (project->ghost_cursor_address.staff_index)
        {
            project->ghost_cursor_address.object_address = 0;
            invalidate_work_region(window_handle, project);
        }
        return 0;
    }
    case WM_MOUSEWHEEL:
    {
        struct Project*project = (struct Project*)GetWindowLongPtrW(window_handle, GWLP_USERDATA);
        if (!project->highest_visible_staff_index)
        {
            return 0;
        }
        int16_t delta = HIWORD(w_param);
        float current_zoom_factor = get_zoom_factor(project->zoom_exponent);
        WORD virtual_key = LOWORD(w_param);
        if (virtual_key == MK_CONTROL)
        {
            if (delta > 0)
            {
                project->zoom_exponent = clamped_subtract(project->zoom_exponent, 1);
            }
            else
            {
                project->zoom_exponent = clamped_add(project->zoom_exponent, 1);
            }
            float new_zoom_factor = get_zoom_factor(project->zoom_exponent);
            float tz_cursor_x = GET_X_LPARAM(l_param);
            reset_viewport_offset_x(window_handle, project, 
                float_round(tz_cursor_x / current_zoom_factor - tz_cursor_x / new_zoom_factor) +
                    project->uz_viewport_offset.x);
            float tz_cursor_y = GET_Y_LPARAM(l_param);
            reset_viewport_offset_y(window_handle, project,
                float_round(tz_cursor_y / current_zoom_factor - tz_cursor_y / new_zoom_factor) +
                    project->uz_viewport_offset.y);
        }
        else
        {
            int32_t uz_shift = unzoom_coordinate(delta, (WHEEL_DELTA_SCALE * current_zoom_factor));
            if (virtual_key == MK_SHIFT)
            {
                reset_viewport_offset_x(window_handle, project,
                    project->uz_viewport_offset.x + uz_shift);
            }
            else
            {
                reset_viewport_offset_y(window_handle, project,
                    project->uz_viewport_offset.y + uz_shift);
            }
        }
        invalidate_work_region(window_handle, project);
        return 0;
    }
    case WM_NOTIFY:
    {
        switch (((LPNMHDR)l_param)->code)
        {
        case TCN_SELCHANGE:
        {
            struct Project*project =
                (struct Project*)GetWindowLongPtrW(window_handle, GWLP_USERDATA);
            switch (SendMessageW(project->control_tabs_handle, TCM_GETCURSEL, 0, 0))
            {
            case STAFF_TAB_INDEX:
                ShowWindow(project->staff_tab_handle, SW_SHOW);
                SendMessageW(project->staff_tab_handle, WM_ENABLE, TRUE, 0);
                break;
            case CLEF_TAB_INDEX:
                ShowWindow(project->clef_tab_handle, SW_SHOW);
                SendMessageW(project->clef_tab_handle, WM_ENABLE, TRUE, 0);
                break;
            case KEY_SIG_TAB_INDEX:
                ShowWindow(project->key_sig_tab_handle, SW_SHOW);
                SendMessageW(project->key_sig_tab_handle, WM_ENABLE, TRUE, 0);
                break;
            case TIME_SIG_TAB_INDEX:
                ShowWindow(project->time_sig_tab_handle, SW_SHOW);
                SendMessageW(project->time_sig_tab_handle, WM_ENABLE, TRUE, 0);
                break;
            case NOTE_TAB_INDEX:
                ShowWindow(project->note_tab_handle, SW_SHOW);
                SendMessageW(project->note_tab_handle, WM_ENABLE, TRUE, 0);
            }
            return 0;
        }
        case TCN_SELCHANGING:
        {
            struct Project*project =
                (struct Project*)GetWindowLongPtrW(window_handle, GWLP_USERDATA);
            switch (SendMessageW(project->control_tabs_handle, TCM_GETCURSEL, 0, 0))
            {
            case STAFF_TAB_INDEX:
                ShowWindow(project->staff_tab_handle, SW_HIDE);
                SendMessageW(project->staff_tab_handle, WM_ENABLE, FALSE, 0);
                break;
            case CLEF_TAB_INDEX:
                ShowWindow(project->clef_tab_handle, SW_HIDE);
                SendMessageW(project->clef_tab_handle, WM_ENABLE, FALSE, 0);
                break;
            case KEY_SIG_TAB_INDEX:
                ShowWindow(project->key_sig_tab_handle, SW_HIDE);
                SendMessageW(project->key_sig_tab_handle, WM_ENABLE, FALSE, 0);
                break;
            case TIME_SIG_TAB_INDEX:
                ShowWindow(project->time_sig_tab_handle, SW_HIDE);
                SendMessageW(project->time_sig_tab_handle, WM_ENABLE, FALSE, 0);
                break;
            case NOTE_TAB_INDEX:
                ShowWindow(project->note_tab_handle, SW_HIDE);
                SendMessageW(project->note_tab_handle, WM_ENABLE, FALSE, 0);
            }
        }
        }
        return 0;
    }
    case WM_PAINT:
    {
        struct Project*project = (struct Project*)GetWindowLongPtrW(window_handle, GWLP_USERDATA);
        if (!project->highest_visible_staff_index)
        {
            break;
        }
        respace_onscreen_slices(window_handle, project);
        float zoom_factor = get_zoom_factor(project->zoom_exponent);
        PAINTSTRUCT paint_struct;
        HDC device_context = BeginPaint(window_handle, &paint_struct);
        HDC back_buffer_device_context = CreateCompatibleDC(device_context);
        SaveDC(back_buffer_device_context);
        SelectObject(back_buffer_device_context, project->main_window_back_buffer);
        SetBkMode(back_buffer_device_context, TRANSPARENT);
        SetTextAlign(back_buffer_device_context, TA_BASELINE);
        SelectObject(back_buffer_device_context, GetStockObject(BLACK_PEN));
        SelectObject(back_buffer_device_context, GetStockObject(BLACK_BRUSH));
        SetTextColor(back_buffer_device_context, BLACK);
        FillRect(back_buffer_device_context, &paint_struct.rcPaint, GetStockObject(WHITE_BRUSH));
        int32_t tuz_staff_middle_y =
            project->utuz_y_of_staff_above_highest_visible - project->uz_viewport_offset.y;
        uint32_t staff_index = project->highest_visible_staff_index;
        struct Staff*staff = resolve_pool_index(&project->staff_pool, staff_index);
        while (true)
        {
            tuz_staff_middle_y += staff->uz_distance_from_staff_above;
            if (zoom_coordinate(get_tuz_y_of_staff_relative_step(tuz_staff_middle_y,
                project->uz_default_staff_space_height *
                    ((struct StaffScale*)resolve_address(project, staff->scale_address))->value,
                staff->line_count, 2 * (staff->line_count - 1)), zoom_factor) >=
                paint_struct.rcPaint.bottom)
            {
                break;
            }
            draw_staff(back_buffer_device_context, project, tuz_staff_middle_y,
                paint_struct.rcPaint.right, staff_index);
            if (!staff->index_of_staff_below)
            {
                break;
            }
            staff_index = staff->index_of_staff_below;
            staff = resolve_pool_index(&project->staff_pool, staff_index);
        }
        BitBlt(device_context, paint_struct.rcPaint.left, paint_struct.rcPaint.top,
            paint_struct.rcPaint.right - paint_struct.rcPaint.left,
            paint_struct.rcPaint.bottom - paint_struct.rcPaint.top,
            back_buffer_device_context, paint_struct.rcPaint.left, paint_struct.rcPaint.top,
            SRCCOPY);
        RestoreDC(back_buffer_device_context, -1);
        EndPaint(window_handle, &paint_struct);
        break;
    }
    case WM_SIZE:
    {
        struct Project*project = (struct Project*)GetWindowLongPtrW(window_handle, GWLP_USERDATA);
        if (project)
        {
            RECT client_rect;
            GetClientRect(window_handle, &client_rect);
            int32_t width = client_rect.right - client_rect.left;
            HDC device_context = GetDC(window_handle);
            DeleteObject(project->main_window_back_buffer);
            project->main_window_back_buffer = CreateCompatibleBitmap(device_context, width,
                client_rect.bottom - client_rect.top);
            ReleaseDC(window_handle, device_context);
            SetWindowPos(project->control_tabs_handle, 0, client_rect.left, 0, width, 70, 0);
            InvalidateRect(window_handle, &client_rect, FALSE);
        }
        return 0;
    }
    }
    return DefWindowProcW(window_handle, message, w_param, l_param);
}