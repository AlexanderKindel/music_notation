#include "edit_staff_scales_dialog/edit_staff_scales_dialog.h"

size_t staff_scale_to_string(struct StaffScale*scale, wchar_t*out);
int32_t populate_staff_scale_list(HDC device_context, HWND scale_list_handle,
    struct Project*project, size_t starting_index);
INT_PTR edit_staff_scales_dialog_proc(HWND dialog_handle, UINT message, WPARAM w_param,
    LPARAM l_param);