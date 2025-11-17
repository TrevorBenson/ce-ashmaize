// Trace instruction execution to compare with GPU

use ashmaize::{Rom, RomGenerationType, b2::VM};

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * 1_024 * 1_024;

fn main() {
    println!("=== Instruction Execution Trace - CPU ===\n");

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
    
    // Execute first 1 instruction with VERY detailed tracing
    let instr_bytes = *vm.program.at(0);
    
    // Decode instruction properly
    let opcode = instr_bytes[0];
    let op1 = instr_bytes[1] >> 4;
    let op2 = instr_bytes[1] & 0x0F;
    let rs = ((instr_bytes[2] as u16) << 8) | (instr_bytes[3] as u16);
    let r1 = ((rs >> 10) as u8) & 0x1F;
    let r2 = ((rs >> 5) as u8) & 0x1F;
    let r3 = (rs as u8) & 0x1F;
    let lit1 = u64::from_le_bytes(*<&[u8; 8]>::try_from(&instr_bytes[4..12]).unwrap());
    let lit2 = u64::from_le_bytes(*<&[u8; 8]>::try_from(&instr_bytes[12..20]).unwrap());
    
    println!("[CPU] Loop 0, Instr 0: opcode={}, op1={}, op2={}, r1={}, r2={}, r3={}",
        opcode, op1, op2, r1, r2, r3);
    println!("  lit1=0x{:016x}, lit2=0x{:016x}", lit1, lit2);
    println!("  Before: regs[0]=0x{:016x}, regs[1]=0x{:016x}, mem_ctr={}",
        vm.regs[0], vm.regs[1], vm.memory_counter);
    
    vm.step(&rom);
    
    println!("  After: regs[{}]=0x{:016x}, regs[0]=0x{:016x}, regs[1]=0x{:016x}, mem_ctr={}\n",
        r3, vm.regs[r3 as usize], vm.regs[0], vm.regs[1], vm.memory_counter);
}

