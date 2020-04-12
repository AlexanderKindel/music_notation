#ifndef SHARED_DECLARATIONS_H
#define SHARED_DECLARATIONS_H

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <windows.h>
#include <commctrl.h>

struct DialogTemplate
{
    DLGTEMPLATE header;
    wchar_t body[];
};

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

#define STAFF_OBJECT_ADDRESS_IS_NON_NULL(address) (address).object_address
#define NULL_STAFF_OBJECT_ADDRESS(address) (address).object_address = 0

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
    uint16_t rod_intersection_count;
    bool needs_respacing;
};

#define SLICE_IS_RHYTHMIC(slice_pointer) slice_pointer->whole_notes_long.denominator

struct StaffScale
{
    uint32_t address;
    float value;
    wchar_t value_string[16];
    wchar_t name[16];
};

struct StaffScaleArray
{
    size_t count;
    size_t max_count_reached;
    struct StaffScale*scales;
    void*max_allowed_address;
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
    uint32_t scale_address;
    uint8_t line_count;
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

size_t get_integer_size(size_t value_count)
{
    return sizeof(uint32_t) * (1 + value_count);
}

wchar_t OK_STRING[] = L"OK";
wchar_t CANCEL_STRING[] = L"Cancel";
wchar_t EDIT_SCALES_STRING[] = L"Edit scale list";

#define IDC_EDIT_SCALES_SCALE_LIST 8
#define IDC_EDIT_SCALES_ADD_SCALE 9
#define IDC_EDIT_SCALES_EDIT_SCALE 10
#define IDC_EDIT_SCALES_REMOVE_SCALE 11

wchar_t EDIT_SCALES_ADD_SCALE_STRING[] = L"Add new scale";
wchar_t EDIT_SCALES_EDIT_SCALE_STRING[] = L"Edit selected scale";
wchar_t EDIT_SCALES_REMOVE_SCALE_STRING[] = L"Remove selected scale";

#define IDC_ADD_STAFF_LINE_COUNT_LABEL 8
#define IDC_ADD_STAFF_LINE_COUNT_SPIN 9
#define IDC_ADD_STAFF_LINE_COUNT_DISPLAY 10
#define IDC_ADD_STAFF_SCALE_LABEL 11
#define IDC_ADD_STAFF_SCALE_LIST 12
#define IDC_ADD_STAFF_EDIT_SCALES 14

wchar_t ADD_STAFF_LINE_COUNT_LABEL_STRING[] = L"Line count:";
wchar_t ADD_STAFF_SCALE_LABEL_STRING[] = L"Scale:";

#define IDC_EDIT_STAFF_SCALE_NAME_LABEL 8
#define IDC_EDIT_STAFF_SCALE_NAME 9
#define IDC_EDIT_STAFF_SCALE_VALUE_LABEL 10
#define IDC_EDIT_STAFF_SCALE_VALUE 11

wchar_t EDIT_STAFF_NAME_LABEL_STRING[] = L"Name:";
wchar_t EDIT_STAFF_VALUE_LABEL_STRING[] = L"Value:";

#define IDC_REMAP_STAFF_SCALE_MESSAGE 8
#define IDC_REMAP_STAFF_SCALE_LIST 9

#endif