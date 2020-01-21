#include "declarations.h"

struct Integer*allocate_integer(struct Stack*stack, size_t value_count)
{
    return allocate_stack_slot(stack, get_integer_size(value_count), _alignof(struct Integer));
}

struct Integer*copy_integer_to_stack(struct Integer*a, struct Stack*stack)
{
    struct Integer*out = allocate_integer(stack, a->value_count);
    memcpy(out, a, get_integer_size(a->value_count));
    return out;
}

struct Integer*copy_integer_to_persistent_memory(struct Integer*a, struct Project*project)
{
    struct Integer*out;
    size_t size = get_integer_size(a->value_count);
    if (a->value_count > 1)
    {
        out = malloc(size);
    }
    else
    {
        out = allocate_pool_slot(&INTEGER_POOL(project));
    }
    memcpy(out, a, size);
    return out;
}

void free_integer_from_persistent_memory(struct Integer*a, struct Project*project)
{
    if (a->value_count > 1)
    {
        free(a);
    }
    else
    {
        free_pool_slot(&INTEGER_POOL(project), a);
    }
}

void initialize_integer(struct Integer*out, uint32_t value)
{
    if (value)
    {
        out->value_count = 1;
        out->value[0] = value;
    }
    else
    {
        out->value_count = 0;
    }
}

struct Integer*initialize_stack_integer(struct Stack*out_stack, uint32_t value)
{
    struct Integer*out = allocate_integer(out_stack, 1);
    initialize_integer(out, value);
    return out;
}

struct Integer*initialize_pool_integer(struct Pool*integer_pool, uint32_t value)
{
    struct Integer*out = allocate_pool_slot(integer_pool);
    initialize_integer(out, value);
    return out;
}

float integer_to_float(struct Integer*a)
{
    if (a->value_count)
    {
        return a->value[0];
    }
    return 0.0;
}

void integer_trim_leading_zeroes(struct Integer*a)
{
    for (size_t i = a->value_count; i > 0; --i)
    {
        if (a->value[i - 1] != 0)
        {
            a->value_count = i;
            return;
        }
    }
    a->value_count = 0;
}

void add_b_value_to_a_in_place(struct Integer*a, struct Integer*b)
{
    for (uint32_t i = 0; i < b->value_count; ++i)
    {
        uint64_t remainder = b->value[i];
        for (uint32_t j = i; j < a->value_count; ++j)
        {
            uint64_t sum_value = a->value[j] + remainder;
            a->value[j] = (uint32_t)sum_value;
            remainder = (sum_value & 0xffffffff00000000) >> 32;
            if (remainder == 0)
            {
                break;
            }
        }
    }
}

void add_integer_to_a_in_place(struct Integer*a, struct Integer*b)
{
    if (b->value_count == 0)
    {
        return;
    }
    if (a->value_count == 0)
    {
        memcpy(a, b, get_integer_size(b->value_count));
        return;
    }
    if (a->value_count < b->value_count)
    {
        memset(&a->value[a->value_count], 0,
            (b->value_count - a->value_count + 1) * sizeof(uint32_t));
        a->value_count = b->value_count + 1;
    }
    else
    {
        a->value[a->value_count] = 0;
        a->value_count += 1;
    }
    add_b_value_to_a_in_place(a, b);
    integer_trim_leading_zeroes(a);
}

void subtract_integer_from_a_in_place(struct Integer*a, struct Integer*b, struct Stack*stack)
{
    if (b->value_count == 0)
    {
        return;
    }
    void*stack_savepoint = stack->cursor;
    struct Integer*twos_complement_of_b = allocate_integer(stack, b->value_count);
    twos_complement_of_b->value_count = b->value_count;
    for (uint32_t i = 0; i < b->value_count; ++i)
    {
        twos_complement_of_b->value[i] = ~b->value[i];
    }
    for (uint32_t i = 0; i < b->value_count; ++i)
    {
        uint32_t power = 1;
        for (uint32_t j = 0; j < 32; ++j)
        {
            twos_complement_of_b->value[i] ^= power;
            if ((twos_complement_of_b->value[i] & power) != 0)
            {
                goto break_both_loops;
            }
            power = power << 1;
        }
    }
    break_both_loops:
    add_b_value_to_a_in_place(a, twos_complement_of_b);
    integer_trim_leading_zeroes(a);
    stack->cursor = stack_savepoint;
}

