struct Integer*copy_integer_to_stack(struct Integer*a, struct Stack*stack);
struct Integer*copy_integer_to_persistent_memory(struct Integer*a, struct Project*project);
void free_integer_from_persistent_memory(struct Integer*a, struct Project*project);
void initialize_integer(struct Integer*out, uint32_t value);
struct Integer*initialize_stack_integer(struct Stack*out_stack, uint32_t value);
struct Integer*initialize_pool_integer(struct Pool*integer_pool, uint32_t value);
float integer_to_float(struct Integer*a);
void subtract_integer_from_a_in_place(struct Integer*a, struct Integer*b, struct Stack*local_stack);
struct Integer*multiply_integers(struct Integer*a, struct Integer*b, struct Stack*out_stack);
struct Integer*double_integer(struct Integer*a, struct Stack*out_stack);
void halve_integer_in_place(struct Integer*a);
void divide_integers(struct Division*out, struct Integer*dividend, struct Integer*divisor,
    struct Stack*out_stack, struct Stack*local_stack);
uint8_t integer_to_wchar_string(wchar_t**out, uint16_t integer, wchar_t codepoint_of_zero,
    uint8_t buffer_length);
void copy_rational_to_persistent_memory(struct Project*project, struct Rational*source,
    struct Rational*destination);
void free_rational_from_persistent_memory(struct Project*project, struct Rational*a);
void add_rationals(struct Rational*out, struct Rational*a, struct Rational*b,
    struct Stack*out_stack, struct Stack*local_stack);
void subtract_rationals(struct Rational*out, struct Rational*minuend, struct Rational*subtrahend,
    struct Stack*out_stack, struct Stack*local_stack);
int8_t compare_rationals(struct Rational*a, struct Rational*b, struct Stack*local_stack);