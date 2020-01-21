#ifndef BRAVURA_H
#define BRAVURA_H

struct FloatPoint
{
    float x;
    float y;
};

struct FontMetadata
{
    struct FloatPoint uz_black_notehead_stem_up_se;
    struct FloatPoint uz_black_notehead_stem_down_nw;
    struct FloatPoint uz_half_notehead_stem_up_se;
    struct FloatPoint uz_half_notehead_stem_down_nw;
    float uz_beam_spacing;
    float uz_beam_thickness;
    float uz_double_whole_notehead_x_offset;
    float uz_leger_line_extension;
    float uz_leger_line_thickness;
    float uz_staff_line_thickness;
    float uz_stem_thickness;
    float uz_thin_barline_thickness;
};

struct FontMetadata BRAVURA_METADATA = { { 1.18, 0.168 }, { 0.0, -0.168 }, { 1.18, 0.168 },
    { 0.0, -0.168 }, 0.25, 0.5, 0.36, 0.4, 0.16, 0.13, 0.12, 0.16 };

#endif