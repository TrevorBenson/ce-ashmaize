// Trace what Blake2b input the CPU ACTUALLY uses by computing it manually
// following EXACTLY what the CPU code does

use ashmaize::{Rom, RomGenerationType, b2::VM};

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * 1_024 * 1_024;

fn main() {
    println!("=== Trace CPU's ACTUAL Blake2b Input ===\n");

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

    println!("Creating VM and shuffling program...");
    let mut vm = VM::new(&rom.digest, 256, salt);
    vm.program.shuffle(&vm.prog_seed);
    
    println!("Initial VM state:");
    println!("  ip: {}", vm.ip);
    println!("  memory_counter: {}", vm.memory_counter);
    println!("  loop_counter: {}", vm.loop_counter);
    println!("  regs[0]: 0x{:016x}", vm.regs[0]);
    println!("  regs[1]: 0x{:016x}", vm.regs[1]);
    
    // Get instruction at IP (should be 0)
    let instr_bytes = *vm.program.at(vm.ip);
    
    println!("\nInstruction at IP={}:", vm.ip);
    print!("  Raw bytes: ");
    for b in &instr_bytes {
        print!("{:02x}", b);
    }
    println!();
    
    // Decode
    let opcode = instr_bytes[0];
    let op1 = instr_bytes[1] >> 4;
    let op2 = instr_bytes[1] & 0x0F;
    let rs = ((instr_bytes[2] as u16) << 8) | (instr_bytes[3] as u16);
    let r1 = ((rs >> 10) as u8) & 0x1F;
    let r2 = ((rs >> 5) as u8) & 0x1F;
    let r3 = (rs as u8) & 0x1F;
    let lit1 = u64::from_le_bytes(*<&[u8; 8]>::try_from(&instr_bytes[4..12]).unwrap());
    let lit2 = u64::from_le_bytes(*<&[u8; 8]>::try_from(&instr_bytes[12..20]).unwrap());
    
    println!("  Decoded: opcode={}, op1={}, op2={}, r1={}, r2={}, r3={}", 
        opcode, op1, op2, r1, r2, r3);
    println!("  lit1=0x{:016x}, lit2=0x{:016x}", lit1, lit2);
    
    // Now trace what src1 and src2 SHOULD be based on CPU logic
    println!("\nOperand evaluation:");
    
    // src1 (op1=11 means Literal)
    let src1 = if op1 < 5 {
        let val = vm.regs[r1 as usize];
        println!("  src1 = regs[{}] = 0x{:016x}", r1, val);
        val
    } else if op1 < 9 {
        println!("  src1 = Memory access at lit1");
        0 // placeholder
    } else if op1 < 13 {
        println!("  src1 = Literal = 0x{:016x}", lit1);
        lit1
    } else if op1 < 14 {
        println!("  src1 = Special1");
        0
    } else {
        println!("  src1 = Special2");
        0
    };
    
    // src2 (op2=6 means Memory)
    let src2 = if op2 < 5 {
        let val = vm.regs[r2 as usize];
        println!("  src2 = regs[{}] = 0x{:016x}", r2, val);
        val
    } else if op2 < 9 {
        println!("  src2 = Memory access at lit2=0x{:016x}", lit2);
        // Manually compute like CPU does
        let num_chunks = rom.data.len() / 64;
        let chunk_index = (lit2 as usize) % num_chunks;
        let chunk_start = chunk_index * 64;
        let chunk = &rom.data[chunk_start..chunk_start + 64];
        
        // CPU: increments memory_counter FIRST, then uses it for idx
        let memory_counter_after = vm.memory_counter + 1;
        let idx = ((memory_counter_after % 8) as usize) * 8;
        let val = u64::from_le_bytes(*<&[u8; 8]>::try_from(&chunk[idx..idx + 8]).unwrap());
        println!("    chunk_index={}, memory_counter_after={}, idx={}", 
            chunk_index, memory_counter_after, idx);
        println!("    Extracted value: 0x{:016x}", val);
        val
    } else if op2 < 13 {
        println!("  src2 = Literal = 0x{:016x}", lit2);
        lit2
    } else if op2 < 14 {
        println!("  src2 = Special1");
        0
    } else {
        println!("  src2 = Special2");
        0
    };
    
    println!("\nFinal operands:");
    println!("  src1 = 0x{:016x}", src1);
    println!("  src2 = 0x{:016x}", src2);
    
    println!("\n=== COMPARISON ===");
    println!("GPU src1: 0x57aaaa9e896b78ed");
    println!("CPU src1: 0x{:016x}", src1);
    println!("GPU src2: 0xe8bcb6feb065ea96");
    println!("CPU src2: 0x{:016x}", src2);
    
    if src1 == 0x57aaaa9e896b78ed && src2 == 0xe8bcb6feb065ea96 {
        println!("\n✓ MATCH - Operands are identical!");
        println!("The Blake2b crate must be producing different results!");
    } else {
        println!("\n✗ MISMATCH - This is where CPU and GPU diverge!");
    }
}

