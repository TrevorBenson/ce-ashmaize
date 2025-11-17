// Trace CPU's first 20 instructions to compare with GPU

use ashmaize::{Rom, RomGenerationType, b2::VM};

const MB: usize = 1_024 * 1_024;
const GB: usize = 1_024 * 1_024 * 1_024;

fn main() {
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

    let mut vm = VM::new(&rom.digest, 256, salt);
    vm.program.shuffle(&vm.prog_seed);
    
    println!("Tracing all 256 instructions of loop 0:\n");
    
    for instr_idx in 0..256 {
        let instr_bytes = *vm.program.at(vm.ip);
        
        let opcode = instr_bytes[0];
        let op1 = instr_bytes[1] >> 4;
        let op2 = instr_bytes[1] & 0x0F;
        let rs = ((instr_bytes[2] as u16) << 8) | (instr_bytes[3] as u16);
        let r3 = (rs as u8) & 0x1F;
        
        println!("[CPU Instr {}] opcode={}, r3={}, before: regs[{}]=0x{:016x}",
            instr_idx, opcode, r3, r3, vm.regs[r3 as usize]);
        
        vm.step(&rom);
        
        println!("                                after:  regs[{}]=0x{:016x}",
            r3, vm.regs[r3 as usize]);
    }
    
    println!("\n=== Compare with GPU output ===");
}

