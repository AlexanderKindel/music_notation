#include "declarations.h"

void*round_down_to_alignment(size_t alignment, void*value)
{
    return (void*)(((uintptr_t)value / alignment) * alignment);
}

void*round_up_to_alignment(size_t alignment, void*value)
{
    return round_down_to_alignment(alignment, (uint8_t*)value + alignment - 1);
}

void initialize_stack(struct Stack*out, size_t size)
{
    out->start = VirtualAlloc(0, size, MEM_RESERVE, PAGE_READWRITE);
    out->end = round_down_to_alignment(PAGE_SIZE, (uint8_t*)out->start + size);
    out->cursor = out->start;
    out->cursor_max = out->start;
}

void*start_array(struct Stack*stack, size_t alignment)
{
    stack->cursor = round_up_to_alignment(alignment, stack->cursor);
    return stack->cursor;
}

void*extend_array(struct Stack*stack, size_t element_size)
{
    void*out = stack->cursor;
    stack->cursor = (uint8_t*)stack->cursor + element_size;
    if (stack->cursor > stack->end)
    {
        crash("Ran out of stack memory.");
    }
    while (stack->cursor > stack->cursor_max)
    {
        VirtualAlloc(stack->cursor_max, PAGE_SIZE, MEM_COMMIT, PAGE_READWRITE);
        stack->cursor_max = (uint8_t*)stack->cursor_max + PAGE_SIZE;
    }
    return out;
}

void*allocate_stack_slot(struct Stack*stack, size_t slot_size, size_t alignment)
{
    void*slot = start_array(stack, alignment);
    extend_array(stack, slot_size);
    return slot;
}

void initialize_pool(struct Pool*out, size_t size, uint32_t element_size)
{
    out->start = VirtualAlloc(0, size, MEM_RESERVE, PAGE_READWRITE);
    out->end = round_down_to_alignment(PAGE_SIZE, (uint8_t*)out->start + size);
    out->cursor = (uint8_t*)out->start + element_size;
    out->element_size = element_size;
    out->index_of_first_free_element = 0;
}

void*allocate_pool_slot(struct Pool*pool)
{
    void*out;
    if (pool->index_of_first_free_element)
    {
        out = resolve_pool_index(pool, pool->index_of_first_free_element);
        pool->index_of_first_free_element = *(uint32_t*)out;
        return out;
    }
    out = pool->cursor;
    pool->cursor = (uint8_t*)pool->cursor + pool->element_size;
    if (pool->cursor >= pool->end)
    {
        crash("Ran out of pool memory.");
    }
    VirtualAlloc(out, PAGE_SIZE, MEM_COMMIT, PAGE_READWRITE);
    return out;
}

void free_pool_slot(struct Pool*pool, void*slot)
{
    *(uint32_t*)slot = pool->index_of_first_free_element;
    pool->index_of_first_free_element = get_element_index_in_pool(pool, slot);
}

void*resolve_pool_index(struct Pool*pool, uint32_t index)
{
    ASSERT(index, "Attempted to resolve a pool index of 0.");
    return (uint8_t*)pool->start + pool->element_size * index;
}

void*resolve_address(struct Project*project, uint32_t address)
{
    return *(void**)resolve_pool_index(POINTER_POOL(project), address);
}

uint32_t get_element_index_in_pool(struct Pool*pool, void*element)
{
    return ((uintptr_t)element - (uintptr_t)pool->start) / pool->element_size;
}

struct Page*initialize_page_list(struct Pool*page_pool, size_t element_size)
{
    struct Page*out = allocate_pool_slot(page_pool);
    out->previous_page_index = 0;
    out->next_page_index = 0;
    out->capacity = (PAGE_SIZE - sizeof(struct Page)) / element_size;
    out->occupied_slot_count = 0;
    return out;
}

void*resolve_page_index(struct Page*page, uint32_t element_index, uint32_t element_size)
{
    return page->bytes + element_size * element_index;
}

void initialize_page_element_iter(struct BaseIter*out, void*element, uint32_t element_size)
{
    ((struct PageElementIter*)out)->element = element;
    out->page = (struct Page*)round_down_to_alignment(PAGE_SIZE, element);
    out->element_index_on_page = ((uintptr_t)element - (uintptr_t)out->page->bytes) / element_size;
}

void increment_page_element_iter(struct BaseIter*iter, struct Pool*page_pool,
    uint32_t element_size)
{
    ++iter->element_index_on_page;
    if (iter->element_index_on_page == iter->page->occupied_slot_count)
    {
        if (iter->page->next_page_index)
        {
            iter->page = resolve_pool_index(page_pool, iter->page->next_page_index);
            iter->element_index_on_page = 0;
            ((struct PageElementIter*)iter)->element = iter->page->bytes;
        }
        else
        {
            ((struct PageElementIter*)iter)->element = 0;
        }
    }
    else
    {
        ((struct PageElementIter*)iter)->element =
            (uint8_t*)((struct PageElementIter*)iter)->element + element_size;
    }
}

void decrement_page_element_iter(struct BaseIter*iter, struct Pool*page_pool, uint32_t element_size)
{
    if (iter->element_index_on_page == 0)
    {
        if (iter->page->previous_page_index)
        {
            iter->page = resolve_pool_index(page_pool, iter->page->previous_page_index);
            iter->element_index_on_page = iter->page->occupied_slot_count - 1;
            ((struct PageElementIter*)iter)->element =
                resolve_page_index(iter->page, iter->element_index_on_page, element_size);
        }
        else
        {
            ((struct PageElementIter*)iter)->element = 0;
        }
    }
    else
    {
        --iter->element_index_on_page;
        ((struct PageElementIter*)iter)->element =
            (uint8_t*)((struct PageElementIter*)iter)->element - element_size;
    }
}

