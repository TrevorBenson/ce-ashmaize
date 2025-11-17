// Check CPU's final memory_counter

use ashmaize::{Rom, RomGenerationType, b2::hash};

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

    // Unfortunately we can't easily get the VM's final state without modifying the library
    // But we know from previous traces that the CPU had memory_counter = 850 after full execution
    println!("Checking CPU hash...");
    let result = hash(salt, &rom, 8, 256);
    println!("CPU hash: {}", hex::encode(&result[..8]));
    println!("\nNote: CPU memory_counter was confirmed to be 850 in previous traces");
    println!("GPU memory_counter is now 913 after Bug #11 fix");
    println!("This suggests 63 extra memory accesses on GPU!");
}

