#ifndef DECLARATIONS_H
#define DECLARATIONS_H

#include "bravura_metadata.h"
#include "generated_declarations.h"
#include <commctrl.h>
#include <math.h>
#include <wchar.h>
#include <windowsx.h>

struct BaseIter
{
    uint32_t element_index_on_page;
    struct Page*page;
};

struct PageElementIter
{
    uint32_t element_index_on_page;
    struct Page*page;
    void*element;
};

struct ObjectIter
{
    struct BaseIter base;
    struct Object*object;
};

struct SliceIter
{
    struct BaseIter base;
    struct Slice*slice;
};

struct PositionedSliceIter
{
    struct SliceIter iter;
    int32_t uz_slice_x;
};

struct FontSet
{
    HFONT full_size;
    HFONT two_thirds_size;
};

struct DisplayedAccidental
{
    uint8_t accidental;
    bool is_visible;
};

struct TimeSigStrings
{
    wchar_t buffer[8];
    wchar_t*numerator_string;
    wchar_t*denominator_string;
    uint8_t numerator_string_length;
    uint8_t denominator_string_length;
};

struct VerticalInterval
{
    int32_t top;
    int32_t bottom;
};

struct Division
{
    struct Integer*quotient;
    struct Integer*remainder;
};

#ifdef _DEBUG
#define ASSERT(condition, message) if (!(condition)) { puts(message); abort(); }
#else
#define ASSERT(condition, message)
#endif

#define MAX(a, b) (a > b) ? a : b

//content\memory
size_t PAGE_SIZE;

uintptr_t round_down_to_alignment(size_t alignment, uintptr_t value);
void*start_array(struct Stack*stack, size_t alignment);
void*extend_array(struct Stack*stack, size_t element_size);
void*allocate_stack_slot(struct Stack*stack, size_t slot_size, size_t alignment);
void initialize_pool(struct Pool*out, size_t size, void*start, uint32_t element_size);
void*allocate_pool_slot(struct Pool*pool);
void free_pool_slot(struct Pool*pool, void*slot);
void*resolve_pool_index(struct Pool*pool, uint32_t index);
void*resolve_address(struct Project*project, uint32_t address);
uint32_t get_element_index_in_pool(struct Pool*pool, void*element);
void*resolve_page_index(struct Page*page, uint32_t element_index, uint32_t element_size);
void initialize_page_element_iter(struct BaseIter*out, void*element, uint32_t element_size);
void increment_page_element_iter(struct BaseIter*iter, struct Pool*page_pool,
    uint32_t element_size);
void decrement_page_element_iter(struct BaseIter*iter, struct Pool*page_pool,
    uint32_t element_size);
void move_page_element(struct Project*project, void*new_location, void*old_location,
    uint32_t element_size);
struct Page*insert_new_page_after(struct Page*page, struct Pool*page_pool);
void insert_page_element_before_iter(struct BaseIter*iter, struct Project*project,
    uint32_t element_size);
void remove_page_element_at_iter(struct BaseIter*iter, struct Project*project,
    uint32_t element_size);

//content\rational
struct Integer*copy_integer_to_stack(struct Integer*a, struct Stack*stack);
struct Integer*copy_integer_to_persistent_memory(struct Integer*a, struct Project*project);
void free_integer_from_persistent_memory(struct Integer*a, struct Project*project);
void initialize_integer(struct Integer*out, uint32_t value);
struct Integer*initialize_stack_integer(struct Stack*out_stack, uint32_t value);
struct Integer*initialize_pool_integer(struct Pool*integer_pool, uint32_t value);
float integer_to_float(struct Integer*a);
void subtract_integer_from_a_in_place(struct Integer*a, struct Integer*b, struct Stack*stack);
struct Integer*multiply_integers(struct Integer*a, struct Integer*b, struct Stack*stack);
struct Integer*double_integer(struct Integer*a, struct Stack*stack);
void halve_integer_in_place(struct Integer*a);
void divide_integers(struct Division*out, struct Integer*dividend, struct Integer*divisor,
    struct Stack*stack);
uint8_t integer_to_wchar_string(wchar_t**out, uint16_t integer, wchar_t codepoint_of_zero,
    uint8_t buffer_length);
void copy_rational_to_persistent_memory(struct Project*project, struct Rational*source,
    struct Rational*destination);
void free_rational_from_persistent_memory(struct Project*project, struct Rational*a);
void subtract_rationals(struct Rational*out, struct Rational*minuend, struct Rational*subtrahend,
    struct Stack*stack);
int8_t compare_rationals(struct Rational*a, struct Rational*b, struct Stack*stack);

//content\content
#define OBJECT_ACCIDENTAL 0
#define OBJECT_BARLINE 1
#define OBJECT_CLEF 2
#define OBJECT_DURATION 3
#define OBJECT_KEY_SIG 4
#define OBJECT_NONE 5
#define OBJECT_TIME_SIG 6

