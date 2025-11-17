// Test finalization logic - compare inputs to final Blake2b

use ashmaize::{Rom, RomGenerationType, b2::VM};

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * 1_024 * 1_024;

fn main() {
    println!("=== Finalization Test - CPU ===\n");

    // Use the SAME ROM parameters as main.rs init_rom()
    let rom = Rom::new(
        b"0",
        RomGenerationType::TwoStep {
            pre_size: 16 * MB,
            mixing_numbers: 4,
        },
        1 * GB,
    );

    // Use the actual preimage from test-hash command
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
    let nb_loops = 8u32;
    let nb_instrs = 256u32;

    println!("Salt: {}", String::from_utf8_lossy(salt));
    println!("nb_loops: {}, nb_instrs: {}\n", nb_loops, nb_instrs);

    // Execute the full hash computation
    let mut vm = VM::new(&rom.digest, nb_instrs, salt);
    
    for loop_idx in 0..nb_loops {
        // Print program buffer for first loop before execution
        if loop_idx == 0 {
            let prog_bytes = vm.program.get_instructions();
            println!("CPU Program buffer first 40 bytes:");
            println!("{}", hex::encode(&prog_bytes[..40]));
        }
        
        vm.execute(&rom, nb_instrs);
        println!("After loop {}: loop_counter={}, memory_counter={}", 
            loop_idx, loop_idx + 1, vm.memory_counter);
    }

    println!("\nBefore finalize:");
    println!("  memory_counter: {}", vm.memory_counter);
    println!("  loop_counter: {}", nb_loops);
    println!("  regs[0]: 0x{:016x}", vm.regs[0]);
    println!("  regs[1]: 0x{:016x}", vm.regs[1]);
    println!("  regs[31]: 0x{:016x}", vm.regs[31]);

    let result = vm.finalize();
    println!("\nFinal hash: {}", hex::encode(&result[..16]));
    println!("Full hash: {}", hex::encode(&result));
}

