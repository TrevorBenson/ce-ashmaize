// CPU vs GPU Intermediate Values Debug
// Prints initial VM state to compare

use ashmaize::{Rom, RomGenerationType, b2::VM};

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * MB;

fn main() {
    println!("=== CPU Initial State Debug ===\n");

    // Simple ROM
    let rom = Rom::new(
        b"0",
        RomGenerationType::TwoStep {
            pre_size: 16 * MB,
            mixing_numbers: 4,
        },
        1 * GB,
    );

    // Test salt
    let salt = b"test";
    let nb_instrs = 256;

    println!("ROM Digest: {}", hex::encode(&rom.digest.as_bytes()));
    println!("Salt: {}", String::from_utf8_lossy(salt));
    println!("nb_instrs: {}\n", nb_instrs);

    // Create VM
    let vm = VM::new(&rom.digest, nb_instrs, salt);

    // Use the CPU hash function to check we're running correctly
    let result = ashmaize::b2::hash(salt, &rom, 8, 256);
    println!("CPU Hash Result: {}\n", hex::encode(&result[..16]));
    
    // Check that sum_regs gives us something non-zero
    println!("Initial sum_regs: {}", vm.sum_regs());
}

