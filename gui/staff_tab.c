#include "declarations.h"

INT_PTR edit_staff_scale_dialog_proc(HWND dialog_handle, UINT message, WPARAM w_param,
    LPARAM l_param)
{
    switch (message)
    {
    case WM_COMMAND:
        switch (LOWORD(w_param))
        {
        case IDCANCEL:
            EndDialog(dialog_handle, 0);
            return TRUE;
        case IDOK:
        {
            wchar_t value_string[16];
            SendMessageW(GetDlgItem(dialog_handle, IDC_EDIT_STAFF_SCALE_VALUE), WM_GETTEXT, 16,
                (LPARAM)value_string);
            float new_value = wcstof(value_string, 0);
            if (new_value > 0.0)
            {
                struct StaffScale*scale =
                    (struct StaffScale*)GetWindowLongPtrW(dialog_handle, DWLP_USER);
                memcpy(scale->value_string, value_string, sizeof(value_string));
                scale->value = new_value;
                SendMessageW(GetDlgItem(dialog_handle, IDC_EDIT_STAFF_SCALE_NAME), WM_GETTEXT, 16,
                    (LPARAM)scale->name);
                EndDialog(dialog_handle, 0);
                return TRUE;
            }
            MessageBoxW(dialog_handle, L"The value must be a positive decimal number.", 0, MB_OK);
            return TRUE;
        }
        }
        return FALSE;
    case WM_INITDIALOG:
        size_dialog(dialog_handle);
        SetWindowLongPtrW(dialog_handle, DWLP_USER, l_param);
        SendMessageW(GetDlgItem(dialog_handle, IDC_EDIT_STAFF_SCALE_NAME), WM_SETTEXT, 0,
            (LPARAM)((struct StaffScale*)l_param)->name);
        SendMessageW(GetDlgItem(dialog_handle, IDC_EDIT_STAFF_SCALE_VALUE), WM_SETTEXT, 0,
            (LPARAM)((struct StaffScale*)l_param)->value_string);
        return TRUE;
    }
    return FALSE;
}

INT_PTR remap_staff_scale_dialog_proc(HWND dialog_handle, UINT message, WPARAM w_param,
    LPARAM l_param)
{
    switch (message)
    {
    case WM_COMMAND:
        switch (LOWORD(w_param))
        {
        case IDCANCEL:
            EndDialog(dialog_handle, -1);
            return TRUE;
        case IDOK:
            EndDialog(dialog_handle, 0);
            return TRUE;
        default:
            return FALSE;
        }
    case WM_INITDIALOG:
    {
        size_dialog(dialog_handle);
        HWND scale_list_handle = GetDlgItem(dialog_handle, IDC_REMAP_STAFF_SCALE_LIST);
        struct StaffScale*staff_scales = (struct StaffScale*)l_param;
        for (uint_fast8_t i = 0; i < MAX_STAFF_SCALE_COUNT - 1; ++i)
        {
            if (!staff_scales[i].name[0])
            {
                break;
            }
            SendMessageW(scale_list_handle, CB_ADDSTRING, 0, (LPARAM)staff_scales[i].name);
        }
        SendMessageW(scale_list_handle, CB_SETCURSEL, 0, 0);
        return TRUE;
    }
    default:
        return FALSE;
    }
}

LRESULT insert_staff_scale(struct StaffScale*staff_scales, struct StaffScale*scale)
{
    LRESULT scale_index = 1;
    while (true)
    {
        if (!staff_scales[scale_index].name[0])
        {
            break;
        }
        if (scale->value > staff_scales[scale_index].value)
        {
            memmove(staff_scales + scale_index + 1, staff_scales + scale_index,
                sizeof(struct StaffScale) * (MAX_STAFF_SCALE_COUNT - scale_index - 1));
            break;
        }
        ++scale_index;
    }
    memcpy(staff_scales + scale_index, scale, sizeof(struct StaffScale));
    return scale_index;
}

void remove_staff_scale(size_t removal_index, struct StaffScale*staff_scales)
{
    memmove(staff_scales + removal_index, staff_scales + removal_index + 1,
        sizeof(struct StaffScale) * (MAX_STAFF_SCALE_COUNT - removal_index - 1));
    staff_scales[MAX_STAFF_SCALE_COUNT - 1].name[0] = 0;
}

void staff_scale_to_string(struct StaffScale*scale, wchar_t*out)
{
    wchar_t*source = scale->name;
    wchar_t*destination = out;
    while (*source)
    {
        *destination = *source;
        ++destination;
        ++source;
    }
    *destination = L':';
    ++destination;
    *destination = L' ';
    ++destination;
    source = scale->value_string;
    while (*source)
    {
        *destination = *source;
        ++destination;
        ++source;
    }
    wchar_t x_default_string[] = L" X default";
    memcpy(destination, x_default_string, sizeof(x_default_string));
}

