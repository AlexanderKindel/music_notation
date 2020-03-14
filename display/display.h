#include "respace_slices.h"
#include "viewport.h"

COLORREF BLACK = RGB(0, 0, 0);
COLORREF RED = RGB(255, 0, 0);
COLORREF WHITE = RGB(255, 255, 255);
HPEN GRAY_PEN;
HBRUSH CUSTOM_GRAY_BRUSH;
HPEN RED_PEN;
HBRUSH RED_BRUSH;
HFONT TEXT_FONT;

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
void get_work_region_rect(HWND main_window_handle, struct Project*project, RECT*out);
void invalidate_work_region(HWND main_window_handle, struct Project*project);
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
void draw_staff(HDC device_context, struct Project*project, int32_t tuz_staff_middle_y,
    int32_t tuz_update_region_right_edge, uint32_t staff_index);
float get_tuz_y_of_staff_relative_step(int32_t tuz_staff_middle_y, float uz_staff_space_height,
    uint8_t staff_line_count, int8_t steps_above_bottom_line);
struct VerticalInterval get_tz_staff_vertical_bounds(float uz_staff_space_height, float zoom_factor,
    int32_t tuz_staff_middle_y, uint8_t staff_line_count);
struct StaffObjectAddress get_ghost_cursor_address(struct Project*project, int32_t tz_mouse_x,
    int32_t tz_mouse_y);