#include "declarations.h"

//DWLP_USER is set to a pointer to the struct StaffScale instance being edited.
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
            }
            else
            {
                MessageBoxW(dialog_handle, L"The value must be a positive decimal number.", 0,
                    MB_OK);
            }
            return TRUE;
        }
        }
        return FALSE;
    case WM_INITDIALOG:
    {
        SetWindowLongPtrW(dialog_handle, DWLP_USER, l_param);
        HWND ok_handle = GetDlgItem(dialog_handle, IDOK);
        HWND name_label_handle = GetDlgItem(dialog_handle, IDC_EDIT_STAFF_SCALE_NAME_LABEL);
        HWND name_edit_handle = GetDlgItem(dialog_handle, IDC_EDIT_STAFF_SCALE_NAME);
        HWND value_label_handle = GetDlgItem(dialog_handle, IDC_EDIT_STAFF_SCALE_VALUE_LABEL);
        HWND value_edit_handle = GetDlgItem(dialog_handle, IDC_EDIT_STAFF_SCALE_VALUE);
        SendMessageW(ok_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
        SendMessageW(name_label_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
        SendMessageW(name_edit_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
        SendMessageW(value_label_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
        SendMessageW(value_edit_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
        SendMessageW(name_edit_handle, WM_SETTEXT, 0,
            (LPARAM)((struct StaffScale*)l_param)->name);
        SendMessageW(value_edit_handle, WM_SETTEXT, 0,
            (LPARAM)((struct StaffScale*)l_param)->value_string);
        HDC device_context = GetDC(dialog_handle);
        HFONT old_font = (HFONT)SelectObject(device_context, TEXT_FONT);
        int32_t edit_width = 2 * COMMAND_BUTTON_WIDTH;
        int32_t y = UNRELATED_CONTROL_SPACER;
        SetWindowPos(name_label_handle, 0, UNRELATED_CONTROL_SPACER, y,
            GET_TEXT_WIDTH(device_context, EDIT_STAFF_NAME_LABEL_STRING), TEXT_FONT_HEIGHT,
            SWP_NOZORDER);
        y += TEXT_FONT_HEIGHT + LABEL_SPACER;
        SetWindowPos(name_edit_handle, 0, UNRELATED_CONTROL_SPACER, y, edit_width, BUTTON_HEIGHT,
            SWP_NOZORDER);
        y += BUTTON_HEIGHT + RELATED_CONTROL_SPACER;
        SetWindowPos(value_label_handle, 0, UNRELATED_CONTROL_SPACER, y,
            GET_TEXT_WIDTH(device_context, EDIT_STAFF_VALUE_LABEL_STRING), TEXT_FONT_HEIGHT,
            SWP_NOZORDER);
        SelectObject(device_context, old_font);
        ReleaseDC(dialog_handle, device_context);
        y += TEXT_FONT_HEIGHT + LABEL_SPACER;
        SetWindowPos(value_edit_handle, 0, UNRELATED_CONTROL_SPACER, y, edit_width, BUTTON_HEIGHT,
            SWP_NOZORDER);
        y += BUTTON_HEIGHT + UNRELATED_CONTROL_SPACER;
        int32_t window_width = 2 * UNRELATED_CONTROL_SPACER + edit_width;
        SetWindowPos(ok_handle, 0, (window_width - COMMAND_BUTTON_WIDTH) / 2, y,
            COMMAND_BUTTON_WIDTH, BUTTON_HEIGHT, SWP_NOZORDER);
        center_dialog(dialog_handle, window_width, y + BUTTON_HEIGHT + UNRELATED_CONTROL_SPACER);
        return TRUE;
    }
    }
    return FALSE;
}

struct RemapScaleInfo
{
    HWND edit_scales_list_handle;
    struct Project*project;
    uint32_t address_of_scale_to_remap;
};

//DWLP_USER is set to a pointer to a struct RemapScaleInfo instance.
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
        {
            HWND scale_list_handle = GetDlgItem(dialog_handle, IDC_REMAP_STAFF_SCALE_LIST);
            EndDialog(dialog_handle,
                SendMessageW(scale_list_handle, CB_GETITEMDATA,
                    SendMessageW(scale_list_handle, CB_GETCURSEL, 0, 0), 0));
            return TRUE;
        }
        default:
            return FALSE;
        }
    case WM_INITDIALOG:
    {
        HWND message_handle = GetDlgItem(dialog_handle, IDC_REMAP_STAFF_SCALE_MESSAGE);
        HWND scale_list_handle = GetDlgItem(dialog_handle, IDC_REMAP_STAFF_SCALE_LIST);
        HWND ok_handle = GetDlgItem(dialog_handle, IDOK);
        HWND cancel_handle = GetDlgItem(dialog_handle, IDCANCEL);
        SendMessageW(message_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
        SendMessageW(ok_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
        SendMessageW(cancel_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
        SendMessageW(scale_list_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
        struct RemapScaleInfo*info = (struct RemapScaleInfo*)l_param;
        SendMessageW(scale_list_handle, CB_ADDSTRING, 0, (LPARAM)L"Default");
        SendMessageW(scale_list_handle, CB_SETITEMDATA, 0,
            ((struct StaffScale*)info->project->staff_scales->bytes)->address);
        LRESULT scale_count = SendMessageW(info->edit_scales_list_handle, CB_GETCOUNT, 0, 0);
        void*stack_savepoint = info->project->stack_a.cursor;
        wchar_t*string = start_array(&info->project->stack_a, _alignof(wchar_t));
        size_t remap_scales_index = 1;
        for (size_t edit_scales_list_index = 0; edit_scales_list_index < scale_count;
            ++edit_scales_list_index)
        {
            LRESULT scale_address = SendMessageW(info->edit_scales_list_handle, CB_GETITEMDATA,
                edit_scales_list_index, 0);
            if (scale_address != info->address_of_scale_to_remap)
            {
                extend_array(&info->project->stack_a,
                    sizeof(wchar_t) * SendMessageW(info->edit_scales_list_handle, CB_GETLBTEXTLEN,
                        edit_scales_list_index, 0));
                SendMessageW(info->edit_scales_list_handle, CB_GETLBTEXT, edit_scales_list_index,
                    (LPARAM)string);
                SendMessageW(scale_list_handle, CB_ADDSTRING, edit_scales_list_index,
                    (LPARAM)string);
                SendMessageW(scale_list_handle, CB_SETITEMDATA, remap_scales_index, scale_address);
                ++remap_scales_index;
                info->project->stack_a.cursor = string;
            }
        }
        info->project->stack_a.cursor = stack_savepoint;
        SendMessageW(scale_list_handle, CB_SETCURSEL, 0, 0);
        int32_t message_width = 2 * (COMMAND_BUTTON_WIDTH + RELATED_CONTROL_SPACER);
        int32_t y = UNRELATED_CONTROL_SPACER;
        int32_t message_height = 4 * BUTTON_HEIGHT;
        SetWindowPos(message_handle, 0, UNRELATED_CONTROL_SPACER, y, message_width, message_height,
            SWP_NOZORDER);
        y += message_height;
        SetWindowPos(scale_list_handle, 0, UNRELATED_CONTROL_SPACER, y, message_width,
            7 * ComboBox_GetItemHeight(scale_list_handle), SWP_NOZORDER);
        y += BUTTON_HEIGHT + UNRELATED_CONTROL_SPACER;
        SetWindowPos(ok_handle, 0, UNRELATED_CONTROL_SPACER, y, COMMAND_BUTTON_WIDTH, BUTTON_HEIGHT,
            SWP_NOZORDER);
        SetWindowPos(cancel_handle, 0,
            UNRELATED_CONTROL_SPACER + COMMAND_BUTTON_WIDTH + RELATED_CONTROL_SPACER, y,
            COMMAND_BUTTON_WIDTH, BUTTON_HEIGHT, SWP_NOZORDER);
        center_dialog(dialog_handle, message_width + 2 * UNRELATED_CONTROL_SPACER,
            y + BUTTON_HEIGHT + UNRELATED_CONTROL_SPACER);
        return TRUE;
    }
    }
    return FALSE;
}

LRESULT get_staff_scale_insertion_index(struct StaffScaleIter*out, struct Project*project,
    float scale_to_insert_value)
{
    initialize_page_element_iter(&out->base, project->staff_scales->bytes,
        sizeof(struct StaffScale));
    LRESULT scale_index = 0;
    while (true)
    {
        increment_page_element_iter(&out->base, &project->page_pool, sizeof(struct StaffScale));
        if (!out->scale || scale_to_insert_value > out->scale->value)
        {
            return scale_index;
        }
        ++scale_index;
    }
}

void format_edit_staff_scales_dialog(HDC device_context, HWND dialog_handle,
    int32_t max_scale_string_width)
{
    HWND scale_list_handle = GetDlgItem(dialog_handle, IDC_EDIT_SCALES_SCALE_LIST);
    int32_t edit_button_width = GET_TEXT_WIDTH(device_context, EDIT_SCALES_ADD_SCALE_STRING);
    max_scale_string_width = MAX(max_scale_string_width, edit_button_width);
    edit_button_width = GET_TEXT_WIDTH(device_context, EDIT_SCALES_EDIT_SCALE_STRING);
    max_scale_string_width = MAX(max_scale_string_width, edit_button_width);
    edit_button_width = GET_TEXT_WIDTH(device_context, EDIT_SCALES_REMOVE_SCALE_STRING);
    max_scale_string_width =
        MAX(max_scale_string_width, edit_button_width) + GetSystemMetrics(SM_CXVSCROLL);
    int32_t y = UNRELATED_CONTROL_SPACER;
    SetWindowPos(GetDlgItem(dialog_handle, IDC_EDIT_SCALES_ADD_SCALE), 0, UNRELATED_CONTROL_SPACER,
        y, max_scale_string_width, BUTTON_HEIGHT, SWP_NOZORDER);
    y += BUTTON_HEIGHT + RELATED_CONTROL_SPACER;
    SetWindowPos(GetDlgItem(dialog_handle, IDC_EDIT_SCALES_EDIT_SCALE), 0, UNRELATED_CONTROL_SPACER,
        y, max_scale_string_width, BUTTON_HEIGHT, SWP_NOZORDER);
    y += BUTTON_HEIGHT + RELATED_CONTROL_SPACER;
    SetWindowPos(GetDlgItem(dialog_handle, IDC_EDIT_SCALES_REMOVE_SCALE), 0,
        UNRELATED_CONTROL_SPACER, y, max_scale_string_width, BUTTON_HEIGHT, SWP_NOZORDER);
    y += BUTTON_HEIGHT + UNRELATED_CONTROL_SPACER;
    SetWindowPos(scale_list_handle, 0, UNRELATED_CONTROL_SPACER, y, max_scale_string_width,
        7 * ComboBox_GetItemHeight(scale_list_handle), SWP_NOZORDER);
    y += BUTTON_HEIGHT + UNRELATED_CONTROL_SPACER;
    int32_t window_width = max_scale_string_width + 2 * UNRELATED_CONTROL_SPACER;
    SetWindowPos(GetDlgItem(dialog_handle, IDOK), 0, (window_width - COMMAND_BUTTON_WIDTH) / 2, y,
        COMMAND_BUTTON_WIDTH, BUTTON_HEIGHT, SWP_NOZORDER);
    center_dialog(dialog_handle, window_width, y + BUTTON_HEIGHT + UNRELATED_CONTROL_SPACER);
}

void reset_edit_staff_scales_dialog_width(HWND dialog_handle)
{
    HDC device_context = GetDC(dialog_handle);
    HFONT old_font = (HFONT)SelectObject(device_context, TEXT_FONT);
    HWND scale_list_handle = GetDlgItem(dialog_handle, IDC_EDIT_SCALES_SCALE_LIST);
    int32_t max_scale_string_width = 0;
    LRESULT scale_count = SendMessage(scale_list_handle, CB_GETCOUNT, 0, 0);
    for (LRESULT i = 0; i < scale_count; ++i)
    {
        wchar_t scale_string[43];
        SendMessage(scale_list_handle, CB_GETLBTEXT, i, (LPARAM)scale_string);
        int32_t item_width = get_text_width(device_context, scale_string, wcslen(scale_string));
        max_scale_string_width = MAX(max_scale_string_width, item_width);
    }
    format_edit_staff_scales_dialog(device_context, dialog_handle, max_scale_string_width);
    SelectObject(device_context, old_font);
    ReleaseDC(dialog_handle, device_context);
}

//DWLP_USER is set to a pointer to a struct Project instance.
INT_PTR edit_staff_scales_dialog_proc(HWND dialog_handle, UINT message, WPARAM w_param,
    LPARAM l_param)
{
    switch (message)
    {
    case WM_COMMAND:
        switch (LOWORD(w_param))
        {
        case IDC_EDIT_SCALES_ADD_SCALE:
        {
            struct Project*project = (struct Project*)GetWindowLongPtrW(dialog_handle, DWLP_USER);
            struct StaffScaleIter new_scale_iter;
            size_t new_scale_index = get_staff_scale_insertion_index(&new_scale_iter, project, 1.0);
            insert_page_element_before_iter(&new_scale_iter.base, project,
                sizeof(struct StaffScale));
            new_scale_iter.scale->value = 1.0;
            memcpy(new_scale_iter.scale->value_string, L"1.0", sizeof(L"1.0"));
            memcpy(new_scale_iter.scale->name, L"New", sizeof(L"New"));
            HWND scale_list_handle = GetDlgItem(dialog_handle, IDC_EDIT_SCALES_SCALE_LIST);
            wchar_t new_scale_string[43];
            staff_scale_to_string(new_scale_iter.scale, new_scale_string);
            SendMessageW(scale_list_handle, CB_INSERTSTRING, new_scale_index,
                (LPARAM)new_scale_string);
            SendMessageW(scale_list_handle, CB_SETITEMDATA, new_scale_index,
                new_scale_iter.scale->address);
            SendMessageW(scale_list_handle, CB_SETCURSEL, new_scale_index, 0);
            EnableWindow(GetDlgItem(dialog_handle, IDC_EDIT_SCALES_REMOVE_SCALE), TRUE);
            EnableWindow(GetDlgItem(dialog_handle, IDC_EDIT_SCALES_EDIT_SCALE), TRUE);
            reset_edit_staff_scales_dialog_width(dialog_handle);
            return TRUE;
        }
        case IDC_EDIT_SCALES_EDIT_SCALE:
        {
            struct Project*project = (struct Project*)GetWindowLongPtrW(dialog_handle, DWLP_USER);
            HWND scale_list_handle = GetDlgItem(dialog_handle, IDC_EDIT_SCALES_SCALE_LIST);
            LRESULT unedited_index = SendMessageW(scale_list_handle, CB_GETCURSEL, 0, 0);
            LRESULT unedited_address =
                SendMessageW(scale_list_handle, CB_GETITEMDATA, unedited_index, 0);
            struct StaffScaleIter unedited_iter;
            initialize_page_element_iter(&unedited_iter.base,
                resolve_address(project, unedited_address), sizeof(struct StaffScale));
            DialogBoxIndirectParamW(0, &EDIT_STAFF_SCALE_DIALOG_TEMPLATE.header, dialog_handle,
                edit_staff_scale_dialog_proc, (LPARAM)unedited_iter.scale);
            struct StaffScaleIter edited_iter;
            LRESULT edited_index =
                get_staff_scale_insertion_index(&edited_iter, project, unedited_iter.scale->value);
            insert_page_element_before_iter(&edited_iter.base, project, sizeof(struct StaffScale));
            memcpy((uint8_t*)edited_iter.scale + sizeof(edited_iter.scale->address),
                (uint8_t*)unedited_iter.scale + sizeof(unedited_iter.scale->address),
                sizeof(struct StaffScale) - sizeof(unedited_iter.scale->address));
            remove_page_element_at_iter(&unedited_iter.base, project, sizeof(struct StaffScale));
            SendMessageW(scale_list_handle, CB_DELETESTRING, unedited_index, 0);
            if (unedited_index < edited_index)
            {
                --edited_index;
            }
            wchar_t edited_scale_string[43];
            staff_scale_to_string(edited_iter.scale, edited_scale_string);
            SendMessageW(scale_list_handle, CB_INSERTSTRING, edited_index,
                (LPARAM)edited_scale_string);
            SendMessageW(scale_list_handle, CB_SETCURSEL, edited_index, 0);
            reset_edit_staff_scales_dialog_width(dialog_handle);
            for (struct Staff*staff = resolve_pool_index(&project->staff_pool, 1);
                staff < project->staff_pool.cursor; ++staff)
            {
                if (staff->scale_address == unedited_address)
                {
                    staff->scale_address = edited_iter.scale->address;
                }
            }
            return TRUE;
        }
        case IDC_EDIT_SCALES_REMOVE_SCALE:
        {
            HWND scale_list_handle = GetDlgItem(dialog_handle, IDC_EDIT_SCALES_SCALE_LIST);
            LRESULT removal_index = SendMessageW(scale_list_handle, CB_GETCURSEL, 0, 0);
            uint32_t removal_address =
                SendMessageW(scale_list_handle, CB_GETITEMDATA, removal_index, 0);
            LRESULT remapped_address;
            struct Project*project = (struct Project*)GetWindowLongPtrW(dialog_handle, DWLP_USER);
            struct Staff*staff = resolve_pool_index(&project->staff_pool, 1);
            while (true)
            {
                if (staff == project->staff_pool.cursor)
                {
                    break;
                }
                if (!staff->is_on_free_list && staff->scale_address == removal_address)
                {
                    struct RemapScaleInfo info = { scale_list_handle, project, removal_address };
                    remapped_address = DialogBoxIndirectParamW(0,
                        &REMAP_STAFF_SCALE_DIALOG_TEMPLATE.header, dialog_handle,
                        remap_staff_scale_dialog_proc, (LPARAM)&info);
                    if (remapped_address < 0)
                    {
                        return TRUE;
                    }
                    break;
                }
                ++staff;
            }
            struct StaffScaleIter scale_iter;
            initialize_page_element_iter(&scale_iter.base,
                resolve_address(project, removal_address), sizeof(struct StaffScale));
            remove_page_element_at_iter(&scale_iter.base, project, sizeof(struct StaffScale));
            while (staff < project->staff_pool.cursor)
            {
                if (staff->scale_address == removal_address)
                {
                    staff->scale_address = remapped_address;
                }
                ++staff;
            }
            struct SliceIter slice_iter;
            initialize_page_element_iter(&slice_iter.base, project->slices->bytes,
                sizeof(struct Slice));
            while (slice_iter.slice)
            {
                slice_iter.slice->needs_respacing = true;
                increment_page_element_iter(&slice_iter.base, &project->page_pool,
                    sizeof(struct Slice));
            }
            SendMessageW(scale_list_handle, CB_DELETESTRING, removal_index, 0);
            SendMessageW(scale_list_handle, CB_SETCURSEL, 0, 0);
            if (!SendMessageW(scale_list_handle, CB_GETCOUNT, 0, 0))
            {
                EnableWindow(GetDlgItem(dialog_handle, IDC_EDIT_SCALES_REMOVE_SCALE), FALSE);
                EnableWindow(GetDlgItem(dialog_handle, IDC_EDIT_SCALES_EDIT_SCALE), FALSE);
            }
            reset_edit_staff_scales_dialog_width(dialog_handle);
            return TRUE;
        }
        case IDOK:
            EndDialog(dialog_handle, 0);
            return TRUE;
        }
        return FALSE;
    case WM_INITDIALOG:
    {
        SetWindowLongPtrW(dialog_handle, DWLP_USER, l_param);
        SendMessageW(GetDlgItem(dialog_handle, IDC_EDIT_SCALES_ADD_SCALE), WM_SETFONT,
            (WPARAM)TEXT_FONT, 0);
        SendMessageW(GetDlgItem(dialog_handle, IDC_EDIT_SCALES_EDIT_SCALE), WM_SETFONT,
            (WPARAM)TEXT_FONT, 0);
        SendMessageW(GetDlgItem(dialog_handle, IDC_EDIT_SCALES_REMOVE_SCALE), WM_SETFONT,
            (WPARAM)TEXT_FONT, 0);
        HWND scale_list_handle = GetDlgItem(dialog_handle, IDC_EDIT_SCALES_SCALE_LIST);
        SendMessageW(scale_list_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
        SendMessageW(GetDlgItem(dialog_handle, IDOK), WM_SETFONT, (WPARAM)TEXT_FONT, 0);
        HDC device_context = GetDC(dialog_handle);
        HFONT old_font = (HFONT)SelectObject(device_context, TEXT_FONT);
        format_edit_staff_scales_dialog(device_context, dialog_handle,
            populate_staff_scale_list(device_context, scale_list_handle, (struct Project*)l_param,
                0));
        SelectObject(device_context, old_font);
        ReleaseDC(dialog_handle, device_context);
        return TRUE;
    }
    }
    return FALSE;
}