INT_PTR add_staff_dialog_proc(HWND dialog_handle, UINT message, WPARAM w_param, LPARAM l_param)
{
    switch (message)
    {
    case WM_COMMAND:
        switch (LOWORD(w_param))
        {
        case IDC_ADD_STAFF_ADD_SCALE:
        {
            struct StaffScale*staff_scales =
                ((struct Project*)GetWindowLongPtrW(dialog_handle, DWLP_USER))->staff_scales;
            struct StaffScale new_scale = { 1.0, L"1.0", L"New" };
            size_t new_scale_index = insert_staff_scale(staff_scales, &new_scale);
            HWND scale_list_handle = GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST);
            wchar_t new_scale_string[43];
            staff_scale_to_string(&new_scale, new_scale_string);
            SendMessageW(scale_list_handle, CB_INSERTSTRING, new_scale_index,
                (LPARAM)new_scale_string);
            SendMessageW(scale_list_handle, CB_SETCURSEL, new_scale_index, 0);
            EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_STAFF_EDIT_SCALE), TRUE);
            EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_STAFF_REMOVE_SCALE), TRUE);
            return TRUE;
        }
        case IDC_ADD_STAFF_EDIT_SCALE:
        {
            struct Project*project = (struct Project*)GetWindowLongPtrW(dialog_handle, DWLP_USER);
            LRESULT scale_index = SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST),
                CB_GETCURSEL, 0, 0);
            DialogBoxIndirectParamW(0, EDIT_STAFF_SCALE_DIALOG_TEMPLATE, dialog_handle,
                edit_staff_scale_dialog_proc, (LPARAM)(project->staff_scales + scale_index));
            struct StaffScale edited_scale = project->staff_scales[scale_index];
            remove_staff_scale(scale_index, project->staff_scales);
            LRESULT edited_scale_index = insert_staff_scale(project->staff_scales, &edited_scale);
            HWND scale_list_handle = GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST);
            SendMessageW(scale_list_handle, CB_DELETESTRING, scale_index, 0);
            wchar_t edited_scale_string[43];
            staff_scale_to_string(&edited_scale, edited_scale_string);
            SendMessageW(scale_list_handle, CB_INSERTSTRING, edited_scale_index,
                (LPARAM)edited_scale_string);
            SendMessageW(scale_list_handle, CB_SETCURSEL, edited_scale_index, 0);
            if (scale_index == edited_scale_index)
            {
                return TRUE;
            }
            LRESULT increment;
            LRESULT min_index;
            LRESULT max_index;
            if (scale_index < edited_scale_index)
            {
                increment = -1;
                min_index = scale_index;
                max_index = edited_scale_index;
            }
            else
            {
                increment = 1;
                min_index = edited_scale_index;
                max_index = scale_index;
            }
            for (struct Staff*staff = resolve_pool_index(&STAFF_POOL(project), 1);
                staff < STAFF_POOL(project).cursor; ++staff)
            {
                if (staff->scale_index == scale_index)
                {
                    staff->scale_index = edited_scale_index;
                }
                else if (min_index <= staff->scale_index && staff->scale_index <= max_index)
                {
                    staff->scale_index = (LRESULT)staff->scale_index + increment;
                }
            }
            return TRUE;
        }
        case IDC_ADD_STAFF_REMOVE_SCALE:
        {
            HWND scale_list_handle = GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST);
            LRESULT removal_index = SendMessageW(scale_list_handle, CB_GETCURSEL, 0, 0);
            LRESULT remapped_index;
            struct Project*project = (struct Project*)GetWindowLongPtrW(dialog_handle, DWLP_USER);
            struct Staff*staff = resolve_pool_index(&STAFF_POOL(project), 1);
            while (true)
            {
                if (staff == STAFF_POOL(project).cursor)
                {
                    remapped_index = 0;
                    break;
                }
                if (!staff->is_on_free_list && staff->scale_index == removal_index)
                {
                    wchar_t reassignment_candidates[43][MAX_STAFF_SCALE_COUNT - 1];
                    for (size_t scale_index = 0; scale_index < removal_index; ++scale_index)
                    {
                        SendMessageW(scale_list_handle, CB_GETLBTEXT, scale_index,
                            (LPARAM)(reassignment_candidates + scale_index));
                        reassignment_candidates[scale_index]
                            [SendMessageW(scale_list_handle, CB_GETLBTEXTLEN, scale_index, 0)] = 0;
                    }
                    for (size_t scale_index = removal_index + 1;
                        scale_index < MAX_STAFF_SCALE_COUNT; ++scale_index)
                    {
                        size_t candidate_index = scale_index - 1;
                        SendMessageW(scale_list_handle, CB_GETLBTEXT, scale_index,
                            (LPARAM)(reassignment_candidates + candidate_index));
                        reassignment_candidates[candidate_index]
                            [SendMessageW(scale_list_handle, CB_GETLBTEXTLEN, scale_index, 0)] = 0;
                        if (!reassignment_candidates[candidate_index][0])
                        {
                            break;
                        }
                    }
                    remapped_index = DialogBoxIndirectParamW(0, REMAP_STAFF_SCALE_DIALOG_TEMPLATE,
                        dialog_handle, remap_staff_scale_dialog_proc,
                        (LPARAM)reassignment_candidates);
                    if (remapped_index < 0)
                    {
                        return TRUE;
                    }
                    ++staff;
                    break;
                }
                ++staff;
            }
            remove_staff_scale(removal_index, project->staff_scales);
            while (staff < STAFF_POOL(project).cursor)
            {
                if (staff->scale_index == removal_index)
                {
                    staff->scale_index = remapped_index;
                }
                else if (staff->scale_index > removal_index)
                {
                    --staff->scale_index;
                }
                ++staff;
            }
            SendMessageW(scale_list_handle, CB_DELETESTRING, removal_index, 0);
            SendMessageW(scale_list_handle, CB_SETCURSEL, remapped_index, 0);
            EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_STAFF_ADD_SCALE), true);
            return TRUE;
        }
        case IDC_ADD_STAFF_SCALE_LIST:
            if (HIWORD(w_param) == CBN_SELCHANGE)
            {
                bool enable_editing =
                    SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST), CB_GETCURSEL,
                        0, 0) > 0;
                EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_STAFF_EDIT_SCALE), enable_editing);
                EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_STAFF_REMOVE_SCALE), enable_editing);
            }
            return TRUE;
        case IDCANCEL:
            EndDialog(dialog_handle, 0);
            return TRUE;
        case IDOK:
        {
            struct Project*project = (struct Project*)GetWindowLongPtrW(dialog_handle, DWLP_USER);
            struct Staff*staff = allocate_pool_slot(&STAFF_POOL(project));
            if (project->topmost_staff_index)
            {
                staff->uz_distance_from_staff_above = 80;
                project->utuz_bottom_staff_y += staff->uz_distance_from_staff_above;
                struct Staff*old_bottommost_staff =
                    resolve_pool_index(&STAFF_POOL(project), project->bottommost_staff_index);
                old_bottommost_staff->index_of_staff_below =
                    get_element_index_in_pool(&STAFF_POOL(project), staff);
                staff->index_of_staff_above = project->bottommost_staff_index;
                project->bottommost_staff_index = old_bottommost_staff->index_of_staff_below;
            }
            else
            {
                staff->uz_distance_from_staff_above = DEFAULT_TOP_STAFF_MIDDLE_Y;
                staff->index_of_staff_above = 0;
                project->topmost_staff_index =
                    get_element_index_in_pool(&STAFF_POOL(project), staff);
                project->bottommost_staff_index = project->topmost_staff_index;
                project->highest_visible_staff_index = project->topmost_staff_index;
            }
            staff->index_of_staff_below = 0;
            struct Page*object_page = allocate_pool_slot(&project->page_pool);
            object_page->capacity = (PAGE_SIZE - sizeof(struct Page)) / sizeof(struct Object);
            staff->object_page_index = get_element_index_in_pool(&project->page_pool, object_page);
            staff->line_count = SendMessageW(
                GetDlgItem(dialog_handle, IDC_ADD_STAFF_LINE_COUNT_SPIN), UDM_GETPOS32, 0, 0);
            staff->scale_index = SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST),
                CB_GETCURSEL, 0, 0);
            staff->is_on_free_list = false;
            struct ObjectIter iter;
            initialize_page_element_iter(&iter.base, object_page->bytes, PAGE_SIZE);
            insert_slice_object_before_iter(&iter, project, STAFF_START_SLICE_ADDRESS,
                project->bottommost_staff_index);
            iter.object->object_type = OBJECT_NONE;
            iter.object->is_selected = false;
            iter.object->is_valid_cursor_position = false;
            increment_page_element_iter(&iter.base, &project->page_pool, sizeof(struct Object));
            insert_slice_object_before_iter(&iter, project, HEADER_CLEF_SLICE_ADDRESS,
                project->bottommost_staff_index);
            staff->address_of_clef_beyond_leftmost_slice_to_draw = iter.object->address;
            iter.object->clef = get_selected_clef(project);
            iter.object->object_type = OBJECT_CLEF;
            iter.object->is_selected = false;
            iter.object->is_valid_cursor_position = false;
            increment_page_element_iter(&iter.base, &project->page_pool, sizeof(struct Object));
            uint8_t accidental_count =
                SendMessageW(project->accidental_count_spin_handle, UDM_GETPOS32, 0, 0);
            if (accidental_count)
            {
                insert_slice_object_before_iter(&iter, project, HEADER_KEY_SIG_SLICE_ADDRESS,
                    project->bottommost_staff_index);
                iter.object->key_sig.accidental_count = accidental_count;
                get_key_sig(&iter.object->key_sig,
                    SendMessageW(project->flats_handle, BM_GETCHECK, 0, 0) == BST_CHECKED);
                iter.object->object_type = OBJECT_KEY_SIG;
                iter.object->is_selected = false;
                iter.object->is_valid_cursor_position = false;
                increment_page_element_iter(&iter.base, &project->page_pool, sizeof(struct Object));
            }
            insert_slice_object_before_iter(&iter, project, HEADER_TIME_SIG_SLICE_ADDRESS,
                project->bottommost_staff_index);
            get_selected_time_sig(project, &iter.object->time_sig);
            iter.object->object_type = OBJECT_TIME_SIG;
            iter.object->is_selected = false;
            iter.object->is_valid_cursor_position = false;
            increment_page_element_iter(&iter.base, &project->page_pool, sizeof(struct Object));
            insert_slice_object_before_iter(&iter, project, BODY_START_SLICE_ADDRESS,
                project->bottommost_staff_index);
            iter.object->object_type = OBJECT_NONE;
            iter.object->is_selected = false;
            iter.object->is_valid_cursor_position = true;
            invalidate_work_region(GetWindow(dialog_handle, GW_OWNER));
            EndDialog(dialog_handle, 0);
            return TRUE;
        }
        }
        return FALSE;
    case WM_CTLCOLORSTATIC:
        if (l_param == (LPARAM)GetDlgItem(dialog_handle, IDC_ADD_STAFF_LINE_COUNT_DISPLAY))
        {
            return (INT_PTR)GetStockObject(WHITE_BRUSH);
        }
        return FALSE;
    case WM_INITDIALOG:
    {
        size_dialog(dialog_handle);
        HWND line_count_spin_handle = GetDlgItem(dialog_handle, IDC_ADD_STAFF_LINE_COUNT_SPIN);
        SendMessageW(line_count_spin_handle, UDM_SETRANGE32, 1, 5);
        SendMessageW(line_count_spin_handle, UDM_SETPOS32, 0, 5);
        HWND scale_list_handle = GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST);
        SendMessageW(scale_list_handle, CB_ADDSTRING, 0, (LPARAM)L"Default");
        SetWindowLongPtrW(dialog_handle, DWLP_USER, l_param);
        for (uint_fast8_t i = 1; i < MAX_STAFF_SCALE_COUNT; ++i)
        {
            if (((struct Project*)GetWindowLongPtrW(dialog_handle, DWLP_USER))->
                staff_scales[i].name[0])
            {
                wchar_t scale_string[43];
                staff_scale_to_string(
                    ((struct Project*)GetWindowLongPtrW(dialog_handle, DWLP_USER))->
                    staff_scales + i,
                    scale_string);
                SendMessageW(scale_list_handle, CB_ADDSTRING, 0, (LPARAM)scale_string);
            }
            else
            {
                break;
            }
        }
        SendMessageW(scale_list_handle, CB_SETCURSEL, 0, 0);
        EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_STAFF_EDIT_SCALE), FALSE);
        EnableWindow(GetDlgItem(dialog_handle, IDC_ADD_STAFF_REMOVE_SCALE), FALSE);
        return TRUE;
    }
    }
    return FALSE;
}

LRESULT staff_tab_proc(HWND window_handle, UINT message, WPARAM w_param, LPARAM l_param,
    UINT_PTR id_subclass, DWORD_PTR ref_data)
{
    if (message == WM_COMMAND)
    {
        if (HIWORD(w_param) == BN_CLICKED)
        {
            HWND main_window_handle = GetParent(GetParent(window_handle));
            SetFocus(main_window_handle);
            struct Project*project =
                (struct Project*)GetWindowLongPtrW(main_window_handle, GWLP_USERDATA);
            if (l_param == (LPARAM)project->add_staff_button_handle)
            {
                DialogBoxIndirectParamW(0, ADD_STAFF_DIALOG_TEMPLATE, main_window_handle,
                    add_staff_dialog_proc, (LPARAM)project);
                return 0;
            }
        }
    }
    return DefWindowProcW(window_handle, message, w_param, l_param);
}