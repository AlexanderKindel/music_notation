#include "content.c"
#include "display.c"
#include "gui.c"

HWND init(HINSTANCE instance_handle, struct Project*main_window_memory)
{
    DURATION_RATIO = sqrtf(sqrtf(UZ_WHOLE_NOTE_WIDTH / 2.0));
    COLORREF gray = RGB(127, 127, 127);
    GRAY_PEN = CreatePen(PS_SOLID, 1, gray);
    CUSTOM_GRAY_BRUSH = CreateSolidBrush(gray);
    RED_PEN = CreatePen(PS_SOLID, 1, RED);
    RED_BRUSH = CreateSolidBrush(RED);
    SYSTEM_INFO system_info;
    GetSystemInfo(&system_info);
    PAGE_SIZE = system_info.dwPageSize;
    main_window_memory->staff_scales[0].value = 1.0;
    wchar_t default_string[] = L"Default";
    memcpy(main_window_memory->staff_scales[0].name, default_string, sizeof(default_string));
    wchar_t default_value_string[] = L"1.0";
    memcpy(main_window_memory->staff_scales[0].value_string, default_value_string,
        sizeof(default_value_string));
    main_window_memory->staff_scales[1].value = 0.75;
    wchar_t cue_string[] = L"Cue";
    memcpy(main_window_memory->staff_scales[1].name, cue_string, sizeof(cue_string));
    wchar_t cue_value_string[] = L"0.75";
    memcpy(main_window_memory->staff_scales[1].value_string, cue_value_string,
        sizeof(cue_value_string));
    main_window_memory->staff_scales[2].name[0] = 0;
    uint32_t pool_count = sizeof(pool_element_sizes) / sizeof(uint32_t);
    size_t pool_element_size_sum = PAGE_SIZE;
    for (uint32_t i = 0; i < pool_count; ++i)
    {
        pool_element_size_sum += pool_element_sizes[i];
    }
    initialize_pool(&main_window_memory->page_pool, PAGE_SIZE * UINT32_MAX,
        VirtualAlloc(0, pool_element_size_sum * UINT32_MAX, MEM_RESERVE, PAGE_READWRITE),
        PAGE_SIZE);
    void*previous_pool_end = main_window_memory->page_pool.end;
    for (uint32_t i = 0; i < pool_count; ++i)
    {
        initialize_pool(&main_window_memory->other_pools[i], pool_element_sizes[i] * UINT32_MAX,
            previous_pool_end, pool_element_sizes[i]);
        previous_pool_end = main_window_memory->other_pools[i].end;
    }
    main_window_memory->misc_stack.start = previous_pool_end;
    main_window_memory->misc_stack.end =
        (void*)((size_t)main_window_memory->misc_stack.start + PAGE_SIZE);
    main_window_memory->misc_stack.cursor = main_window_memory->misc_stack.start;
    main_window_memory->misc_stack.cursor_max = main_window_memory->misc_stack.cursor;
    main_window_memory->selection.selection_type = SELECTION_NONE;
    main_window_memory->slices = allocate_pool_slot(&main_window_memory->page_pool);
    main_window_memory->slices->previous_page_index = 0;
    main_window_memory->slices->next_page_index = 0;
    main_window_memory->slices->capacity = (PAGE_SIZE - sizeof(struct Page)) / sizeof(struct Slice);
    main_window_memory->slices->occupied_slot_count = 0;
    struct SliceIter slice_iter = { 0, main_window_memory->slices,
        (struct Slice*)(main_window_memory->slices->bytes) };
    insert_slice_before_iter(&slice_iter, main_window_memory);
    STAFF_START_SLICE_ADDRESS = slice_iter.slice->address;
    slice_iter.slice->uz_distance_from_previous_slice = 20;
    slice_iter.slice->first_object_address_node_index = 0;
    slice_iter.slice->whole_notes_long.denominator = 0;
    slice_iter.slice->needs_respacing = false;
    increment_page_element_iter(&slice_iter.base, &main_window_memory->page_pool,
        sizeof(struct Slice));
    insert_slice_before_iter(&slice_iter, main_window_memory);
    HEADER_CLEF_SLICE_ADDRESS = slice_iter.slice->address;
    slice_iter.slice->uz_distance_from_previous_slice = 0;
    slice_iter.slice->whole_notes_long.denominator = 0;
    increment_page_element_iter(&slice_iter.base, &main_window_memory->page_pool,
        sizeof(struct Slice));
    insert_slice_before_iter(&slice_iter, main_window_memory);
    HEADER_KEY_SIG_SLICE_ADDRESS = slice_iter.slice->address;
    slice_iter.slice->uz_distance_from_previous_slice = 0;
    slice_iter.slice->whole_notes_long.denominator = 0;
    increment_page_element_iter(&slice_iter.base, &main_window_memory->page_pool,
        sizeof(struct Slice));
    insert_slice_before_iter(&slice_iter, main_window_memory);
    HEADER_TIME_SIG_SLICE_ADDRESS = slice_iter.slice->address;
    slice_iter.slice->uz_distance_from_previous_slice = 0;
    slice_iter.slice->whole_notes_long.denominator = 0;
    increment_page_element_iter(&slice_iter.base, &main_window_memory->page_pool,
        sizeof(struct Slice));
    insert_slice_before_iter(&slice_iter, main_window_memory);
    BODY_START_SLICE_ADDRESS = slice_iter.slice->address;
    slice_iter.slice->uz_distance_from_previous_slice = 0;
    slice_iter.slice->whole_notes_long.numerator =
        initialize_pool_integer(&INTEGER_POOL(main_window_memory), 0);
    slice_iter.slice->whole_notes_long.denominator =
        initialize_pool_integer(&INTEGER_POOL(main_window_memory), 1);
    main_window_memory->ghost_cursor_address.object_address = 0;
    InitCommonControlsEx(&(INITCOMMONCONTROLSEX){ sizeof(INITCOMMONCONTROLSEX),
        ICC_BAR_CLASSES | ICC_STANDARD_CLASSES | ICC_TAB_CLASSES | ICC_UPDOWN_CLASS });
    RegisterClassW(&(WNDCLASSW){ CS_HREDRAW | CS_OWNDC, main_window_proc, 0,
        sizeof(struct Project*), instance_handle, 0, LoadCursorA(0, IDC_ARROW),
        (HBRUSH)(COLOR_WINDOW + 1), 0, L"main" });
    HWND main_window_handle = CreateWindowExW(0, L"main", L"Music Notation",
        WS_OVERLAPPEDWINDOW | WS_VISIBLE, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT,
        CW_USEDEFAULT, 0, 0, instance_handle, 0);
    NONCLIENTMETRICSW metrics;
    metrics.cbSize = sizeof(NONCLIENTMETRICSW);
    SystemParametersInfoW(SPI_GETNONCLIENTMETRICS, metrics.cbSize, &metrics, 0);
    HFONT text_font = CreateFontIndirectW(&metrics.lfMessageFont);
    main_window_memory->control_tabs_handle = CreateWindowExW(0, L"SysTabControl32", 0,
        WS_CHILD | WS_VISIBLE, 0, 0, 0, 0, main_window_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->control_tabs_handle, WM_SETFONT, (WPARAM)text_font, 0);
    int32_t tab_top = 25;
    int32_t tab_height = CONTROL_TABS_HEIGHT - tab_top;
    SendMessageW(main_window_memory->control_tabs_handle, TCM_INSERTITEMW, STAFF_TAB_INDEX,
        (LPARAM)&(TCITEMW){ TCIF_TEXT, 0, 0, L"Staves", 0, -1, 0 });
    main_window_memory->staff_tab_handle = CreateWindowExW(0, L"static", 0, WS_CHILD | WS_VISIBLE,
        0, tab_top, 500, tab_height, main_window_memory->control_tabs_handle, 0, instance_handle,
        0);
    SetWindowSubclass(main_window_memory->staff_tab_handle, staff_tab_proc, 0, 0);
    main_window_memory->add_staff_button_handle = CreateWindowExW(0, L"button", L"Add staff",
        BS_PUSHBUTTON | BS_VCENTER | WS_CHILD | WS_VISIBLE, 10, 10, 55, 20,
        main_window_memory->staff_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->add_staff_button_handle, WM_SETFONT, (WPARAM)text_font, 0);
    SendMessageW(main_window_memory->control_tabs_handle, TCM_INSERTITEMW, CLEF_TAB_INDEX,
        (LPARAM)&(TCITEMW){ TCIF_TEXT, 0, 0, L"Clefs", 0, -1, 0 });
    main_window_memory->clef_tab_handle = CreateWindowExW(0, L"static", 0, WS_CHILD, 0, tab_top,
        500, tab_height, main_window_memory->control_tabs_handle, 0, instance_handle, 0);
    SetWindowSubclass(main_window_memory->clef_tab_handle, clef_tab_proc, 0, 0);
    HWND clef_shape_label_handle = CreateWindowExW(0, L"static", L"Shape:",
        SS_LEFT | WS_CHILD | WS_VISIBLE, 5, 0, 50, 20, main_window_memory->clef_tab_handle, 0,
        instance_handle, 0);
    SendMessageW(clef_shape_label_handle, WM_SETFONT, (WPARAM)text_font, 0);
    main_window_memory->c_clef_handle = CreateWindowExW(0, L"button", L"C",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_GROUP | WS_VISIBLE, 60, 0, 35, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->c_clef_handle, WM_SETFONT, (WPARAM)text_font, 0);
    main_window_memory->f_clef_handle = CreateWindowExW(0, L"button", L"F",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 115, 0, 35, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->f_clef_handle, WM_SETFONT, (WPARAM)text_font, 0);
    main_window_memory->g_clef_handle = CreateWindowExW(0, L"button", L"G",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 170, 0, 35, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->g_clef_handle, WM_SETFONT, (WPARAM)text_font, 0);
    SendMessageW(main_window_memory->g_clef_handle, BM_SETCHECK, BST_CHECKED, 0);
    HWND unpitched_clef_handle = CreateWindowExW(0, L"button", L"Unpitched",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 225, 0, 75, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(unpitched_clef_handle, WM_SETFONT, (WPARAM)text_font, 0);
    HWND clef_octave_label_handle = CreateWindowExW(0, L"static", L"Octave:",
        SS_LEFT | WS_CHILD | WS_VISIBLE, 5, 20, 50, 20, main_window_memory->clef_tab_handle, 0,
        instance_handle, 0);
    SendMessageW(clef_octave_label_handle, WM_SETFONT, (WPARAM)text_font, 0);
    main_window_memory->clef_15ma_handle = CreateWindowExW(0, L"button", L"15ma",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_GROUP | WS_VISIBLE, 60, 20, 50, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->clef_15ma_handle, WM_SETFONT, (WPARAM)text_font, 0);
    main_window_memory->clef_8va_handle = CreateWindowExW(0, L"button", L"8va",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 115, 20, 50, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->clef_8va_handle, WM_SETFONT, (WPARAM)text_font, 0);
    main_window_memory->clef_none_handle = CreateWindowExW(0, L"button", L"None",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 170, 20, 50, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->clef_none_handle, WM_SETFONT, (WPARAM)text_font, 0);
    SendMessageW(main_window_memory->clef_none_handle, BM_SETCHECK, BST_CHECKED, 0);
    main_window_memory->clef_8vb_handle = CreateWindowExW(0, L"button", L"8vb",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 225, 20, 50, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->clef_8vb_handle, WM_SETFONT, (WPARAM)text_font, 0);
    main_window_memory->clef_15mb_handle = CreateWindowExW(0, L"button", L"15ma",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 280, 20, 50, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->clef_15mb_handle, WM_SETFONT, (WPARAM)text_font, 0);
    main_window_memory->add_clef_button_handle = CreateWindowExW(0, L"button", L"Add clef",
        BS_PUSHBUTTON | WS_DISABLED | WS_CHILD | WS_VISIBLE | BS_VCENTER, 335, 10, 55, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->add_clef_button_handle, WM_SETFONT, (WPARAM)text_font, 0);
    SendMessageW(main_window_memory->control_tabs_handle, TCM_INSERTITEMW, KEY_SIG_TAB_INDEX,
        (LPARAM)&(TCITEMW){ TCIF_TEXT, 0, 0, L"Key Sigs", 0, -1, 0 });
    main_window_memory->key_sig_tab_handle = CreateWindowExW(0, L"static", 0, WS_CHILD, 0, tab_top,
        500, tab_height, main_window_memory->control_tabs_handle, 0, instance_handle, 0);
    SetWindowSubclass(main_window_memory->key_sig_tab_handle, key_sig_tab_proc, 0, 0);
    HWND accidental_count_label_handle = CreateWindowExW(0, L"static", L"Accidental count:",
        SS_LEFT | WS_CHILD | WS_VISIBLE, 5, 10, 95, 20, main_window_memory->key_sig_tab_handle, 0,
        instance_handle, 0);
    SendMessageW(accidental_count_label_handle, WM_SETFONT, (WPARAM)text_font, 0);
    HWND accidental_count_display_handle = CreateWindowExW(0, L"static", 0,
        WS_BORDER | WS_CHILD | WS_VISIBLE, 105, 10, 30, 20, main_window_memory->key_sig_tab_handle,
        0, instance_handle, 0);
    SendMessageW(accidental_count_display_handle, WM_SETFONT, (WPARAM)text_font, 0);
    main_window_memory->accidental_count_spin_handle = CreateWindowExW(0, UPDOWN_CLASSW, 0,
        UDS_ALIGNRIGHT | UDS_AUTOBUDDY | UDS_SETBUDDYINT | WS_CHILD | WS_VISIBLE, 0, 0, 0, 0,
        main_window_memory->key_sig_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->accidental_count_spin_handle, UDM_SETRANGE32, 0, 7);
    main_window_memory->sharps_handle = CreateWindowExW(0, L"button", L"Sharps",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_DISABLED | WS_GROUP | WS_VISIBLE, 150, 0, 55, 20,
        main_window_memory->key_sig_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->sharps_handle, BM_SETCHECK, BST_CHECKED, 0);
    SendMessageW(main_window_memory->sharps_handle, WM_SETFONT, (WPARAM)text_font, 0);
    main_window_memory->flats_handle = CreateWindowExW(0, L"button", L"Flats",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_DISABLED | WS_VISIBLE, 150, 20, 55, 20,
        main_window_memory->key_sig_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->flats_handle, WM_SETFONT, (WPARAM)text_font, 0);
    main_window_memory->add_key_sig_button_handle = CreateWindowExW(0, L"button",
        L"Add key signature", BS_PUSHBUTTON | BS_VCENTER | WS_DISABLED | WS_CHILD | WS_VISIBLE, 215,
        10, 105, 20, main_window_memory->key_sig_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->control_tabs_handle, TCM_INSERTITEMW, TIME_SIG_TAB_INDEX,
        (LPARAM)&(TCITEMW){ TCIF_TEXT, 0, 0, L"Time sigs", 0, -1, 0 });
    SendMessageW(main_window_memory->add_key_sig_button_handle, WM_SETFONT, (WPARAM)text_font, 0);
    main_window_memory->time_sig_tab_handle = CreateWindowExW(0, L"static", 0, WS_CHILD, 0, tab_top,
        500, tab_height, main_window_memory->control_tabs_handle, 0, instance_handle, 0);
    SetWindowSubclass(main_window_memory->time_sig_tab_handle, time_sig_tab_proc, 0, 0);
    HWND numerator_label_handle = CreateWindowExW(0, L"static", L"Numerator:",
        SS_LEFT | WS_CHILD | WS_VISIBLE, 5, 0, 90, 20, main_window_memory->time_sig_tab_handle, 0,
        instance_handle, 0);
    SendMessageW(numerator_label_handle, WM_SETFONT, (WPARAM)text_font, 0);
    HWND numerator_display_handle = CreateWindowExW(0, L"static", 0,
        WS_BORDER | WS_CHILD | WS_VISIBLE, 90, 0, 45, 20, main_window_memory->time_sig_tab_handle,
        0, instance_handle, 0);
    SendMessageW(numerator_display_handle, WM_SETFONT, (WPARAM)text_font, 0);
    main_window_memory->numerator_spin_handle = CreateWindowExW(0, UPDOWN_CLASSW, 0,
        UDS_ALIGNRIGHT | UDS_AUTOBUDDY | UDS_SETBUDDYINT | WS_CHILD | WS_VISIBLE, 0, 0, 0, 0,
        main_window_memory->time_sig_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->numerator_spin_handle, UDM_SETRANGE32, 0, 100);
    SendMessageW(main_window_memory->numerator_spin_handle, UDM_SETPOS32, 0, 4);
    HWND denominator_label_handle = CreateWindowExW(0, L"static", L"Denominator:",
        SS_LEFT | WS_CHILD | WS_VISIBLE, 5, 20, 90, 20, main_window_memory->time_sig_tab_handle, 0,
        instance_handle, 0);
    SendMessageW(denominator_label_handle, WM_SETFONT, (WPARAM)text_font, 0);
    main_window_memory->denominator_display_handle = CreateWindowExW(0, L"static", L"4",
        WS_BORDER | WS_CHILD | WS_VISIBLE, 90, 20, 45, 20, main_window_memory->time_sig_tab_handle,
        0, instance_handle, 0);
    SendMessageW(main_window_memory->denominator_display_handle, WM_SETFONT, (WPARAM)text_font, 0);
    main_window_memory->denominator_spin_handle = CreateWindowExW(0, UPDOWN_CLASSW, 0,
        UDS_ALIGNRIGHT | UDS_AUTOBUDDY | WS_CHILD | WS_VISIBLE, 0, 0, 0, 0,
        main_window_memory->time_sig_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->denominator_spin_handle, UDM_SETRANGE32, MIN_LOG2_DURATION, 0);
    SendMessageW(main_window_memory->denominator_spin_handle, UDM_SETPOS32, 0, -2);
    main_window_memory->add_time_sig_button_handle = CreateWindowExW(0, L"button",
        L"Add time signature", BS_PUSHBUTTON | BS_VCENTER | WS_DISABLED | WS_CHILD | WS_VISIBLE,
        145, 10, 115, 20, main_window_memory->time_sig_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->add_time_sig_button_handle, WM_SETFONT, (WPARAM)text_font, 0);
    SendMessageW(main_window_memory->control_tabs_handle, TCM_INSERTITEMW, NOTE_TAB_INDEX,
        (LPARAM)&(TCITEMW){ TCIF_TEXT, 0, 0, L"Notes", 0, -1, 0 });
    main_window_memory->note_tab_handle = CreateWindowExW(0, L"static", 0, WS_CHILD, 0, tab_top,
        500, tab_height, main_window_memory->control_tabs_handle, 0, instance_handle, 0);
    SetWindowSubclass(main_window_memory->note_tab_handle, note_tab_proc, 0, 0);
    int32_t x = 0;
    int32_t label_height = 20;
    HWND duration_label_handle = CreateWindowExW(0, L"static", L"Duration:",
        SS_CENTER | WS_CHILD | WS_VISIBLE, 0, 0, 110, label_height,
        main_window_memory->note_tab_handle, 0, instance_handle, 0);
    SendMessageW(duration_label_handle, WM_SETFONT, (WPARAM)text_font, 0);
    main_window_memory->duration_display_handle = CreateWindowExW(0, L"static", L"quarter",
        WS_BORDER | WS_CHILD | WS_VISIBLE, x, label_height, 110, label_height,
        main_window_memory->note_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->duration_display_handle, WM_SETFONT, (WPARAM)text_font, 0);
    main_window_memory->duration_spin_handle = CreateWindowExW(0, UPDOWN_CLASSW, 0,
        UDS_ALIGNRIGHT | UDS_AUTOBUDDY | WS_CHILD | WS_VISIBLE, 0, 0, 0, 0,
        main_window_memory->note_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->duration_spin_handle, UDM_SETRANGE32, MIN_LOG2_DURATION,
        MAX_LOG2_DURATION);
    SendMessageW(main_window_memory->duration_spin_handle, UDM_SETPOS32, 0, -2);
    x += 110;
    HWND augmentation_dot_label_handle = CreateWindowExW(0, L"static", L"Augmentation dots:",
        SS_CENTER | WS_CHILD | WS_VISIBLE, x, 0, 110, 20, main_window_memory->note_tab_handle, 0,
        instance_handle, 0);
    SendMessageW(augmentation_dot_label_handle, WM_SETFONT, (WPARAM)text_font, 0);
    HWND augmentation_dot_display_handle = CreateWindowExW(0, L"static", 0,
        WS_BORDER | WS_VISIBLE | WS_CHILD, x, label_height, 110, 20,
        main_window_memory->note_tab_handle, 0, instance_handle, 0);
    SendMessageW(augmentation_dot_display_handle, WM_SETFONT, (WPARAM)text_font, 0);
    main_window_memory->augmentation_dot_spin_handle = CreateWindowExW(0, UPDOWN_CLASSW, 0,
        UDS_ALIGNRIGHT | UDS_AUTOBUDDY | UDS_SETBUDDYINT | WS_CHILD | WS_VISIBLE, 0, 0, 0, 0,
        main_window_memory->note_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->augmentation_dot_spin_handle, UDM_SETRANGE32, 0,
        -2 - MIN_LOG2_DURATION);
    main_window_memory->main_window_back_buffer = 0;
    main_window_memory->uz_viewport_offset = (POINT){ 0, 0 };
    main_window_memory->uz_default_staff_space_height = 10.0;
    main_window_memory->utuz_x_of_slice_beyond_leftmost_to_draw = 0;
    main_window_memory->utuz_y_of_staff_above_highest_visible = 0;
    main_window_memory->utuz_last_slice_x = 20;
    main_window_memory->utuz_bottom_staff_y = DEFAULT_TOP_STAFF_MIDDLE_Y;
    main_window_memory->topmost_staff_index = 0;
    main_window_memory->highest_visible_staff_index = 0;
    main_window_memory->address_of_leftmost_slice_to_draw = STAFF_START_SLICE_ADDRESS;
    main_window_memory->zoom_exponent = 0;
    SetWindowLongPtrW(main_window_handle, GWLP_USERDATA, (LONG_PTR)main_window_memory);
    ADD_STAFF_DIALOG_TEMPLATE = allocate_stack_slot(&main_window_memory->misc_stack,
        sizeof(DLGTEMPLATE), _alignof(DWORD));
    ADD_STAFF_DIALOG_TEMPLATE->style = DS_CENTER | DS_SETFONT;
    ADD_STAFF_DIALOG_TEMPLATE->dwExtendedStyle = 0;
    ADD_STAFF_DIALOG_TEMPLATE->cdit = 0;
    ADD_STAFF_DIALOG_TEMPLATE->x = 0;
    ADD_STAFF_DIALOG_TEMPLATE->y = 0;
    ADD_STAFF_DIALOG_TEMPLATE->cx = 165;
    ADD_STAFF_DIALOG_TEMPLATE->cy = 80;
    *(uint32_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint32_t)) = 0;
    wchar_t add_staff_dialog_title[] = L"Add Staff";
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(add_staff_dialog_title)),
        add_staff_dialog_title, sizeof(add_staff_dialog_title));
    if (metrics.lfMessageFont.lfHeight < 0)
    {
        metrics.lfMessageFont.lfHeight =
            (-metrics.lfMessageFont.lfHeight * 72) / GetDeviceCaps(GetDC(0), LOGPIXELSY);
    }
    void*font_info = extend_array(&main_window_memory->misc_stack, sizeof(uint16_t));
    *(uint16_t*)font_info = metrics.lfMessageFont.lfHeight;
    wchar_t*name_character = metrics.lfMessageFont.lfFaceName;
    while (true)
    {
        *(wchar_t*)extend_array(&main_window_memory->misc_stack, sizeof(wchar_t)) = *name_character;
        if (!*name_character)
        {
            break;
        }
        ++name_character;
    }
    size_t font_info_length = (size_t)main_window_memory->misc_stack.cursor - (size_t)font_info;
    ++ADD_STAFF_DIALOG_TEMPLATE->cdit;
    DLGITEMTEMPLATE*item = allocate_stack_slot(&main_window_memory->misc_stack,
        sizeof(DLGITEMTEMPLATE), _alignof(DWORD));
    item->style = BS_PUSHBUTTON | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 80;
    item->y = 65;
    item->cx = 30;
    item->cy = 10;
    item->id = IDCANCEL;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0xffff;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0x80;
    wchar_t cancel_string[] = L"Cancel";
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(cancel_string)), cancel_string,
        sizeof(cancel_string));
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    ++ADD_STAFF_DIALOG_TEMPLATE->cdit;
    item = allocate_stack_slot(&main_window_memory->misc_stack, sizeof(DLGITEMTEMPLATE),
        _alignof(DWORD));
    item->style = BS_PUSHBUTTON | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 50;
    item->y = 65;
    item->cx = 30;
    item->cy = 10;
    item->id = IDOK;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0xffff;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0x80;
    wchar_t ok_string[] = L"OK";
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(ok_string)), ok_string,
        sizeof(ok_string));
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    ++ADD_STAFF_DIALOG_TEMPLATE->cdit;
    item = allocate_stack_slot(&main_window_memory->misc_stack, sizeof(DLGITEMTEMPLATE),
        _alignof(DWORD));
    item->style = SS_LEFT | WS_CHILD | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 5;
    item->y = 5;
    item->cx = 40;
    item->cy = 10;
    item->id = 0;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0xffff;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0x82;
    wchar_t line_count_string[] = L"Line count:";
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(line_count_string)),
        line_count_string, sizeof(line_count_string));
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    ++ADD_STAFF_DIALOG_TEMPLATE->cdit;
    item = allocate_stack_slot(&main_window_memory->misc_stack, sizeof(DLGITEMTEMPLATE),
        _alignof(DWORD));
    item->style = WS_BORDER | WS_CHILD | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 45;
    item->y = 5;
    item->cx = 20;
    item->cy = 10;
    item->id = IDC_ADD_STAFF_LINE_COUNT_DISPLAY;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0xffff;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0x82;
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(L"5")), L"5", sizeof(L"5"));
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    ++ADD_STAFF_DIALOG_TEMPLATE->cdit;
    item = allocate_stack_slot(&main_window_memory->misc_stack, sizeof(DLGITEMTEMPLATE),
        _alignof(DWORD));
    item->style = UDS_ALIGNRIGHT | UDS_AUTOBUDDY | UDS_SETBUDDYINT | WS_CHILD | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 0;
    item->y = 0;
    item->cx = 0;
    item->cy = 0;
    item->id = IDC_ADD_STAFF_LINE_COUNT_SPIN;
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(UPDOWN_CLASSW)), UPDOWN_CLASSW,
        sizeof(UPDOWN_CLASSW));
    *(wchar_t*)extend_array(&main_window_memory->misc_stack, sizeof(wchar_t)) = 0;
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    ++ADD_STAFF_DIALOG_TEMPLATE->cdit;
    item = allocate_stack_slot(&main_window_memory->misc_stack, sizeof(DLGITEMTEMPLATE),
        _alignof(DWORD));
    item->style = SS_LEFT | WS_CHILD | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 5;
    item->y = 25;
    item->cx = 60;
    item->cy = 10;
    item->id = 0;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0xffff;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0x82;
    wchar_t scale_string[] = L"Scale:";
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(scale_string)), scale_string,
        sizeof(scale_string));
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    ++ADD_STAFF_DIALOG_TEMPLATE->cdit;
    item = allocate_stack_slot(&main_window_memory->misc_stack, sizeof(DLGITEMTEMPLATE),
        _alignof(DWORD));
    item->style = CBS_DROPDOWNLIST | CBS_HASSTRINGS | WS_CHILD | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 5;
    item->y = 35;
    item->cx = 70;
    item->cy = 100;
    item->id = IDC_ADD_STAFF_SCALE_LIST;
    wchar_t combobox_string[] = L"COMBOBOX";
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(combobox_string)), combobox_string,
        sizeof(combobox_string));
    *(wchar_t*)extend_array(&main_window_memory->misc_stack, sizeof(wchar_t)) = 0;
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    ++ADD_STAFF_DIALOG_TEMPLATE->cdit;
    item = allocate_stack_slot(&main_window_memory->misc_stack, sizeof(DLGITEMTEMPLATE),
        _alignof(DWORD));
    item->style = BS_PUSHBUTTON | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 85;
    item->y = 25;
    item->cx = 75;
    item->cy = 10;
    item->id = IDC_ADD_STAFF_ADD_SCALE;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0xffff;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0x80;
    wchar_t add_scale_string[] = L"Add new scale";
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(add_scale_string)),
        add_scale_string, sizeof(add_scale_string));
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    ++ADD_STAFF_DIALOG_TEMPLATE->cdit;
    item = allocate_stack_slot(&main_window_memory->misc_stack, sizeof(DLGITEMTEMPLATE),
        _alignof(DWORD));
    item->style = BS_PUSHBUTTON | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 85;
    item->y = 35;
    item->cx = 75;
    item->cy = 10;
    item->id = IDC_ADD_STAFF_EDIT_SCALE;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0xffff;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0x80;
    wchar_t edit_scale_string[] = L"Edit selected scale";
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(edit_scale_string)),
        edit_scale_string, sizeof(edit_scale_string));
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    ++ADD_STAFF_DIALOG_TEMPLATE->cdit;
    item = allocate_stack_slot(&main_window_memory->misc_stack, sizeof(DLGITEMTEMPLATE),
        _alignof(DWORD));
    item->style = BS_PUSHBUTTON | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 85;
    item->y = 45;
    item->cx = 75;
    item->cy = 10;
    item->id = IDC_ADD_STAFF_REMOVE_SCALE;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0xffff;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0x80;
    wchar_t remove_scale_string[] = L"Remove selected scale";
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(remove_scale_string)),
        remove_scale_string, sizeof(remove_scale_string));
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    EDIT_STAFF_SCALE_DIALOG_TEMPLATE = allocate_stack_slot(&main_window_memory->misc_stack,
        sizeof(DLGTEMPLATE), _alignof(DWORD));
    EDIT_STAFF_SCALE_DIALOG_TEMPLATE->style = DS_CENTER | DS_SETFONT;
    EDIT_STAFF_SCALE_DIALOG_TEMPLATE->dwExtendedStyle = 0;
    EDIT_STAFF_SCALE_DIALOG_TEMPLATE->cdit = 0;
    EDIT_STAFF_SCALE_DIALOG_TEMPLATE->x = 0;
    EDIT_STAFF_SCALE_DIALOG_TEMPLATE->y = 0;
    EDIT_STAFF_SCALE_DIALOG_TEMPLATE->cx = 70;
    EDIT_STAFF_SCALE_DIALOG_TEMPLATE->cy = 70;
    *(uint32_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint32_t)) = 0;
    wchar_t edit_staff_scale_dialog_title[] = L"Edit Staff Scale";
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(edit_staff_scale_dialog_title)),
        edit_staff_scale_dialog_title, sizeof(edit_staff_scale_dialog_title));
    memcpy(extend_array(&main_window_memory->misc_stack, font_info_length), font_info,
        font_info_length);
    ++EDIT_STAFF_SCALE_DIALOG_TEMPLATE->cdit;
    item = allocate_stack_slot(&main_window_memory->misc_stack, sizeof(DLGITEMTEMPLATE),
        _alignof(DWORD));
    item->style = BS_PUSHBUTTON | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 35;
    item->y = 55;
    item->cx = 30;
    item->cy = 10;
    item->id = IDCANCEL;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0xffff;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0x80;
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(cancel_string)), cancel_string,
        sizeof(cancel_string));
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    ++EDIT_STAFF_SCALE_DIALOG_TEMPLATE->cdit;
    item = allocate_stack_slot(&main_window_memory->misc_stack, sizeof(DLGITEMTEMPLATE),
        _alignof(DWORD));
    item->style = BS_PUSHBUTTON | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 5;
    item->y = 55;
    item->cx = 30;
    item->cy = 10;
    item->id = IDOK;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0xffff;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0x80;
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(ok_string)), ok_string,
        sizeof(ok_string));
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    ++EDIT_STAFF_SCALE_DIALOG_TEMPLATE->cdit;
    item = allocate_stack_slot(&main_window_memory->misc_stack, sizeof(DLGITEMTEMPLATE),
        _alignof(DWORD));
    item->style = SS_LEFT | WS_CHILD | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 5;
    item->y = 5;
    item->cx = 60;
    item->cy = 10;
    item->id = 0;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0xffff;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0x82;
    wchar_t name_string[] = L"Name:";
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(name_string)), name_string,
        sizeof(name_string));
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    ++EDIT_STAFF_SCALE_DIALOG_TEMPLATE->cdit;
    item = allocate_stack_slot(&main_window_memory->misc_stack, sizeof(DLGITEMTEMPLATE),
        _alignof(DWORD));
    item->style = WS_BORDER | WS_CHILD | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 5;
    item->y = 15;
    item->cx = 60;
    item->cy = 10;
    item->id = IDC_EDIT_STAFF_SCALE_NAME;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0xffff;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0x81;
    *(wchar_t*)extend_array(&main_window_memory->misc_stack, sizeof(wchar_t)) = 0;
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    ++EDIT_STAFF_SCALE_DIALOG_TEMPLATE->cdit;
    item = allocate_stack_slot(&main_window_memory->misc_stack, sizeof(DLGITEMTEMPLATE),
        _alignof(DWORD));
    item->style = SS_LEFT | WS_CHILD | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 5;
    item->y = 25;
    item->cx = 60;
    item->cy = 10;
    item->id = 0;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0xffff;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0x82;
    *(wchar_t*)extend_array(&main_window_memory->misc_stack, sizeof(wchar_t)) = 0;
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    ++EDIT_STAFF_SCALE_DIALOG_TEMPLATE->cdit;
    item = allocate_stack_slot(&main_window_memory->misc_stack, sizeof(DLGITEMTEMPLATE),
        _alignof(DWORD));
    item->style = WS_BORDER | WS_CHILD | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 5;
    item->y = 35;
    item->cx = 60;
    item->cy = 10;
    item->id = IDC_EDIT_STAFF_SCALE_VALUE;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0xffff;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0x81;
    *(wchar_t*)extend_array(&main_window_memory->misc_stack, sizeof(wchar_t)) = 0;
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    REMAP_STAFF_SCALE_DIALOG_TEMPLATE = allocate_stack_slot(&main_window_memory->misc_stack,
        sizeof(DLGTEMPLATE), _alignof(DWORD));
    REMAP_STAFF_SCALE_DIALOG_TEMPLATE->style = DS_CENTER | DS_SETFONT;
    REMAP_STAFF_SCALE_DIALOG_TEMPLATE->dwExtendedStyle = 0;
    REMAP_STAFF_SCALE_DIALOG_TEMPLATE->cdit = 0;
    REMAP_STAFF_SCALE_DIALOG_TEMPLATE->x = 0;
    REMAP_STAFF_SCALE_DIALOG_TEMPLATE->y = 0;
    REMAP_STAFF_SCALE_DIALOG_TEMPLATE->cx = 125;
    REMAP_STAFF_SCALE_DIALOG_TEMPLATE->cy = 85;
    *(uint32_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint32_t)) = 0;
    wchar_t remap_staff_scale_dialog_title[] = L"Remap Staff Scale";
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(remap_staff_scale_dialog_title)),
        remap_staff_scale_dialog_title, sizeof(remap_staff_scale_dialog_title));
    memcpy(extend_array(&main_window_memory->misc_stack, font_info_length), font_info,
        font_info_length);
    ++REMAP_STAFF_SCALE_DIALOG_TEMPLATE->cdit;
    item = allocate_stack_slot(&main_window_memory->misc_stack, sizeof(DLGITEMTEMPLATE),
        _alignof(DWORD));
    item->style = BS_PUSHBUTTON | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 60;
    item->y = 70;
    item->cx = 30;
    item->cy = 10;
    item->id = IDCANCEL;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0xffff;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0x80;
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(cancel_string)), cancel_string,
        sizeof(cancel_string));
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    ++REMAP_STAFF_SCALE_DIALOG_TEMPLATE->cdit;
    item = allocate_stack_slot(&main_window_memory->misc_stack, sizeof(DLGITEMTEMPLATE),
        _alignof(DWORD));
    item->style = BS_PUSHBUTTON | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 30;
    item->y = 70;
    item->cx = 30;
    item->cy = 10;
    item->id = IDOK;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0xffff;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0x80;
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(ok_string)), ok_string,
        sizeof(ok_string));
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    ++REMAP_STAFF_SCALE_DIALOG_TEMPLATE->cdit;
    item = allocate_stack_slot(&main_window_memory->misc_stack, sizeof(DLGITEMTEMPLATE),
        _alignof(DWORD));
    item->style = SS_LEFT | WS_CHILD | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 5;
    item->y = 5;
    item->cx = 115;
    item->cy = 35;
    item->id = 0;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0xffff;
    *(uint16_t*)extend_array(&main_window_memory->misc_stack, sizeof(uint16_t)) = 0x82;
    wchar_t remap_scale_string[] = L"One or more existing staves use the scale marked for "
        "deletion. Choose a new scale for these staves.";
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(remap_scale_string)),
        remap_scale_string, sizeof(remap_scale_string));
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    ++REMAP_STAFF_SCALE_DIALOG_TEMPLATE->cdit;
    item = allocate_stack_slot(&main_window_memory->misc_stack, sizeof(DLGITEMTEMPLATE),
        _alignof(DWORD));
    item->style = CBS_DROPDOWNLIST | CBS_HASSTRINGS | WS_CHILD | WS_VISIBLE;
    item->dwExtendedStyle = 0;
    item->x = 5;
    item->y = 40;
    item->cx = 110;
    item->cy = 100;
    item->id = IDC_REMAP_STAFF_SCALE_LIST;
    memcpy(extend_array(&main_window_memory->misc_stack, sizeof(combobox_string)), combobox_string,
        sizeof(combobox_string));
    *(wchar_t*)extend_array(&main_window_memory->misc_stack, sizeof(wchar_t)) = 0;
    *(WORD*)allocate_stack_slot(&main_window_memory->misc_stack, sizeof(WORD), _alignof(WORD)) = 0;
    ShowWindow(main_window_handle, SW_MAXIMIZE);
    return main_window_handle;
}

int WINAPI wWinMain(HINSTANCE instance_handle, HINSTANCE previous_instance_handle,
    PWSTR command_line, int nCmdShow)
{
    struct Project main_window_memory;
    HWND main_window_handle = init(instance_handle, &main_window_memory);
    MSG msg;
    while (GetMessage(&msg, main_window_handle, 0, 0) > 0)
    {
        TranslateMessage(&msg);
        DispatchMessage(&msg);
    }
    return 0;
}