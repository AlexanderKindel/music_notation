#include "memory.h"
#include "rational.h"
#include "duration.h"

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
void add_object_to_slice(struct ObjectIter*iter, struct Project*project, struct Slice*slice,
    uint32_t staff_index);
void insert_slice_object_before_iter(struct ObjectIter*iter, struct Project*project,
    struct Slice*slice, uint32_t staff_index);
void remove_object_from_slice(struct ObjectIter*iter, struct Project*project);
void remove_object_at_iter(struct ObjectIter*iter, struct Project*project);
void remove_object_tree_at_iter(struct ObjectIter*iter, struct Project*project);
void delete_object(struct Object*object, struct Project*project);
bool object_is_header(struct Object*object);
void cancel_selection(HWND main_window_handle);
void reset_accidental_displays(struct ObjectIter*iter, struct Project*project,
    uint8_t*key_sig_accidentals);
void reset_accidental_displays_from_previous_key_sig(struct Object*object, struct Project*project);
void get_key_sig(struct KeySig*out, struct Project*project);
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