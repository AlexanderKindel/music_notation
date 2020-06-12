#include "declarations.h"
#include "edit_staff_scales_dialog/edit_staff_scales_dialog.c"

size_t staff_scale_to_string(struct StaffScale*scale, wchar_t*out)
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
    return destination - out + sizeof(x_default_string) / sizeof(wchar_t);
}

int32_t populate_staff_scale_list(HDC device_context, HWND scale_list_handle,
    struct Project*project, size_t starting_index)
{
    struct StaffScaleIter scale_iter;
    initialize_page_element_iter(&scale_iter.base, project->staff_scales->bytes,
        sizeof(struct StaffScale));
    increment_page_element_iter(&scale_iter.base, &project->page_pool, sizeof(struct StaffScale));
    int32_t max_item_width = 0;
    while (scale_iter.scale)
    {
        wchar_t scale_string[43];
        int32_t item_width = get_text_width(device_context, scale_string,
            staff_scale_to_string(scale_iter.scale, scale_string));
        max_item_width = MAX(max_item_width, item_width);
        SendMessageW(scale_list_handle, CB_ADDSTRING, 0, (LPARAM)scale_string);
        SendMessageW(scale_list_handle, CB_SETITEMDATA, starting_index, scale_iter.scale->address);
        increment_page_element_iter(&scale_iter.base, &project->page_pool,
            sizeof(struct StaffScale));
        ++starting_index;
    }
    SendMessageW(scale_list_handle, CB_SETCURSEL, 0, 0);
    return max_item_width;
}

void format_add_staff_dialog(HWND dialog_handle)
{
    HWND scale_list_handle = GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST);
    HDC device_context = GetDC(dialog_handle);
    HFONT old_font = (HFONT)SelectObject(device_context, TEXT_FONT);
    wchar_t default_string[] = L"Default";
    SendMessageW(scale_list_handle, CB_ADDSTRING, 0, (LPARAM)default_string);
    struct Project*project = (struct Project*)GetWindowLongPtrW(dialog_handle, DWLP_USER);
    SendMessageW(scale_list_handle, CB_SETITEMDATA, 0,
        ((struct StaffScale*)project->staff_scales->bytes)->address);
    int32_t default_string_width = GET_TEXT_WIDTH(device_context, default_string);
    int32_t scale_list_width =
        populate_staff_scale_list(device_context, scale_list_handle, project, 1);
    scale_list_width = MAX(default_string_width, scale_list_width);
    SendMessageW(scale_list_handle, CB_SETCURSEL, 0, 0);
    int32_t edit_scales_width =
        GET_TEXT_WIDTH(device_context, EDIT_SCALES_STRING) + TEXT_CONTROL_X_BUFFER;
    scale_list_width = MAX(scale_list_width, edit_scales_width);
    int32_t scale_label_width = GET_TEXT_WIDTH(device_context, ADD_STAFF_SCALE_LABEL_STRING);
    scale_list_width = MAX(scale_list_width, scale_label_width) + GetSystemMetrics(SM_CXVSCROLL);
    int32_t y = UNRELATED_CONTROL_SPACER;
    SetWindowPos(GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LABEL), 0, UNRELATED_CONTROL_SPACER,
        y, scale_label_width, TEXT_FONT_HEIGHT, SWP_NOZORDER);
    int32_t line_count_controls_x = scale_list_width + 2 * UNRELATED_CONTROL_SPACER;
    int32_t line_count_label_width =
        GET_TEXT_WIDTH(device_context, ADD_STAFF_LINE_COUNT_LABEL_STRING);
    SelectObject(device_context, old_font);
    ReleaseDC(dialog_handle, device_context);
    SetWindowPos(GetDlgItem(dialog_handle, IDC_ADD_STAFF_LINE_COUNT_LABEL), 0,
        line_count_controls_x, y, line_count_label_width, TEXT_FONT_HEIGHT, SWP_NOZORDER);
    y += TEXT_FONT_HEIGHT + LABEL_SPACER;
    SetWindowPos(scale_list_handle, 0, UNRELATED_CONTROL_SPACER, y, scale_list_width,
        7 * ComboBox_GetItemHeight(scale_list_handle), SWP_NOZORDER);
    HWND line_count_display_handle = GetDlgItem(dialog_handle, IDC_ADD_STAFF_LINE_COUNT_DISPLAY);
    SetWindowPos(line_count_display_handle, 0, line_count_controls_x, y, COMMAND_BUTTON_WIDTH,
        BUTTON_HEIGHT, SWP_NOZORDER);
    RECT line_count_display_rect;
    GetClientRect(line_count_display_handle, &line_count_display_rect);
    MapWindowPoints(line_count_display_handle, dialog_handle, (POINT*)&line_count_display_rect, 2);
    SetWindowPos(GetDlgItem(dialog_handle, IDC_ADD_STAFF_LINE_COUNT_SPIN), 0,
        line_count_display_rect.right - UNRELATED_CONTROL_SPACER, line_count_display_rect.top,
        UNRELATED_CONTROL_SPACER, line_count_display_rect.bottom - line_count_display_rect.top,
        SWP_NOZORDER);
    y += BUTTON_HEIGHT + RELATED_CONTROL_SPACER;
    SetWindowPos(GetDlgItem(dialog_handle, IDC_ADD_STAFF_EDIT_SCALES), 0, UNRELATED_CONTROL_SPACER,
        y, scale_list_width, BUTTON_HEIGHT, SWP_NOZORDER);
    y += BUTTON_HEIGHT + UNRELATED_CONTROL_SPACER;
    int32_t window_width = line_count_controls_x + COMMAND_BUTTON_WIDTH + UNRELATED_CONTROL_SPACER;
    SetWindowPos(GetDlgItem(dialog_handle, IDOK), 0,
        scale_list_width + UNRELATED_CONTROL_SPACER - COMMAND_BUTTON_WIDTH, y, COMMAND_BUTTON_WIDTH,
        BUTTON_HEIGHT, SWP_NOZORDER);
    SetWindowPos(GetDlgItem(dialog_handle, IDCANCEL), 0, line_count_controls_x, y,
        COMMAND_BUTTON_WIDTH, BUTTON_HEIGHT, SWP_NOZORDER);
    center_dialog(dialog_handle, window_width, y + BUTTON_HEIGHT + UNRELATED_CONTROL_SPACER);
}

