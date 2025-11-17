// Detailed CPU src2 tracing for instruction 0

use ashmaize::{Rom, RomGenerationType, b2::VM};

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * 1_024 * 1_024;

fn main() {
    println!("=== CPU src2 Detailed Trace ===\n");

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
    
    // Get instruction 0
    let instr_bytes = *vm.program.at(0);
    let opcode = instr_bytes[0];
    let op1 = instr_bytes[1] >> 4;
    let op2 = instr_bytes[1] & 0x0F;
    let lit2 = u64::from_le_bytes(*<&[u8; 8]>::try_from(&instr_bytes[12..20]).unwrap());
    
    println!("Instruction 0: opcode={}, op1={}, op2={}", opcode, op1, op2);
    println!("lit2=0x{:016x}", lit2);
    println!("op2={} → Memory operand", op2);
    
    // Calculate chunk access
    let num_chunks = rom.data.len() / 64;
    let chunk_index = (lit2 as usize) % num_chunks;
    let chunk_start = chunk_index * 64;
    let chunk = &rom.data[chunk_start..chunk_start + 64];
    
    println!("\nMemory access calculation:");
    println!("  addr (lit2) = 0x{:016x}", lit2);
    println!("  num_chunks = {}", num_chunks);
    println!("  chunk_index = {} (matches GPU ✓)", chunk_index);
    
    print!("  Chunk first 16 bytes: ");
    for i in 0..16 {
        print!("{:02x}", chunk[i]);
    }
    println!();
    
    // Simulate counter increment
    let memory_counter_before = vm.memory_counter;
    let memory_counter_after = memory_counter_before + 1;
    let idx = ((memory_counter_after % 8) as usize) * 8;
    
    println!("\nValue extraction:");
    println!("  memory_counter_before = {}", memory_counter_before);
    println!("  memory_counter_after = {}", memory_counter_after);
    println!("  idx_in_chunk = (memory_counter_after % 8) * 8 = {}", idx);
    
    print!("  Extracted bytes [{}..{}]: ", idx, idx + 8);
    for i in idx..idx + 8 {
        print!("{:02x}", chunk[i]);
    }
    println!();
    
    let src2 = u64::from_le_bytes(*<&[u8; 8]>::try_from(&chunk[idx..idx + 8]).unwrap());
    println!("  src2 (little-endian u64) = 0x{:016x}", src2);
    
    println!("\n=== COMPARISON ===");
    println!("CPU src2: 0x{:016x}", src2);
    println!("GPU src2: 0xe8bcb6feb065ea96");
    if src2 == 0xe8bcb6feb065ea96 {
        println!("✓ MATCH!");
    } else {
        println!("✗ MISMATCH - This is the bug!");
    }
}
