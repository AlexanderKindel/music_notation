#ifndef SHARED_DECLARATIONS_H
#define SHARED_DECLARATIONS_H

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <windows.h>

struct Stack
{
    void*start;
    void*end;
    void*cursor;
    void*cursor_max;
};

struct Pool
{
    void*start;
    void*end;
    void*cursor;
    uint32_t element_size;

    //The first uint32_t of each element is treated as the index of the next one on the free list.
    uint32_t index_of_first_free_element;
};

struct Page
{
    uint32_t next_page_index;
    uint32_t previous_page_index;
    uint32_t capacity;
    uint32_t occupied_slot_count;

    //Pages are always allocated on OS page boundaries, so since bytes is a multiple of 64 bits
    //from the beginning of the struct, it will always have the strictest alignment.
    uint8_t bytes[];
};

struct Clef
{
    uint16_t codepoint;
    int8_t steps_of_baseline_above_staff_middle;
};

struct Pitch
{
    uint8_t accidental;
    int8_t steps_above_c4;
};

struct NotePitch
{
    uint32_t accidental_object_address;
    struct Pitch pitch;
};

struct Duration
{
    struct NotePitch pitch;
    bool is_pitched;
    uint8_t augmentation_dot_count;
    int8_t log2;
};

struct KeySigAccidental
{
    uint8_t accidental : 3;
    uint8_t letter_name : 3;
};

struct KeySig
{
    int8_t floors[7];
    uint8_t accidental_count;
    struct KeySigAccidental accidentals[7];
};

struct TimeSig
{
    uint16_t numerator;
    uint16_t denominator;
};

struct Object
{
    uint32_t address;
    union
    {
        uint32_t accidental_note_address;
        struct Clef clef;
        struct Duration duration;
        struct KeySig key_sig;
        struct TimeSig time_sig;
    };
    int32_t uz_distance_to_next_slice;
    uint32_t slice_address;
    uint8_t object_type;
    bool is_selected;
    bool is_valid_cursor_position;
};

struct StaffObjectAddress
{
    uint32_t staff_index;
    uint32_t object_address;
};

struct AddressNode
{
    uint32_t index_of_next;
    struct StaffObjectAddress address;
};

struct Integer
{
    uint32_t value_count;
    uint32_t value[];
};

struct Rational
{
    struct Integer*numerator;
    struct Integer*denominator;
};

struct Slice
{
    uint32_t address;
    uint32_t first_object_address_node_index;

    //The Slice has no rhythmic alignment when whole_notes_long.denominator == 0.
    struct Rational whole_notes_long;
    int32_t uz_distance_from_previous_slice;
    bool needs_respacing;
};

struct StaffScale
{
    float value;
    wchar_t value_string[16];
    wchar_t name[16];
};

struct Staff
{
    int32_t uz_distance_from_staff_above;//From vertical center to vertical center.

    //Set to the header clef's address when the header clef is onscreen since there is no clef
    //beyond the leftmost slice to draw in that case.
    uint32_t address_of_clef_beyond_leftmost_slice_to_draw;
    uint32_t index_of_staff_above;
    uint32_t index_of_staff_below;
    uint32_t object_page_index;
    uint8_t line_count;
    uint8_t scale_index;
    bool is_on_free_list;
};

enum SelectionType
{
    SELECTION_CURSOR,
    SELECTION_OBJECT,
    SELECTION_NONE
};

struct Selection
{
    enum SelectionType selection_type;
    struct StaffObjectAddress address;
    int8_t range_floor;//When selection_type == SELECTION_CURSOR
};

#define MAX_STAFF_SCALE_COUNT 16

size_t get_integer_size(size_t value_count)
{
    return sizeof(uint32_t) * (1 + value_count);
}

#endif