uint32_t STAFF_START_SLICE_ADDRESS;
uint32_t HEADER_CLEF_SLICE_ADDRESS;
uint32_t HEADER_KEY_SIG_SLICE_ADDRESS;
uint32_t HEADER_TIME_SIG_SLICE_ADDRESS;
uint32_t BODY_START_SLICE_ADDRESS;

#define DOUBLE_FLAT 1
#define FLAT 2
#define NATURAL 3
#define SHARP 4
#define DOUBLE_SHARP 5

int8_t LETTER_NAME_B = 6;
int8_t LETTER_NAME_F = 3;

__declspec(noreturn) void crash(char*message);
int8_t clamped_add(int8_t augend, uint8_t addend);
int8_t clamped_subtract(int8_t minuend, uint8_t subtrahend);
void insert_sliceless_object_before_iter(struct ObjectIter*iter, struct Project*project);
void insert_slice_object_before_iter(struct ObjectIter*iter, struct Project*project,
    uint32_t slice_address, uint32_t staff_index);
void insert_rhythmic_slice_object_before_iter(struct ObjectIter*object_iter,
    struct SliceIter*slice_iter, struct Project*project,
    struct Rational*whole_notes_after_current_slice, uint32_t staff_index);
void remove_object_at_iter(struct ObjectIter*iter, struct Project*project);
void remove_object_tree_at_iter(struct ObjectIter*iter, struct Project*project);
void delete_object(struct Object*object, struct Project*project);
bool object_is_header(struct Object*object);
void get_whole_notes_long(struct Duration*duration, struct Rational*out, struct Stack*out_stack);
void overwrite_with_duration(struct Duration*duration, struct ObjectIter*iter,
    struct Project*project, uint32_t staff_index);
int8_t pitch_to_letter_name(int8_t pitch);
struct DisplayedAccidental get_default_accidental(struct Object*note, struct Project*project);
void cancel_selection(HWND main_window_handle);
void reset_accidental_displays(struct ObjectIter*iter, struct Project*project,
    uint8_t*key_sig_accidentals);
void reset_accidental_displays_from_previous_key_sig(struct Object*object, struct Project*project);
void get_key_sig(struct KeySig*out, bool is_flats);
void set_cursor_to_next_valid_state(struct Project*project);
struct Slice*get_next_slice_right_of_iter(struct ObjectIter*iter, struct Project*project);
struct Slice*get_next_slice_right_of_object(struct Object*object, struct Project*project);
void initialize_slice_iter_to_ut_leftmost_to_draw(struct PositionedSliceIter*out,
    struct Project*project);
void initialize_slice_iter_to_t_leftmost_to_draw(struct PositionedSliceIter*out,
    struct Project*project);
void increment_slice_iter(struct Pool*page_pool, struct PositionedSliceIter*iter);
void decrement_slice_iter(struct Pool*page_pool, struct PositionedSliceIter*iter);
struct Object*get_leftmost_staff_object_to_draw(struct PositionedSliceIter*iter,
    struct Project*project, uint32_t staff_index);
void insert_slice_before_iter(struct SliceIter*iter, struct Project*project);
struct Object*get_nth_object_on_staff(struct Project*project, uint32_t staff_index, uint8_t n);
int8_t get_staff_middle_pitch(struct Clef*clef);
int8_t get_staff_bottom_line_pitch(uint8_t line_count, int8_t middle_pitch);

//display\viewport
int8_t get_staff_middle_pitch_at_viewport_left_edge(struct Project*project, struct Staff*staff);
void reset_viewport_offset_x(HWND main_window_handle, struct Project*project,
    int32_t uz_new_offset_x);
void reset_viewport_offset_y(HWND main_window_handle, struct Project*project,
    int32_t uz_new_offset_y);

//display\display
COLORREF BLACK = RGB(0, 0, 0);
COLORREF RED = RGB(255, 0, 0);
COLORREF WHITE = RGB(255, 255, 255);
HPEN GRAY_PEN;
HBRUSH CUSTOM_GRAY_BRUSH;
HPEN RED_PEN;
HBRUSH RED_BRUSH;

int32_t DEFAULT_TOP_STAFF_MIDDLE_Y = 135;
float UZ_DISTANCE_BETWEEN_ACCIDENTAL_AND_NOTE = 0.2;
float UZ_DISTANCE_BETWEEN_AUGMENTATION_DOTS = 0.2;
float UZ_WHOLE_NOTE_WIDTH = 10.0;
float DURATION_RATIO;
float WHEEL_DELTA_SCALE = 8.0;