//DWLP_USER is set to a pointer to a struct Project instance.
INT_PTR add_staff_dialog_proc(HWND dialog_handle, UINT message, WPARAM w_param, LPARAM l_param)
{
    switch (message)
    {
    case WM_COMMAND:
        switch (LOWORD(w_param))
        {
        case IDC_ADD_STAFF_EDIT_SCALES:
        {
            struct Project*project = (struct Project*)GetWindowLongPtrW(dialog_handle, DWLP_USER);
            DialogBoxIndirectParamW(0, &EDIT_STAFF_SCALES_DIALOG_TEMPLATE.header, dialog_handle,
                edit_staff_scales_dialog_proc, (LPARAM)project);
            HWND scale_list_handle = GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST);
            SendMessageW(scale_list_handle, CB_RESETCONTENT, 0, 0);
            format_add_staff_dialog(dialog_handle);
            return TRUE;
        }
        case IDCANCEL:
            EndDialog(dialog_handle, 0);
            return TRUE;
        case IDOK:
        {
            struct Project*project = (struct Project*)GetWindowLongPtrW(dialog_handle, DWLP_USER);
            struct Staff*staff = allocate_pool_slot(&project->staff_pool);
            if (project->topmost_staff_index)
            {
                staff->uz_distance_from_staff_above = 80;
                project->utuz_bottom_staff_y += staff->uz_distance_from_staff_above;
                struct Staff*old_bottommost_staff =
                    resolve_pool_index(&project->staff_pool, project->bottommost_staff_index);
                old_bottommost_staff->index_of_staff_below =
                    get_element_index_in_pool(&project->staff_pool, staff);
                staff->index_of_staff_above = project->bottommost_staff_index;
                project->bottommost_staff_index = old_bottommost_staff->index_of_staff_below;
            }
            else
            {
                staff->uz_distance_from_staff_above = DEFAULT_TOP_STAFF_MIDDLE_Y;
                staff->index_of_staff_above = 0;
                project->topmost_staff_index =
                    get_element_index_in_pool(&project->staff_pool, staff);
                project->bottommost_staff_index = project->topmost_staff_index;
                project->highest_visible_staff_index = project->topmost_staff_index;
            }
            staff->index_of_staff_below = 0;
            struct Page*object_page = allocate_pool_slot(&project->page_pool);
            object_page->capacity = (PAGE_SIZE - sizeof(struct Page)) / sizeof(struct Object);
            staff->object_page_index = get_element_index_in_pool(&project->page_pool, object_page);
            staff->line_count = SendMessageW(
                GetDlgItem(dialog_handle, IDC_ADD_STAFF_LINE_COUNT_SPIN), UDM_GETPOS32, 0, 0);
            HWND scale_list_handle = GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST);
            staff->scale_address = SendMessageW(scale_list_handle, CB_GETITEMDATA,
                SendMessageW(scale_list_handle, CB_GETCURSEL, 0, 0), 0);
            staff->is_on_free_list = false;
            struct ObjectIter object_iter;
            initialize_page_element_iter(&object_iter.base, object_page->bytes, PAGE_SIZE);
            struct SliceIter slice_iter;
            initialize_page_element_iter(&slice_iter.base,
                resolve_address(project, STAFF_START_SLICE_ADDRESS), PAGE_SIZE);
            insert_slice_object_before_iter(&object_iter, project, slice_iter.slice,
                project->bottommost_staff_index);
            object_iter.object->object_type = OBJECT_NONE;
            object_iter.object->is_selected = false;
            object_iter.object->is_valid_cursor_position = false;
            increment_page_element_iter(&object_iter.base, &project->page_pool,
                sizeof(struct Object));
            increment_page_element_iter(&slice_iter.base, &project->page_pool,
                sizeof(struct Slice));
            insert_slice_object_before_iter(&object_iter, project, slice_iter.slice,
                project->bottommost_staff_index);
            staff->address_of_clef_beyond_leftmost_slice_to_draw = object_iter.object->address;
            object_iter.object->clef = get_selected_clef(project);
            object_iter.object->object_type = OBJECT_CLEF;
            object_iter.object->is_selected = false;
            object_iter.object->is_valid_cursor_position = false;
            increment_page_element_iter(&object_iter.base, &project->page_pool,
                sizeof(struct Object));
            increment_page_element_iter(&slice_iter.base, &project->page_pool,
                sizeof(struct Slice));
            insert_slice_object_before_iter(&object_iter, project, slice_iter.slice,
                project->bottommost_staff_index);
            object_iter.object->key_sig.accidental_count =
                SendMessageW(project->accidental_count_spin_handle, UDM_GETPOS32, 0, 0);
            get_key_sig(&object_iter.object->key_sig, project);
            object_iter.object->object_type = OBJECT_KEY_SIG;
            object_iter.object->is_selected = false;
            object_iter.object->is_valid_cursor_position = false;
            increment_page_element_iter(&object_iter.base, &project->page_pool,
                sizeof(struct Object));
            increment_page_element_iter(&slice_iter.base, &project->page_pool,
                sizeof(struct Slice));
            insert_slice_object_before_iter(&object_iter, project, slice_iter.slice,
                project->bottommost_staff_index);
            get_selected_time_sig(project, &object_iter.object->time_sig);
            object_iter.object->object_type = OBJECT_TIME_SIG;
            object_iter.object->is_selected = false;
            object_iter.object->is_valid_cursor_position = false;
            increment_page_element_iter(&object_iter.base, &project->page_pool,
                sizeof(struct Object));
            increment_page_element_iter(&slice_iter.base, &project->page_pool,
                sizeof(struct Slice));
            insert_slice_object_before_iter(&object_iter, project, slice_iter.slice,
                project->bottommost_staff_index);
            object_iter.object->object_type = OBJECT_NONE;
            object_iter.object->is_selected = false;
            object_iter.object->is_valid_cursor_position = true;
            invalidate_work_region(GetWindow(dialog_handle, GW_OWNER), project);
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
        SetWindowLongPtrW(dialog_handle, DWLP_USER, l_param);
        SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_STAFF_EDIT_SCALES), WM_SETFONT,
            (WPARAM)TEXT_FONT, 0);
        SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LABEL), WM_SETFONT,
            (WPARAM)TEXT_FONT, 0);
        SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_STAFF_SCALE_LIST), WM_SETFONT,
            (WPARAM)TEXT_FONT, 0);
        SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_STAFF_LINE_COUNT_LABEL), WM_SETFONT,
            (WPARAM)TEXT_FONT, 0);
        SendMessageW(GetDlgItem(dialog_handle, IDC_ADD_STAFF_LINE_COUNT_DISPLAY), WM_SETFONT,
            (WPARAM)TEXT_FONT, 0);
        SendMessageW(GetDlgItem(dialog_handle, IDOK), WM_SETFONT, (WPARAM)TEXT_FONT, 0);
        SendMessageW(GetDlgItem(dialog_handle, IDCANCEL), WM_SETFONT, (WPARAM)TEXT_FONT, 0);
        HWND line_count_spin_handle = GetDlgItem(dialog_handle, IDC_ADD_STAFF_LINE_COUNT_SPIN);
        SendMessageW(line_count_spin_handle, UDM_SETRANGE32, 1, 5);
        SendMessageW(line_count_spin_handle, UDM_SETPOS32, 0, 5);
        format_add_staff_dialog(dialog_handle);
        return TRUE;
    }
    }
    return FALSE;
}

LRESULT staff_tab_proc(HWND window_handle, UINT message, WPARAM w_param, LPARAM l_param,
    UINT_PTR id_subclass, DWORD_PTR ref_data)
{
    if (message == WM_COMMAND && HIWORD(w_param) == BN_CLICKED)
    {
        HWND main_window_handle = GetParent(GetParent(window_handle));
        struct Project*project =
            (struct Project*)GetWindowLongPtrW(main_window_handle, GWLP_USERDATA);
        SetFocus(main_window_handle);
        if (l_param == (LPARAM)project->add_staff_button_handle)
        {
            DialogBoxIndirectParamW(0, &ADD_STAFF_DIALOG_TEMPLATE.header, main_window_handle,
                add_staff_dialog_proc, (LPARAM)project);
        }
        else if (l_param == (LPARAM)project->edit_staff_scales_button_handle)
        {
            DialogBoxIndirectParamW(0, &EDIT_STAFF_SCALES_DIALOG_TEMPLATE.header,
                main_window_handle, edit_staff_scales_dialog_proc, (LPARAM)project);
        }
        return 0;
    }
    return DefWindowProcW(window_handle, message, w_param, l_param);
}