// Trace CPU's post_instructions intermediate values

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
    
    // Execute loop 0
    for _ in 0..256 {
        vm.step(&rom);
    }
    
    println!("Before post_instructions (end of loop 0):");
    let sum_regs = vm.sum_regs();
    println!("  sum_regs: 0x{:016x}", sum_regs);
    println!("  loop_counter: {}", vm.loop_counter);
    println!("  memory_counter: {}", vm.memory_counter);
    
    // Call post_instructions
    vm.post_instructions();
    
    println!("\nAfter post_instructions:");
    println!("  prog_seed: {}", hex::encode(&vm.prog_seed));
    println!("  loop_counter: {}", vm.loop_counter);
    
    println!("\n=== Expected GPU to match ===");
}