void move_page_element(struct Project*project, void*new_location, void*old_location,
    uint32_t element_size)
{
    *(void**)resolve_pool_index(POINTER_POOL(project), *(uint32_t*)old_location) = new_location;
    memcpy(new_location, old_location, element_size);
}

void shift_page_elements_up_an_index(struct Page*page, struct Project*project,
    void*lowest_index_element, uint32_t element_size)
{
    void*element = resolve_page_index(page, page->occupied_slot_count, element_size);
    while (element > lowest_index_element)
    {
        void*next_highest_index_element = (uint8_t*)element - element_size;
        move_page_element(project, element, next_highest_index_element, element_size);
        element = next_highest_index_element;
    }
    ++page->occupied_slot_count;
}

struct Page*insert_new_page_after(struct Page*page, struct Pool*page_pool)
{
    struct Page*new_page = allocate_pool_slot(page_pool);
    new_page->next_page_index = page->next_page_index;
    page->next_page_index = get_element_index_in_pool(page_pool, new_page);
    if (new_page->next_page_index)
    {
        ((struct Page*)resolve_pool_index(page_pool, new_page->next_page_index))->
            previous_page_index = page->next_page_index;
    }
    new_page->previous_page_index = get_element_index_in_pool(page_pool, page);
    new_page->capacity = page->capacity;
    new_page->occupied_slot_count = 0;
    return new_page;
}

void insert_unfilled_page_element_before_iter(struct PageElementIter*iter, struct Project*project,
    uint32_t element_size)
{
    shift_page_elements_up_an_index(iter->page, project, iter->element, element_size);
    void**pointer_to_slot = allocate_pool_slot(POINTER_POOL(project));
    *pointer_to_slot = iter->element;
    *(uint32_t*)iter->element = get_element_index_in_pool(POINTER_POOL(project), pointer_to_slot);
}

void insert_page_element_before_iter(struct BaseIter*iter, struct Project*project,
    uint32_t element_size)
{
    if (iter->page->occupied_slot_count < iter->page->capacity)
    {
        if (!((struct PageElementIter*)iter)->element)
        {
            ((struct PageElementIter*)iter)->element =
                resolve_page_index(iter->page, iter->element_index_on_page, element_size);
        }
        insert_unfilled_page_element_before_iter((struct PageElementIter*)iter, project,
            element_size);
        return;
    }
    struct Page*next_page;
    if (iter->page->next_page_index)
    {
        next_page =
            resolve_pool_index(&project->page_pool, iter->page->next_page_index);
        if (next_page->occupied_slot_count == next_page->capacity)
        {
            next_page = insert_new_page_after(iter->page, &project->page_pool);
        }
        else
        {
            shift_page_elements_up_an_index(next_page, project, next_page->bytes + element_size,
                element_size);
        }
    }
    else
    {
        next_page = insert_new_page_after(iter->page, &project->page_pool);
    }
    ++next_page->occupied_slot_count;
    if (((struct PageElementIter*)iter)->element)
    {
        move_page_element(project, next_page->bytes + element_size,
            resolve_page_index(iter->page, iter->page->occupied_slot_count - 1, element_size),
            element_size);
        --iter->page->occupied_slot_count;
        insert_unfilled_page_element_before_iter((struct PageElementIter*)iter, project,
            element_size);
    }
    else
    {
        iter->element_index_on_page = 0;
        iter->page = next_page;
        ((struct PageElementIter*)iter)->element = next_page->bytes;
    }
}

void remove_page_element_at_iter(struct BaseIter*iter, struct Project*project,
    uint32_t element_size)
{
    free_pool_slot(POINTER_POOL(project), resolve_pool_index(POINTER_POOL(project),
        *(uint32_t*)((struct PageElementIter*)iter)->element));
    struct Page*page =
        (struct Page*)round_down_to_alignment(PAGE_SIZE, ((struct PageElementIter*)iter)->element);
    if (page->occupied_slot_count == 1)
    {
        if (page->previous_page_index)
        {
            struct Page*previous_page =
                resolve_pool_index(&project->page_pool, page->previous_page_index);
            previous_page->next_page_index = page->next_page_index;
            iter->page = previous_page;
            iter->element_index_on_page = previous_page->occupied_slot_count;
            ((struct PageElementIter*)iter)->element = 0;
        }
        if (page->next_page_index)
        {
            struct Page*next_page = resolve_pool_index(&project->page_pool, page->next_page_index);
            next_page->previous_page_index = page->previous_page_index;
            iter->page = next_page;
            iter->element_index_on_page = 0;
            ((struct PageElementIter*)iter)->element = next_page->bytes;
        }
        free_pool_slot(&project->page_pool, page);
    }
    else
    {
        if (iter->element_index_on_page + 1 == page->occupied_slot_count)
        {
            if (!page->next_page_index)
            {
                ((struct PageElementIter*)iter)->element = 0;
            }
        }
        else
        {
            void*highest_index_element =
                resolve_page_index(page, page->occupied_slot_count, element_size);
            void*element = ((struct PageElementIter*)iter)->element;
            while (true)
            {
                void*next_lowest_index_element = (uint8_t*)element + element_size;
                if (next_lowest_index_element == highest_index_element)
                {
                    break;
                }
                move_page_element(project, element, next_lowest_index_element, element_size);
                element = next_lowest_index_element;
            }
        }
        --page->occupied_slot_count;
    }
}