// Trace program generation to compare with GPU

use ashmaize::{Rom, RomGenerationType, b2::VM};

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * 1_024 * 1_024;

fn main() {
    println!("=== Program Generation Trace ===\n");

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
    
    let mut vm = VM::new(&rom.digest, 256, salt);
    
    // Program is initially all zeros
    println!("Program before shuffle (first 40 bytes):");
    println!("{}", hex::encode(&vm.program.get_instructions()[..40]));
    
    // Now manually call shuffle (which is what execute() does)
    vm.program.shuffle(&vm.prog_seed);
    
    println!("\nProgram after shuffle (first 40 bytes):");
    println!("{}", hex::encode(&vm.program.get_instructions()[..40]));
    
    println!("\nprog_seed used for shuffle:");
    println!("{}", hex::encode(&vm.prog_seed));
}

