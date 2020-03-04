#include "clef_tab.h"
#include "staff_tab.h"

int32_t CONTROL_TABS_HEIGHT = 65;

#define STAFF_TAB_INDEX 0
#define CLEF_TAB_INDEX 1
#define KEY_SIG_TAB_INDEX 2
#define TIME_SIG_TAB_INDEX 3
#define NOTE_TAB_INDEX 4

int32_t MAX_LOG2_DURATION = 1;
int32_t MIN_LOG2_DURATION = -10;

int32_t TEXT_FONT_HEIGHT;
int32_t BUTTON_HEIGHT;
int32_t RELATED_CONTROL_SPACER;
int32_t UNRELATED_CONTROL_SPACER;
int32_t COMMAND_BUTTON_WIDTH;
int32_t LABEL_SPACER;
int32_t TEXT_CONTROL_X_BUFFER;

int32_t get_text_width(HDC device_context, wchar_t*text, size_t text_length);

#define GET_TEXT_WIDTH(device_context, text)\
get_text_width(device_context, text, sizeof(text) / sizeof(wchar_t))

void center_dialog(HWND dialog_handle, int window_width, int window_height);
LRESULT key_sig_tab_proc(HWND window_handle, UINT message, WPARAM w_param, LPARAM l_param,
    UINT_PTR id_subclass, DWORD_PTR ref_data);
LRESULT note_tab_proc(HWND window_handle, UINT message, WPARAM w_param, LPARAM l_param,
    UINT_PTR id_subclass, DWORD_PTR ref_data);
void get_selected_time_sig(struct Project*project, struct TimeSig*out);
LRESULT time_sig_tab_proc(HWND window_handle, UINT message, WPARAM w_param, LPARAM l_param,
    UINT_PTR id_subclass, DWORD_PTR ref_data);
void enable_add_header_object_buttons(struct Project*project, BOOL enable);
LRESULT CALLBACK main_window_proc(HWND window_handle, UINT message, WPARAM w_param,
    LPARAM l_param);