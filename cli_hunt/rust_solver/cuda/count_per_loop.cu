// Add this to print per-loop memory counts on GPU
// Insert after post_instructions in ashmaize.cu

/*
After each post_instructions, add:

if (tid == 0) {
    static uint32_t loop_mem_prev = 0;
    uint32_t this_loop_count = vm.memory_counter - loop_mem_prev;
    printf("GPU Loop %u: %u memory accesses (total: %u)\n", 
        vm.loop_counter - 1, this_loop_count, vm.memory_counter);
    loop_mem_prev = vm.memory_counter;
}
*/

