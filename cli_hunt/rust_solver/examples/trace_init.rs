// Trace VM initialization to compare with GPU

use ashmaize::{Rom, RomGenerationType, b2::VM};

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * 1_024 * 1_024;

fn main() {
    println!("=== VM Initialization Trace ===\n");

    // Use the SAME ROM parameters as main.rs init_rom()
    let rom = Rom::new(
        b"0",
        RomGenerationType::TwoStep {
            pre_size: 16 * MB,
            mixing_numbers: 4,
        },
        1 * GB,
    );

    let address = "test";
    let challenge_id = "1";
    let difficulty = "00000001";
    let no_pre_mine = "0";
    let latest_submission = "0";
    let no_pre_mine_hour = "1";  // Match GPU test
    let nonce = 0x12345u64;
    
    let suffix = format!(
        "{}{}{}{}{}{}",
        address, challenge_id, difficulty, no_pre_mine, latest_submission, no_pre_mine_hour
    );
    let preimage = format!("{:016x}{}", nonce, suffix);
    let salt = preimage.as_bytes();

    println!("Salt: {}", String::from_utf8_lossy(salt));
    println!("Salt length: {} bytes\n", salt.len());
    
    // Manually trace VM initialization
    println!("ROM digest: {}", hex::encode(rom.digest.as_bytes()));
    println!("ROM digest length: {} bytes\n", rom.digest.as_bytes().len());
    
    // Construct init_buffer_input
    let mut init_buffer_input = rom.digest.as_bytes().to_vec();
    init_buffer_input.extend_from_slice(salt);
    
    println!("init_buffer_input ({} bytes):", init_buffer_input.len());
    println!("{}\n", hex::encode(&init_buffer_input));
    
    // Create a VM to get the prog_seed via VM::new (which calls hprime internally)
    let vm = VM::new(&rom.digest, 256, salt);
    println!("VM prog_seed from VM::new():");
    println!("{}", hex::encode(&vm.prog_seed));
}

