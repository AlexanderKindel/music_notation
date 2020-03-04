size_t PAGE_SIZE;

void*round_down_to_alignment(size_t alignment, void*value);
void*round_up_to_alignment(size_t alignment, void*value);
void initialize_stack(struct Stack*out, size_t size);
void*start_array(struct Stack*stack, size_t alignment);
void*extend_array(struct Stack*stack, size_t element_size);
void*allocate_stack_slot(struct Stack*stack, size_t slot_size, size_t alignment);
void initialize_pool(struct Pool*out, size_t size, uint32_t element_size);
void*allocate_pool_slot(struct Pool*pool);
void free_pool_slot(struct Pool*pool, void*slot);
void*resolve_pool_index(struct Pool*pool, uint32_t index);
void*resolve_address(struct Project*project, uint32_t address);
uint32_t get_element_index_in_pool(struct Pool*pool, void*element);
struct Page*initialize_page_list(struct Pool*page_pool, size_t element_size);
void*resolve_page_index(struct Page*page, uint32_t element_index, uint32_t element_size);
void initialize_page_element_iter(struct BaseIter*out, void*element, uint32_t element_size);
void increment_page_element_iter(struct BaseIter*iter, struct Pool*page_pool,
    uint32_t element_size);
void decrement_page_element_iter(struct BaseIter*iter, struct Pool*page_pool,
    uint32_t element_size);
void move_page_element(struct Project*project, void*new_location, void*old_location,
    uint32_t element_size);
struct Page*insert_new_page_after(struct Page*page, struct Pool*page_pool);
void insert_page_element_before_iter(struct BaseIter*iter, struct Project*project,
    uint32_t element_size);
void remove_page_element_at_iter(struct BaseIter*iter, struct Project*project,
    uint32_t element_size);