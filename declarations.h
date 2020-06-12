#ifndef DECLARATIONS_H
#define DECLARATIONS_H

#include "bravura_metadata.h"
#include "generated_declarations.h"
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

struct StaffScaleIter
{
    struct BaseIter base;
    struct StaffScale*scale;
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
#define ASSERT(condition, message) if (!(condition)) { puts(message); *(int*)0 = 1; }
#else
#define ASSERT(condition, message)
#endif

#define MAX(a, b) ((a > b) ? a : b)
#define MIN(a, b) ((a < b) ? a : b)
#define SWAP(a, b, type) { type temp = a; a = b; b = temp; }

#include "content.h"
#include "display.h"
#include "gui.h"

#endif