// Check CPU VM's final state including memory_counter

use ashmaize::{Rom, RomGenerationType};
use ashmaize::b2::{VM, INSTR_SIZE};

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

    // Create VM and run full execution
    let mut vm = VM::new(&rom.digest, 256, salt);
    vm.program.shuffle(&vm.prog_seed);

    println!("Running full VM execution (8 loops, 256 instructions each)...\n");

    // Run the full execution like the hash function does
    for loop_num in 0..8 {
        vm.program.shuffle(&vm.prog_seed);
        
        for _ in 0..256 {
            vm.step(&rom);
        }
        
        vm.post_instructions();
    }

    println!("Final VM state:");
    println!("  loop_counter: {}", vm.loop_counter);
    println!("  memory_counter: {}", vm.memory_counter);
    println!("  ip: {}", vm.ip);
    println!("  regs[0]: 0x{:016x}", vm.regs[0]);
    println!("  regs[1]: 0x{:016x}", vm.regs[1]);
    println!("  regs[31]: 0x{:016x}", vm.regs[31]);

    println!("\n=== COMPARISON ===");
    println!("CPU memory_counter: {}", vm.memory_counter);
    println!("GPU memory_counter: 913 (from test)");
    
    let cpu_mem_ctr = vm.memory_counter;
    
    // Finalize to get hash
    let hash = vm.finalize();
    println!("\nFinal hash: {}", hex::encode(&hash[..8]));
    println!("Expected:   0ec51a39c5ba9d33");
    
    if cpu_mem_ctr == 913 {
        println!("\n✓ MATCH - Both have same number of memory accesses!");
    } else {
        println!("\n✗ MISMATCH - Different number of memory accesses!");
        println!("Difference: {} accesses", (cpu_mem_ctr as i32 - 913).abs());
    }
}