int32_t float_round(float a);
float get_zoom_factor(int8_t zoom_exponent);
int32_t zoom_coordinate(float tuz_coordinate, float zoom_factor);
int32_t unzoom_coordinate(float tz_coordinate, float zoom_factor);
void get_work_region_rect(HWND main_window_handle, RECT*out);
void invalidate_work_region(HWND main_window_handle);
HFONT get_staff_font(float staff_space_height, float staff_height_multiple);
void get_staff_font_set(struct FontSet*out, float staff_space_height);
void release_font_set(struct FontSet*font_set);
int32_t get_character_width(HDC device_context, HFONT font, uint32_t codepoint);
int32_t get_string_width(HDC device_context, HFONT font, wchar_t*string, uint8_t string_length);
void draw_character(HDC device_context, HFONT z_font, float tuz_x, float tuz_y, float zoom_factor,
    uint16_t codepoint);
struct VerticalInterval get_tz_horizontal_line_vertical_bounds(float tuz_vertical_center,
    float uz_thickness, float zoom_factor);
void draw_horizontal_line(HDC device_context, int32_t tuz_left_edge, int32_t tuz_right_edge,
    float tuz_vertical_center, float uz_thickness, float zoom_factor);
void draw_object(struct FontSet*z_font_set, HDC device_context, int8_t*staff_middle_pitch,
    struct Object*object, struct Project*project, struct Staff*staff, float uz_staff_space_height,
    float zoom_factor, int32_t tuz_staff_middle_y, int32_t tuz_x);
void draw_object_with_selection(struct FontSet*z_font_set, HDC device_context,
    int8_t*staff_middle_pitch, struct Object*object, struct Project*project, struct Staff*staff,
    float uz_staff_space_height, float zoom_factor, int32_t tuz_staff_middle_y, int32_t tuz_x);
uint32_t get_address_of_clicked_staff_object(HDC back_buffer_device_context, struct Project*project,
    struct Staff*staff, float zoom_factor, int32_t staff_middle_y, int32_t tz_mouse_x,
    int32_t tz_mouse_y);
uint16_t get_accidental_codepoint(uint8_t accidental);
uint16_t get_duration_codepoint(struct Duration*duration);
void time_sig_to_strings(struct TimeSigStrings*out, struct TimeSig time_sig);
int32_t uz_get_default_distance_from_object_origin_to_slice(float uz_staff_space_height,
    struct Object*object);
int32_t reset_distance_from_previous_slice(HDC device_context, struct Project*project,
    struct Slice*slice);
int32_t respace_slice_range_left_of_iter(HDC device_context, struct Project*project,
    struct SliceIter*iter);
void respace_onscreen_slices(HWND main_window_handle, struct Project*project);
void draw_staff(HDC device_context, struct Project*project, int32_t tuz_staff_middle_y,
    int32_t tuz_update_region_right_edge, uint32_t staff_index);
float get_tuz_y_of_staff_relative_step(int32_t tuz_staff_middle_y, float uz_staff_space_height,
    uint8_t staff_line_count, int8_t steps_above_bottom_line);
struct VerticalInterval get_tz_staff_vertical_bounds(float uz_staff_space_height, float zoom_factor,
    int32_t tuz_staff_middle_y, uint8_t staff_line_count);
struct StaffObjectAddress get_ghost_cursor_address(struct Project*project, int32_t tz_mouse_x,
    int32_t tz_mouse_y);

//gui\gui
int32_t CONTROL_TABS_HEIGHT = 65;

#define STAFF_TAB_INDEX 0
#define CLEF_TAB_INDEX 1
#define KEY_SIG_TAB_INDEX 2
#define TIME_SIG_TAB_INDEX 3
#define NOTE_TAB_INDEX 4

#define IDC_ADD_STAFF_LINE_COUNT_SPIN 8
#define IDC_ADD_STAFF_LINE_COUNT_DISPLAY 9
#define IDC_ADD_STAFF_SCALE_LIST 10
#define IDC_ADD_STAFF_ADD_SCALE 11
#define IDC_ADD_STAFF_EDIT_SCALE 12
#define IDC_ADD_STAFF_REMOVE_SCALE 13

#define IDC_EDIT_STAFF_SCALE_NAME 8
#define IDC_EDIT_STAFF_SCALE_VALUE 9

#define IDC_REMAP_STAFF_SCALE_LIST 8

DLGTEMPLATE*ADD_STAFF_DIALOG_TEMPLATE;
DLGTEMPLATE*EDIT_STAFF_SCALE_DIALOG_TEMPLATE;
DLGTEMPLATE*REMAP_STAFF_SCALE_DIALOG_TEMPLATE;

int32_t MAX_LOG2_DURATION = 1;
int32_t MIN_LOG2_DURATION = -10;

void size_dialog(HWND dialog_handle);
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

//gui\clef_tab
struct Clef get_selected_clef(struct Project*project);
LRESULT clef_tab_proc(HWND window_handle, UINT message, WPARAM w_param, LPARAM l_param,
    UINT_PTR id_subclass, DWORD_PTR ref_data);

//gui\staff_tab
LRESULT staff_tab_proc(HWND window_handle, UINT message, WPARAM w_param, LPARAM l_param,
    UINT_PTR id_subclass, DWORD_PTR ref_data);

#endif