struct Integer*multiply_integers(struct Integer*a, struct Integer*b, struct Stack*stack)
{
    struct Integer*out = allocate_integer(stack, a->value_count + b->value_count);
    out->value_count = 0;
    for (int i = 0; i < a->value_count; ++i)
    {
        for (int j = 0; j < b->value_count; ++j)
        {
            uint64_t product_component = (uint64_t)a->value[i] * b->value[j];
            size_t shift = i + j;
            struct Integer*integer_component = allocate_integer(stack, shift + 2);
            integer_component->value_count = shift;
            memset(&integer_component->value, 0, shift * sizeof(uint32_t));
            if (product_component > 0)
            {
                integer_component->value[shift] = (uint32_t)product_component;
                integer_component->value_count += 1;
                uint32_t high_bytes = (product_component & 0xffffffff00000000) >> 32;
                if (high_bytes > 0)
                {
                    integer_component->value[shift + 1] = high_bytes;
                    integer_component->value_count += 1;
                }
            }
            add_integer_to_a_in_place(out, integer_component);
        }
    }
    stack->cursor = out->value + out->value_count;
    return out;
}

size_t get_leading_digit_place(struct Integer*a)
{
    if (!a->value_count)
    {
        return 0;
    }
    uint32_t divisor_leading_digit = 0x80000000;
    uint32_t last_value = a->value[a->value_count - 1];
    size_t i = 31;
    while (true)
    {
        if ((last_value & divisor_leading_digit) != 0)
        {
            return i;
        }
        divisor_leading_digit = divisor_leading_digit >> 1;
        --i;
    }
}

void upshift_integer(struct Integer*a, uint8_t shift)
{
    for (size_t i = a->value_count; i-- > 1;)
    {
        a->value[i] = a->value[i] << shift | a->value[i - 1] >> (32 - shift);
    }
    a->value[0] = a->value[0] << shift;
}

struct Integer*double_integer(struct Integer*a, struct Stack*stack)
{
    struct Integer*out;
    if (!a->value_count)
    {
        out = allocate_integer(stack, 0);
        out->value_count = 0;
        return out;
    }
    if (a->value[a->value_count - 1] & 0x80000000)
    {
        out = allocate_integer(stack, a->value_count + 1);
        out->value_count = a->value_count + 1;
        out->value[a->value_count] = 0;
    }
    else
    {
        out = allocate_integer(stack, a->value_count);
        out->value_count = a->value_count;
    }
    memcpy(&out->value, &a->value, a->value_count * sizeof(uint32_t));
    upshift_integer(out, 1);
    return out;
}

void downshift_integer(struct Integer*a, uint8_t shift)
{
    for (size_t i = 0; i < a->value_count - 1; ++i)
    {
        a->value[i] = a->value[i] >> shift | a->value[i + 1] << (32 - shift);
    }
    a->value[a->value_count - 1] = a->value[a->value_count - 1] >> shift;
}

void halve_integer_in_place(struct Integer*a)
{
    downshift_integer(a, 1);
    integer_trim_leading_zeroes(a);
}

int8_t compare_integers(struct Integer*a, struct Integer*b)
{
    if (a->value_count > b->value_count)
    {
        return 1;
    }
    if (a->value_count < b->value_count)
    {
        return -1;
    }
    for (uint32_t i = a->value_count; i-- > 0;)
    {
        if (a->value[i] > b->value[i])
        {
            return 1;
        }
        if (a->value[i] < b->value[i])
        {
            return -1;
        }
    }
    return 0;
}

