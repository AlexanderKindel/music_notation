#include "declarations.h"

struct Clef get_selected_clef(struct Project*project)
{
    struct Clef out;
    if (SendMessageW(project->c_clef_handle, BM_GETCHECK, 0, 0) == BST_CHECKED)
    {
        out.steps_of_baseline_above_staff_middle = 0;
        if (SendMessageW(project->clef_none_handle, BM_GETCHECK, 0, 0) == BST_CHECKED)
        {
            out.codepoint = 0xe05c;
        }
        else
        {
            out.codepoint = 0xe05d;
        }
    }
    else if (SendMessageW(project->f_clef_handle, BM_GETCHECK, 0, 0) == BST_CHECKED)
    {
        out.steps_of_baseline_above_staff_middle = 2;
        if (SendMessageW(project->clef_15ma_handle, BM_GETCHECK, 0, 0) == BST_CHECKED)
        {
            out.codepoint = 0xe066;
        }
        else if (SendMessageW(project->clef_8va_handle, BM_GETCHECK, 0, 0) == BST_CHECKED)
        {
            out.codepoint = 0xe065;
        }
        else if (SendMessageW(project->clef_none_handle, BM_GETCHECK, 0, 0) == BST_CHECKED)
        {
            out.codepoint = 0xe062;
        }
        else if (SendMessageW(project->clef_8vb_handle, BM_GETCHECK, 0, 0) == BST_CHECKED)
        {
            out.codepoint = 0xe064;
        }
        else
        {
            out.codepoint = 0xe063;
        }
    }
    else if (SendMessageW(project->g_clef_handle, BM_GETCHECK, 0, 0) == BST_CHECKED)
    {
        out.steps_of_baseline_above_staff_middle = -2;
        if (SendMessageW(project->clef_15ma_handle, BM_GETCHECK, 0, 0) == BST_CHECKED)
        {
            out.codepoint = 0xe054;
        }
        else if (SendMessageW(project->clef_8va_handle, BM_GETCHECK, 0, 0) == BST_CHECKED)
        {
            out.codepoint = 0xe053;
        }
        else if (SendMessageW(project->clef_none_handle, BM_GETCHECK, 0, 0) == BST_CHECKED)
        {
            out.codepoint = 0xe050;
        }
        else if (SendMessageW(project->clef_8vb_handle, BM_GETCHECK, 0, 0) == BST_CHECKED)
        {
            out.codepoint = 0xe052;
        }
        else
        {
            out.codepoint = 0xe051;
        }
    }
    else
    {
        out.steps_of_baseline_above_staff_middle = 0;
        out.codepoint = 0xe069;
    }
    return out;
}

LRESULT clef_tab_proc(HWND window_handle, UINT message, WPARAM w_param, LPARAM l_param,
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
            if ((HWND)l_param == project->add_clef_button_handle)
            {
                switch (project->selection.selection_type)
                {
                case SELECTION_CURSOR:
                {
                    struct Object*clef =
                        resolve_address(project, project->selection.address.object_address);
                    struct ObjectIter iter;
                    initialize_page_element_iter(&iter.base, clef, sizeof(struct Object));
                    insert_sliceless_object_before_iter(&iter, project);
                    clef->clef = get_selected_clef(project);
                    clef->object_type = OBJECT_CLEF;
                    clef->is_selected = false;
                    clef->is_valid_cursor_position = true;
                    project->selection.address.object_address = clef->address;
                    set_cursor_to_next_valid_state(project);
                    invalidate_work_region(main_window_handle, project);
                    return 0;
                }
                case SELECTION_OBJECT:
                {
                    struct ObjectIter iter;
                    initialize_page_element_iter(&iter.base,
                        resolve_address(project, project->selection.address.object_address),
                        sizeof(struct Object));
                    while (iter.object->object_type != OBJECT_CLEF)
                    {
                        decrement_page_element_iter(&iter.base, &project->page_pool,
                            sizeof(struct Object));
                    }
                    iter.object->clef = get_selected_clef(project);
                    cancel_selection(main_window_handle);
                    project->selection.selection_type = SELECTION_CURSOR;
                    project->selection.address.object_address = iter.object->address;
                    set_cursor_to_next_valid_state(project);
                    get_next_slice_right_of_iter(&iter, project)->needs_respacing = true;
                }
                }
            }
            else if ((HWND)l_param == project->c_clef_handle)
            {
                EnableWindow(project->clef_15ma_handle, FALSE);
                EnableWindow(project->clef_8va_handle, FALSE);
                EnableWindow(project->clef_none_handle, TRUE);
                EnableWindow(project->clef_8vb_handle, TRUE);
                EnableWindow(project->clef_15mb_handle, FALSE);
                if (!(SendMessageW(project->clef_none_handle, BM_GETCHECK, 0, 0) == BST_CHECKED ||
                    SendMessageW(project->clef_8vb_handle, BM_GETCHECK, 0, 0) == BST_CHECKED))
                {
                    SendMessageW(project->clef_15ma_handle, BM_SETCHECK, BST_UNCHECKED, 0);
                    SendMessageW(project->clef_8va_handle, BM_SETCHECK, BST_UNCHECKED, 0);
                    SendMessageW(project->clef_none_handle, BM_SETCHECK, BST_CHECKED, 0);
                    SendMessageW(project->clef_8vb_handle, BM_SETCHECK, BST_UNCHECKED, 0);
                    SendMessageW(project->clef_15mb_handle, BM_SETCHECK, BST_UNCHECKED, 0);
                }
            }
            else if ((HWND)l_param == project->clef_none_handle)
            {
                EnableWindow(project->clef_15ma_handle, FALSE);
                EnableWindow(project->clef_8va_handle, FALSE);
                EnableWindow(project->clef_none_handle, FALSE);
                EnableWindow(project->clef_8vb_handle, FALSE);
                EnableWindow(project->clef_15mb_handle, FALSE);
            }
            else
            {
                EnableWindow(project->clef_15ma_handle, TRUE);
                EnableWindow(project->clef_8va_handle, TRUE);
                EnableWindow(project->clef_none_handle, TRUE);
                EnableWindow(project->clef_8vb_handle, TRUE);
                EnableWindow(project->clef_15mb_handle, TRUE);
            }
            invalidate_work_region(main_window_handle, project);
            return 0;
        }
    }
    return DefWindowProcW(window_handle, message, w_param, l_param);
}