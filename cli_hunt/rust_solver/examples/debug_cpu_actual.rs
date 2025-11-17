// Debug ACTUAL CPU execution by running the real hash function
// and instrumenting to see what's happening

use ashmaize::{Rom, RomGenerationType};

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * 1_024 * 1_024;

fn main() {
    println!("=== Debug ACTUAL CPU Execution ===\n");

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
    let no_pre_mine_hour = "1";
    let nonce = 0x12345u64;
    
    let suffix = format!(
        "{}{}{}{}{}{}",
        address, challenge_id, difficulty, no_pre_mine, latest_submission, no_pre_mine_hour
    );
    let preimage = format!("{:016x}{}", nonce, suffix);
    let salt = preimage.as_bytes();

    println!("Salt: {}", String::from_utf8_lossy(salt));
    println!("Parameters: nb_loops=8, nb_instrs=256");
    
    // Run the ACTUAL CPU hash
    let result = ashmaize::b2::hash(salt, &rom, 8, 256);
    
    println!("\nCPU Result: {}", hex::encode(&result[..8]));
    println!("Full Result: {}", hex::encode(&result));
    
    println!("\n=== COMPARISON ===");
    println!("Expected (from test-hash): 0ec51a39c5ba9d33");
    println!("Our result:                {}", hex::encode(&result[..8]));
    
    if hex::encode(&result[..8]) == "0ec51a39c5ba9d33" {
        println!("✓ MATCH - We're running the same test!");
    } else {
        println!("✗ MISMATCH - Parameters are different!");
    }
}