void divide_integers(struct Division*out, struct Integer*dividend, struct Integer*divisor,
    struct Stack*stack)
{
    size_t dividend_leading_digit_place = get_leading_digit_place(dividend);
    size_t divisor_leading_digit_place = get_leading_digit_place(divisor);
    if (dividend->value_count > divisor->value_count ||
        (dividend->value_count == divisor->value_count &&
            dividend_leading_digit_place >= divisor_leading_digit_place))
    {
        out->quotient = allocate_integer(stack, dividend->value_count);
        out->quotient->value_count = dividend->value_count;
        memset(&out->quotient->value, 0, out->quotient->value_count * sizeof(uint32_t));
        out->remainder = copy_integer_to_stack(dividend, stack);
        struct Integer*shifted_divisor = allocate_integer(stack, dividend->value_count);
        shifted_divisor->value_count = dividend->value_count;
        size_t quotient_value_index = dividend->value_count - divisor->value_count;
        memset(&shifted_divisor->value, 0, quotient_value_index * sizeof(uint32_t));
        memcpy(&shifted_divisor->value[quotient_value_index], &divisor->value,
            divisor->value_count * sizeof(uint32_t));
        int shift = dividend_leading_digit_place - divisor_leading_digit_place;
        uint32_t quotient_digit;
        if (shift > 0)
        {
            upshift_integer(shifted_divisor, shift);
            quotient_digit = 1 << shift;
        }
        else if (shift < 0)
        {
            shift *= -1;
            downshift_integer(shifted_divisor, shift);
            quotient_digit = 1 << (32 - shift);
            quotient_value_index -= 1;
        }
        else
        {
            quotient_digit = 1;
        }
        while (true)
        {
            for (int i = 32; i > 0; --i)
            {
                if (compare_integers(out->remainder, shifted_divisor) >= 0)
                {
                    out->quotient->value[quotient_value_index] |= quotient_digit;
                    subtract_integer_from_a_in_place(out->remainder, shifted_divisor, stack);
                }
                if (quotient_digit == 1)
                {
                    if (quotient_value_index == 0)
                    {
                        goto break_both_loops;
                    }
                    quotient_digit = 0x80000000;
                    quotient_value_index -= 1;
                }
                else
                {
                    quotient_digit = quotient_digit >> 1;
                }
                halve_integer_in_place(shifted_divisor);
            }
        }
    break_both_loops:
        integer_trim_leading_zeroes(out->quotient);
        stack->cursor = out->remainder->value + out->remainder->value_count;
    }
    else
    {
        out->quotient = initialize_stack_integer(stack, 0);
        out->remainder = copy_integer_to_stack(dividend, stack);
    }
}

struct Integer*get_integer_quotient(struct Integer*dividend, struct Integer*divisor,
    struct Stack*stack)
{
    struct Division division;
    divide_integers(&division, dividend, divisor, stack);
    return division.quotient;
}

uint8_t integer_to_wchar_string(wchar_t**out, uint16_t integer, wchar_t codepoint_of_zero,
    uint8_t buffer_length)
{
    *out += buffer_length;
    wchar_t*buffer_end = *out;
    while (integer)
    {
        --*out;
        **out = codepoint_of_zero + integer % 10;
        integer /= 10;
    }
    return (uint8_t)(buffer_end - *out);
}

void copy_rational_to_persistent_memory(struct Project*project, struct Rational*source,
    struct Rational*destination)
{
    destination->numerator = copy_integer_to_persistent_memory(source->numerator, project);
    destination->denominator = copy_integer_to_persistent_memory(source->denominator, project);
}

void free_rational_from_persistent_memory(struct Project*project, struct Rational*a)
{
    free_integer_from_persistent_memory(a->numerator, project);
    free_integer_from_persistent_memory(a->denominator, project);
}

void subtract_rationals(struct Rational*out, struct Rational*minuend, struct Rational*subtrahend,
    struct Stack*stack)
{
    struct Integer*unreduced_numerator =
        multiply_integers(minuend->numerator, subtrahend->denominator, stack);
    subtract_integer_from_a_in_place(unreduced_numerator,
        multiply_integers(subtrahend->numerator, minuend->denominator, stack), stack);
    struct Integer*unreduced_denominator =
        multiply_integers(minuend->denominator, subtrahend->denominator, stack);
    struct Integer*gcd = unreduced_numerator;
    struct Integer*a = unreduced_denominator;
    while (a->value_count)
    {
        struct Integer*b = a;
        struct Division division;
        divide_integers(&division, gcd, a, stack);
        a = division.remainder;
        gcd = b;
    }
    out->numerator = get_integer_quotient(unreduced_numerator, gcd, stack);
    out->denominator = get_integer_quotient(unreduced_denominator, gcd, stack);
}

int8_t compare_rationals(struct Rational*a, struct Rational*b, struct Stack*stack)
{
    void*stack_savepoint = stack->cursor;
    int8_t out = compare_integers(multiply_integers(a->numerator, b->denominator, stack),
        multiply_integers(b->numerator, a->denominator, stack));
    stack->cursor = stack_savepoint;
    return out;
}