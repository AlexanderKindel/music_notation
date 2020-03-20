#include "content.c"
#include "display.c"
#include "gui.c"

HWND init(HINSTANCE instance_handle, struct Project*main_window_memory)
{
    COLORREF gray = RGB(127, 127, 127);
    GRAY_PEN = CreatePen(PS_SOLID, 1, gray);
    CUSTOM_GRAY_BRUSH = CreateSolidBrush(gray);
    RED_PEN = CreatePen(PS_SOLID, 1, RED);
    RED_BRUSH = CreateSolidBrush(RED);
    SYSTEM_INFO system_info;
    GetSystemInfo(&system_info);
    PAGE_SIZE = system_info.dwPageSize;
    AddFontResourceExW(L"Bravura.otf", FR_PRIVATE, 0);
    initialize_pool(&main_window_memory->page_pool, PAGE_SIZE * UINT32_MAX, PAGE_SIZE);
    initialize_pool(&main_window_memory->staff_pool, sizeof(struct Staff) * UINT32_MAX,
        sizeof(struct Staff));
    for (uint32_t i = 0; i < sizeof(pool_element_sizes) / sizeof(uint32_t); ++i)
    {
        initialize_pool(&main_window_memory->other_pools[i], pool_element_sizes[i] * UINT32_MAX,
            pool_element_sizes[i]);
    }
    initialize_stack(&main_window_memory->stack_a, PAGE_SIZE);
    initialize_stack(&main_window_memory->stack_b, PAGE_SIZE);
    main_window_memory->selection.selection_type = SELECTION_NONE;
    main_window_memory->slices =
        initialize_page_list(&main_window_memory->page_pool, sizeof(struct Slice));
    struct SliceIter slice_iter = { 0, main_window_memory->slices,
        (struct Slice*)(main_window_memory->slices->bytes) };
    insert_slice_before_iter(&slice_iter, main_window_memory);
    STAFF_START_SLICE_ADDRESS = slice_iter.slice->address;
    slice_iter.slice->uz_distance_from_previous_slice = 20;
    slice_iter.slice->first_object_address_node_index = 0;
    slice_iter.slice->whole_notes_long.numerator =
        initialize_pool_integer(&INTEGER_POOL(main_window_memory), 0);
    slice_iter.slice->whole_notes_long.denominator =
        initialize_pool_integer(&INTEGER_POOL(main_window_memory), 1);
    slice_iter.slice->needs_respacing = false;
    increment_page_element_iter(&slice_iter.base, &main_window_memory->page_pool,
        sizeof(struct Slice));
    insert_slice_before_iter(&slice_iter, main_window_memory);
    HEADER_CLEF_SLICE_ADDRESS = slice_iter.slice->address;
    slice_iter.slice->uz_distance_from_previous_slice = 0;
    slice_iter.slice->first_object_address_node_index = 0;
    slice_iter.slice->whole_notes_long.denominator = 0;
    increment_page_element_iter(&slice_iter.base, &main_window_memory->page_pool,
        sizeof(struct Slice));
    insert_slice_before_iter(&slice_iter, main_window_memory);
    HEADER_KEY_SIG_SLICE_ADDRESS = slice_iter.slice->address;
    slice_iter.slice->uz_distance_from_previous_slice = 0;
    slice_iter.slice->first_object_address_node_index = 0;
    slice_iter.slice->whole_notes_long.denominator = 0;
    increment_page_element_iter(&slice_iter.base, &main_window_memory->page_pool,
        sizeof(struct Slice));
    insert_slice_before_iter(&slice_iter, main_window_memory);
    HEADER_TIME_SIG_SLICE_ADDRESS = slice_iter.slice->address;
    slice_iter.slice->uz_distance_from_previous_slice = 0;
    slice_iter.slice->first_object_address_node_index = 0;
    slice_iter.slice->whole_notes_long.denominator = 0;
    increment_page_element_iter(&slice_iter.base, &main_window_memory->page_pool,
        sizeof(struct Slice));
    insert_slice_before_iter(&slice_iter, main_window_memory);
    BODY_START_SLICE_ADDRESS = slice_iter.slice->address;
    slice_iter.slice->uz_distance_from_previous_slice = 0;
    slice_iter.slice->first_object_address_node_index = 0;
    slice_iter.slice->whole_notes_long.numerator =
        initialize_pool_integer(&INTEGER_POOL(main_window_memory), 0);
    slice_iter.slice->whole_notes_long.denominator =
        initialize_pool_integer(&INTEGER_POOL(main_window_memory), 1);
    main_window_memory->staff_scales =
        initialize_page_list(&main_window_memory->page_pool, sizeof(struct StaffScale));
    struct StaffScaleIter scale_iter = { 0, main_window_memory->staff_scales,
        (struct StaffScale*)(main_window_memory->staff_scales->bytes) };
    insert_page_element_before_iter(&scale_iter.base, main_window_memory,
        sizeof(struct StaffScale));
    scale_iter.scale->value = 1.0;
    wchar_t default_string[] = L"Default";
    memcpy(scale_iter.scale->name, default_string, sizeof(default_string));
    wchar_t default_value_string[] = L"1.0";
    memcpy(scale_iter.scale->value_string, default_value_string, sizeof(default_value_string));
    increment_page_element_iter(&scale_iter.base, &main_window_memory->page_pool,
        sizeof(struct StaffScale));
    insert_page_element_before_iter(&scale_iter.base, main_window_memory,
        sizeof(struct StaffScale));
    scale_iter.scale->value = 0.75;
    wchar_t cue_string[] = L"Cue";
    memcpy(scale_iter.scale->name, cue_string, sizeof(cue_string));
    wchar_t cue_value_string[] = L"0.75";
    memcpy(scale_iter.scale->value_string, cue_value_string, sizeof(cue_value_string));
    main_window_memory->ghost_cursor_address.object_address = 0;
    InitCommonControlsEx(&(INITCOMMONCONTROLSEX){ sizeof(INITCOMMONCONTROLSEX),
        ICC_BAR_CLASSES | ICC_STANDARD_CLASSES | ICC_TAB_CLASSES | ICC_UPDOWN_CLASS });
    RegisterClassW(&(WNDCLASSW){ CS_HREDRAW | CS_OWNDC, main_window_proc, 0,
        sizeof(struct Project*), instance_handle, 0, LoadCursorA(0, IDC_ARROW),
        (HBRUSH)(COLOR_WINDOW + 1), 0, L"main" });
    HWND main_window_handle = CreateWindowExW(0, L"main", L"Music Notation",
        WS_OVERLAPPEDWINDOW | WS_VISIBLE, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT,
        CW_USEDEFAULT, 0, 0, instance_handle, 0);
    NONCLIENTMETRICSW non_client_metrics;
    non_client_metrics.cbSize = sizeof(NONCLIENTMETRICSW);
    SystemParametersInfoW(SPI_GETNONCLIENTMETRICS, non_client_metrics.cbSize, &non_client_metrics,
        0);
    TEXT_FONT = CreateFontIndirectW(&non_client_metrics.lfMessageFont);
    HDC device_context = GetDC(main_window_handle);
    HFONT old_font = (HFONT)SelectObject(device_context, TEXT_FONT);
    TEXTMETRIC text_metrics;
    GetTextMetrics(device_context, &text_metrics);
    TEXT_FONT_HEIGHT = text_metrics.tmHeight;
    TEXT_CONTROL_X_BUFFER = 2 * get_character_width(device_context, TEXT_FONT, ' ');
    BUTTON_HEIGHT = (14 * text_metrics.tmAscent) / 8;
    RELATED_CONTROL_SPACER = (7 * text_metrics.tmAscent) / 8;
    UNRELATED_CONTROL_SPACER = (11 * text_metrics.tmAscent) / 8;
    COMMAND_BUTTON_WIDTH = (50 * text_metrics.tmAveCharWidth) / 4;
    LABEL_SPACER = (5 * text_metrics.tmAscent) / 8 - text_metrics.tmDescent;
    LABEL_SPACER = MAX(LABEL_SPACER, 0);
    main_window_memory->control_tabs_handle = CreateWindowExW(0, L"SysTabControl32", 0,
        WS_CHILD | WS_VISIBLE, 0, 0, 0, 0, main_window_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->control_tabs_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    int32_t tab_top = 25;
    int32_t tab_height = CONTROL_TABS_HEIGHT - tab_top;
    SendMessageW(main_window_memory->control_tabs_handle, TCM_INSERTITEMW, STAFF_TAB_INDEX,
        (LPARAM)&(TCITEMW){ TCIF_TEXT, 0, 0, L"Staves", 0, -1, 0 });
    main_window_memory->staff_tab_handle = CreateWindowExW(0, L"static", 0, WS_CHILD | WS_VISIBLE,
        0, tab_top, 500, tab_height, main_window_memory->control_tabs_handle, 0, instance_handle,
        0);
    SetWindowSubclass(main_window_memory->staff_tab_handle, staff_tab_proc, 0, 0);
    wchar_t add_staff_string[] = L"Add staff";
    int32_t add_staff_width =
        GET_TEXT_WIDTH(device_context, add_staff_string) + TEXT_CONTROL_X_BUFFER;
    main_window_memory->add_staff_button_handle = CreateWindowExW(0, L"button", add_staff_string,
        BS_PUSHBUTTON | BS_VCENTER | WS_CHILD | WS_VISIBLE, UNRELATED_CONTROL_SPACER,
        UNRELATED_CONTROL_SPACER, add_staff_width, BUTTON_HEIGHT,
        main_window_memory->staff_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->add_staff_button_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    main_window_memory->edit_staff_scales_button_handle = CreateWindowExW(0, L"button",
        EDIT_SCALES_STRING, BS_PUSHBUTTON | BS_VCENTER | WS_CHILD | WS_VISIBLE,
        2 * UNRELATED_CONTROL_SPACER + add_staff_width, UNRELATED_CONTROL_SPACER,
        GET_TEXT_WIDTH(device_context, EDIT_SCALES_STRING), BUTTON_HEIGHT,
        main_window_memory->staff_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->edit_staff_scales_button_handle, WM_SETFONT, (WPARAM)TEXT_FONT,
        0);
    SelectObject(device_context, old_font);
    ReleaseDC(main_window_handle, device_context);
    SendMessageW(main_window_memory->control_tabs_handle, TCM_INSERTITEMW, CLEF_TAB_INDEX,
        (LPARAM)&(TCITEMW){ TCIF_TEXT, 0, 0, L"Clefs", 0, -1, 0 });
    main_window_memory->clef_tab_handle = CreateWindowExW(0, L"static", 0, WS_CHILD, 0, tab_top,
        500, tab_height, main_window_memory->control_tabs_handle, 0, instance_handle, 0);
    SetWindowSubclass(main_window_memory->clef_tab_handle, clef_tab_proc, 0, 0);
    HWND clef_shape_label_handle = CreateWindowExW(0, L"static", L"Shape:",
        SS_LEFT | WS_CHILD | WS_VISIBLE, 5, 0, 50, 20, main_window_memory->clef_tab_handle, 0,
        instance_handle, 0);
    SendMessageW(clef_shape_label_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    main_window_memory->c_clef_handle = CreateWindowExW(0, L"button", L"C",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_GROUP | WS_VISIBLE, 60, 0, 35, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->c_clef_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    main_window_memory->f_clef_handle = CreateWindowExW(0, L"button", L"F",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 115, 0, 35, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->f_clef_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    main_window_memory->g_clef_handle = CreateWindowExW(0, L"button", L"G",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 170, 0, 35, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->g_clef_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    SendMessageW(main_window_memory->g_clef_handle, BM_SETCHECK, BST_CHECKED, 0);
    HWND unpitched_clef_handle = CreateWindowExW(0, L"button", L"Unpitched",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 225, 0, 75, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(unpitched_clef_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    HWND clef_octave_label_handle = CreateWindowExW(0, L"static", L"Octave:",
        SS_LEFT | WS_CHILD | WS_VISIBLE, 5, 20, 50, 20, main_window_memory->clef_tab_handle, 0,
        instance_handle, 0);
    SendMessageW(clef_octave_label_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    main_window_memory->clef_15ma_handle = CreateWindowExW(0, L"button", L"15ma",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_GROUP | WS_VISIBLE, 60, 20, 50, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->clef_15ma_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    main_window_memory->clef_8va_handle = CreateWindowExW(0, L"button", L"8va",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 115, 20, 50, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->clef_8va_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    main_window_memory->clef_none_handle = CreateWindowExW(0, L"button", L"None",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 170, 20, 50, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->clef_none_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    SendMessageW(main_window_memory->clef_none_handle, BM_SETCHECK, BST_CHECKED, 0);
    main_window_memory->clef_8vb_handle = CreateWindowExW(0, L"button", L"8vb",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 225, 20, 50, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->clef_8vb_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    main_window_memory->clef_15mb_handle = CreateWindowExW(0, L"button", L"15ma",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_VISIBLE, 280, 20, 50, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->clef_15mb_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    main_window_memory->add_clef_button_handle = CreateWindowExW(0, L"button", L"Add clef",
        BS_PUSHBUTTON | WS_DISABLED | WS_CHILD | WS_VISIBLE | BS_VCENTER, 335, 10, 55, 20,
        main_window_memory->clef_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->add_clef_button_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    SendMessageW(main_window_memory->control_tabs_handle, TCM_INSERTITEMW, KEY_SIG_TAB_INDEX,
        (LPARAM)&(TCITEMW){ TCIF_TEXT, 0, 0, L"Key Sigs", 0, -1, 0 });
    main_window_memory->key_sig_tab_handle = CreateWindowExW(0, L"static", 0, WS_CHILD, 0, tab_top,
        500, tab_height, main_window_memory->control_tabs_handle, 0, instance_handle, 0);
    SetWindowSubclass(main_window_memory->key_sig_tab_handle, key_sig_tab_proc, 0, 0);
    HWND accidental_count_label_handle = CreateWindowExW(0, L"static", L"Accidental count:",
        SS_LEFT | WS_CHILD | WS_VISIBLE, 5, 10, 95, 20, main_window_memory->key_sig_tab_handle, 0,
        instance_handle, 0);
    SendMessageW(accidental_count_label_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    HWND accidental_count_display_handle = CreateWindowExW(0, L"static", 0,
        WS_BORDER | WS_CHILD | WS_VISIBLE, 105, 10, 30, 20, main_window_memory->key_sig_tab_handle,
        0, instance_handle, 0);
    SendMessageW(accidental_count_display_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    main_window_memory->accidental_count_spin_handle = CreateWindowExW(0, UPDOWN_CLASSW, 0,
        UDS_ALIGNRIGHT | UDS_AUTOBUDDY | UDS_SETBUDDYINT | WS_CHILD | WS_VISIBLE, 0, 0, 0, 0,
        main_window_memory->key_sig_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->accidental_count_spin_handle, UDM_SETRANGE32, 0, 7);
    main_window_memory->sharps_handle = CreateWindowExW(0, L"button", L"Sharps",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_DISABLED | WS_GROUP | WS_VISIBLE, 150, 0, 55, 20,
        main_window_memory->key_sig_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->sharps_handle, BM_SETCHECK, BST_CHECKED, 0);
    SendMessageW(main_window_memory->sharps_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    main_window_memory->flats_handle = CreateWindowExW(0, L"button", L"Flats",
        BS_AUTORADIOBUTTON | WS_CHILD | WS_DISABLED | WS_VISIBLE, 150, 20, 55, 20,
        main_window_memory->key_sig_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->flats_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    main_window_memory->add_key_sig_button_handle = CreateWindowExW(0, L"button",
        L"Add key signature", BS_PUSHBUTTON | BS_VCENTER | WS_DISABLED | WS_CHILD | WS_VISIBLE, 215,
        10, 105, 20, main_window_memory->key_sig_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->control_tabs_handle, TCM_INSERTITEMW, TIME_SIG_TAB_INDEX,
        (LPARAM)&(TCITEMW){ TCIF_TEXT, 0, 0, L"Time sigs", 0, -1, 0 });
    SendMessageW(main_window_memory->add_key_sig_button_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    main_window_memory->time_sig_tab_handle = CreateWindowExW(0, L"static", 0, WS_CHILD, 0, tab_top,
        500, tab_height, main_window_memory->control_tabs_handle, 0, instance_handle, 0);
    SetWindowSubclass(main_window_memory->time_sig_tab_handle, time_sig_tab_proc, 0, 0);
    HWND numerator_label_handle = CreateWindowExW(0, L"static", L"Numerator:",
        SS_LEFT | WS_CHILD | WS_VISIBLE, 5, 0, 90, 20, main_window_memory->time_sig_tab_handle, 0,
        instance_handle, 0);
    SendMessageW(numerator_label_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    HWND numerator_display_handle = CreateWindowExW(0, L"static", 0,
        WS_BORDER | WS_CHILD | WS_VISIBLE, 90, 0, 45, 20, main_window_memory->time_sig_tab_handle,
        0, instance_handle, 0);
    SendMessageW(numerator_display_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    main_window_memory->numerator_spin_handle = CreateWindowExW(0, UPDOWN_CLASSW, 0,
        UDS_ALIGNRIGHT | UDS_AUTOBUDDY | UDS_SETBUDDYINT | WS_CHILD | WS_VISIBLE, 0, 0, 0, 0,
        main_window_memory->time_sig_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->numerator_spin_handle, UDM_SETRANGE32, 0, 100);
    SendMessageW(main_window_memory->numerator_spin_handle, UDM_SETPOS32, 0, 4);
    HWND denominator_label_handle = CreateWindowExW(0, L"static", L"Denominator:",
        SS_LEFT | WS_CHILD | WS_VISIBLE, 5, 20, 90, 20, main_window_memory->time_sig_tab_handle, 0,
        instance_handle, 0);
    SendMessageW(denominator_label_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    main_window_memory->denominator_display_handle = CreateWindowExW(0, L"static", L"4",
        WS_BORDER | WS_CHILD | WS_VISIBLE, 90, 20, 45, 20, main_window_memory->time_sig_tab_handle,
        0, instance_handle, 0);
    SendMessageW(main_window_memory->denominator_display_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    main_window_memory->denominator_spin_handle = CreateWindowExW(0, UPDOWN_CLASSW, 0,
        UDS_ALIGNRIGHT | UDS_AUTOBUDDY | WS_CHILD | WS_VISIBLE, 0, 0, 0, 0,
        main_window_memory->time_sig_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->denominator_spin_handle, UDM_SETRANGE32, MIN_LOG2_DURATION, 0);
    SendMessageW(main_window_memory->denominator_spin_handle, UDM_SETPOS32, 0, -2);
    main_window_memory->add_time_sig_button_handle = CreateWindowExW(0, L"button",
        L"Add time signature", BS_PUSHBUTTON | BS_VCENTER | WS_DISABLED | WS_CHILD | WS_VISIBLE,
        145, 10, 115, 20, main_window_memory->time_sig_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->add_time_sig_button_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
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
    SendMessageW(duration_label_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    main_window_memory->duration_display_handle = CreateWindowExW(0, L"static", L"quarter",
        WS_BORDER | WS_CHILD | WS_VISIBLE, x, label_height, 110, label_height,
        main_window_memory->note_tab_handle, 0, instance_handle, 0);
    SendMessageW(main_window_memory->duration_display_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
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
    SendMessageW(augmentation_dot_label_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
    HWND augmentation_dot_display_handle = CreateWindowExW(0, L"static", 0,
        WS_BORDER | WS_VISIBLE | WS_CHILD, x, label_height, 110, 20,
        main_window_memory->note_tab_handle, 0, instance_handle, 0);
    SendMessageW(augmentation_dot_display_handle, WM_SETFONT, (WPARAM)TEXT_FONT, 0);
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