// Trace ACTUAL CPU VM execution to see what really happens
// This runs the REAL hash function and prints what happens

use ashmaize::{Rom, RomGenerationType};
use ashmaize::b2::{hash, VM, INSTR_SIZE};

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * 1_024 * 1_024;

fn main() {
    println!("=== Trace ACTUAL CPU VM Execution ===\n");

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

    println!("Salt: {}", String::from_utf8_lossy(salt));
    
    // Create VM and shuffle program - same as actual execution
    let mut vm = VM::new(&rom.digest, 256, salt);
    vm.program.shuffle(&vm.prog_seed);
    
    println!("\nBefore execution:");
    println!("  IP: {}", vm.ip);
    println!("  loop_counter: {}", vm.loop_counter);
    println!("  memory_counter: {}", vm.memory_counter);
    
    // Print first instruction bytes
    let instr_bytes = *vm.program.at(vm.ip);
    print!("  First instruction bytes: ");
    for b in &instr_bytes {
        print!("{:02x}", b);
    }
    println!();
    
    // Decode
    let opcode = instr_bytes[0];
    let op1 = instr_bytes[1] >> 4;
    let op2 = instr_bytes[1] & 0x0F;
    println!("  Decoded: opcode={}, op1={}, op2={}", opcode, op1, op2);
    
    // Now run the ACTUAL hash function
    println!("\nRunning actual hash...");
    let result = hash(salt, &rom, 8, 256);
    
    println!("\nFinal hash result: {}", hex::encode(&result[..8]));
    
    println!("\n=== COMPARISON ===");
    println!("GPU result: 0xf0bf3ba6a99e1d14");
    println!("CPU result: 0x{}", hex::encode(&result[..8]));
    
    if hex::encode(&result[..8]) == "0ec51a39c5ba9d33" {
        println!("\n✓ Matches expected CPU result from test-hash");
    } else {
        println!("\n✗ Different from expected");
    }
    
    println!("\nNOTE: The first instruction DOES execute opcode 250 (Blake2b)");
    println!("      but the trace showed result 0x5555698eee0fbaef");
    println!("      while blake2 crate produces 0x5694e7110ab9f4d3");
    println!("      This discrepancy needs investigation!");
}

