// Trace CPU's Blake2b input for instruction 0

use ashmaize::{Rom, RomGenerationType, b2::VM};

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * 1_024 * 1_024;

fn main() {
    println!("=== CPU Blake2b Input Trace ===\n");

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

    let mut vm = VM::new(&rom.digest, 256, salt);
    vm.program.shuffle(&vm.prog_seed);
    
    // For instruction 0, we know:
    // - opcode = 250 (Blake2b Hash, v=2)
    // - op1 = 11 (Literal)
    // - op2 = 6 (Memory)
    // - lit1 = 0x57aaaa9e896b78ed
    // - lit2 = 0x9c6e1a4905d7e8b0
    
    // src1 = lit1 (Literal)
    let src1: u64 = 0x57aaaa9e896b78ed;
    
    // src2 = mem_access64(lit2) - we already traced this = 0xe8bcb6feb065ea96
    let src2: u64 = 0xe8bcb6feb065ea96;
    
    println!("Instruction 0 operands:");
    println!("  src1 (literal): 0x{:016x}", src1);
    println!("  src2 (memory):  0x{:016x}", src2);
    
    // Create Blake2b input (16 bytes) - little-endian
    let mut input = [0u8; 16];
    input[0..8].copy_from_slice(&src1.to_le_bytes());
    input[8..16].copy_from_slice(&src2.to_le_bytes());
    
    print!("\nBlake2b input (16 bytes): ");
    for b in &input {
        print!("{:02x}", b);
    }
    println!();
    
    println!("\n=== COMPARISON ===");
    println!("GPU input: ed786b899eaaaa5796ea65b0feb6bce8");
    print!("CPU input: ");
    for b in &input {
        print!("{:02x}", b);
    }
    println!();
    
    let gpu_input = "ed786b899eaaaa5796ea65b0feb6bce8";
    let cpu_input_hex = input.iter().map(|b| format!("{:02x}", b)).collect::<String>();
    
    if gpu_input == cpu_input_hex {
        println!("✓ MATCH - Inputs are identical!");
    } else {
        println!("✗ MISMATCH - This is the bug!");
        println!("\nDifference analysis:");
        for (i, (g, c)) in gpu_input.as_bytes().chunks(2)
            .zip(cpu_input_hex.as_bytes().chunks(2)).enumerate() {
            let gb = format!("{}{}", g[0] as char, g[1] as char);
            let cb = format!("{}{}", c[0] as char, c[1] as char);
            if gb != cb {
                println!("  Byte {}: GPU={} CPU={}", i, gb, cb);
            }
        }
    }
}

