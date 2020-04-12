void get_whole_notes_long(struct Duration*duration, struct Rational*out, struct Stack*out_stack);
void overwrite_with_duration(struct Duration*duration, struct ObjectIter*iter,
    struct Project*project, uint32_t staff_index);
int8_t pitch_to_letter_name(int8_t pitch);
struct DisplayedAccidental get_default_accidental(struct Object*note, struct Project*project);