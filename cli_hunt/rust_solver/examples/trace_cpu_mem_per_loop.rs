// Trace CPU memory accesses per loop

use ashmaize::{Rom, RomGenerationType, b2::VM};

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * 1_024 * 1_024;

fn main() {
    let rom = Rom::new(
        b"0",
        RomGenerationType::TwoStep {
            pre_size: 16 * MB,
            mixing_numbers: 4,
        },
        1 * GB,
    );

    let nonce = 0x12345u64;
    let suffix = "test100000001001";
    let preimage = format!("{:016x}{}", nonce, suffix);
    let salt = preimage.as_bytes();

    let mut vm = VM::new(&rom.digest, 256, salt);
    vm.program.shuffle(&vm.prog_seed);

    println!("Tracing CPU memory accesses per loop:\n");
    
    let mut loop_mem_counts = vec![0; 8];
    
    for loop_num in 0..8 {
        if loop_num > 0 {
            vm.program.shuffle(&vm.prog_seed);
        }
        
        let mem_before = vm.memory_counter;
        
        for _ in 0..256 {
            vm.step(&rom);
        }
        
        vm.post_instructions();
        
        let mem_after = vm.memory_counter;
        loop_mem_counts[loop_num] = mem_after - mem_before;
        
        println!("Loop {}: {} memory accesses (total: {})", 
            loop_num, loop_mem_counts[loop_num], mem_after);
    }
    
    println!("\n=== SUMMARY ===");
    println!("Per-loop memory access counts:");
    for (i, count) in loop_mem_counts.iter().enumerate() {
        println!("  Loop {}: {} accesses", i, count);
    }
    
    let total: u32 = loop_mem_counts.iter().sum();
    let avg = total as f32 / 8.0;
    println!("\nTotal: {} accesses", total);
    println!("Average per loop: {:.2} accesses", avg);
    
    println!("\nFinal memory_counter: {}", vm.memory_counter);
    println!("Expected (CPU): 850");
    println!("GPU shows: 913");
    println!("Difference: {} extra accesses on GPU", 913 - 850);
}

