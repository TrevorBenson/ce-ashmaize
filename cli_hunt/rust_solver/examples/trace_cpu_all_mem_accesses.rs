// Trace ALL CPU memory accesses

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

    println!("Tracing ALL CPU memory accesses:\n");
    
    for loop_num in 0..8 {
        if loop_num > 0 {
            vm.program.shuffle(&vm.prog_seed);
        }
        
        for _ in 0..256 {
            let loop_ctr_before = vm.loop_counter;
            let ip_before = vm.ip;
            let mem_ctr_before = vm.memory_counter;
            
            vm.step(&rom);
            
            let mem_accesses_in_step = vm.memory_counter - mem_ctr_before;
            
            if mem_accesses_in_step > 0 {
                for i in 0..mem_accesses_in_step {
                    println!("[CPU mem_{}] loop={}, ip={}", 
                        mem_ctr_before + i, loop_ctr_before, ip_before);
                }
            }
        }
        
        vm.post_instructions();
    }
    
    println!("\nFinal CPU memory_counter: {}", vm.memory_counter);
}

