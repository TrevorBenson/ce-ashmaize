// Trace memory access to compare with GPU

use ashmaize::{Rom, RomGenerationType, b2::VM};
use blake2::Digest;

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * 1_024 * 1_024;

fn main() {
    println!("=== Memory Access Trace - CPU ===\n");

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
    let no_pre_mine_hour = "1";
    let nonce = 0x12345u64;
    
    let suffix = format!(
        "{}{}{}{}{}{}",
        address, challenge_id, difficulty, no_pre_mine, latest_submission, no_pre_mine_hour
    );
    let preimage = format!("{:016x}{}", nonce, suffix);
    let salt = preimage.as_bytes();

    println!("Salt: {}", String::from_utf8_lossy(salt));
    
    let mut vm = VM::new(&rom.digest, 256, salt);
    
    // Shuffle program for first loop
    vm.program.shuffle(&vm.prog_seed);
    
    // Get instruction 0
    let instr_bytes = *vm.program.at(0);
    let opcode = instr_bytes[0];
    let op1 = instr_bytes[1] >> 4;
    let op2 = instr_bytes[1] & 0x0F;
    
    // Extract lit1 and lit2
    let lit1 = u64::from_le_bytes(*<&[u8; 8]>::try_from(&instr_bytes[4..12]).unwrap());
    let lit2 = u64::from_le_bytes(*<&[u8; 8]>::try_from(&instr_bytes[12..20]).unwrap());
    
    println!("\n[CPU] Instr 0: opcode={}, op1={}, op2={}", opcode, op1, op2);
    println!("  lit1=0x{:016x}, lit2=0x{:016x}", lit1, lit2);
    
    // Manually simulate memory accesses by calculating ROM access directly
    if op1 >= 5 && op1 < 9 {
        println!("\n[CPU] src1 memory access:");
        println!("  addr={}, mem_ctr_before={}", lit1, vm.memory_counter);
        
        // Calculate chunk index and read from rom.data
        let num_chunks = rom.data.len() / 64;
        let chunk_index = (lit1 as usize) % num_chunks;
        let chunk_start = chunk_index * 64;
        let chunk = &rom.data[chunk_start..chunk_start + 64];
        
        print!("  Chunk first 16 bytes: ");
        for i in 0..16 {
            print!("{:02x}", chunk[i]);
        }
        println!();
        
        // Simulate digest update and counter increment
        vm.mem_digest.update(chunk);
        vm.memory_counter += 1;
        let idx = ((vm.memory_counter % 8) as usize) * 8;
        let result = u64::from_le_bytes(*<&[u8; 8]>::try_from(&chunk[idx..idx + 8]).unwrap());
        println!("  mem_ctr_after={}, idx_in_chunk={}, result=0x{:016x}", 
            vm.memory_counter, idx, result);
    }
    
    if op2 >= 5 && op2 < 9 {
        println!("\n[CPU] src2 memory access:");
        println!("  addr={}, mem_ctr_before={}", lit2, vm.memory_counter);
        
        // Calculate chunk index and read from rom.data
        let num_chunks = rom.data.len() / 64;
        let chunk_index = (lit2 as usize) % num_chunks;
        let chunk_start = chunk_index * 64;
        let chunk = &rom.data[chunk_start..chunk_start + 64];
        
        print!("  Chunk first 16 bytes: ");
        for i in 0..16 {
            print!("{:02x}", chunk[i]);
        }
        println!();
        
        // Simulate digest update and counter increment
        vm.mem_digest.update(chunk);
        vm.memory_counter += 1;
        let idx = ((vm.memory_counter % 8) as usize) * 8;
        let result = u64::from_le_bytes(*<&[u8; 8]>::try_from(&chunk[idx..idx + 8]).unwrap());
        println!("  mem_ctr_after={}, idx_in_chunk={}, result=0x{:016x}",
            vm.memory_counter, idx, result);
    }
}
