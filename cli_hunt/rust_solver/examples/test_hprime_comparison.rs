// Test to verify prog_seed matches after loop 0 between CPU and GPU

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
    
    println!("Initial prog_seed:");
    println!("  {}", hex::encode(&vm.prog_seed));
    
    // First shuffle (like GPU does at start of loop 0)
    vm.program.shuffle(&vm.prog_seed);
    
    println!("\nAfter first shuffle:");
    println!("  prog_seed unchanged: {}", hex::encode(&vm.prog_seed));
    
    // Execute loop 0
    for _ in 0..256 {
        vm.step(&rom);
    }
    
    println!("\nAfter executing loop 0 (before post_instructions):");
    println!("  prog_seed still: {}", hex::encode(&vm.prog_seed));
    println!("  memory_counter: {}", vm.memory_counter);
    
    // Call post_instructions
    vm.post_instructions();
    
    println!("\nAfter post_instructions (end of loop 0):");
    println!("  prog_seed NOW: {}", hex::encode(&vm.prog_seed));
    println!("  loop_counter: {}", vm.loop_counter);
    println!("  memory_counter: {}", vm.memory_counter);
    
    println!("\n=== This prog_seed will be used to shuffle for loop 1 ===");
    println!("GPU should print the same prog_seed after loop 0!");
}

