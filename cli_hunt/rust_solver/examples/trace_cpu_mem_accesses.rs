// Trace CPU memory accesses to compare with GPU

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

    println!("Tracing first 20 CPU memory accesses:\n");
    
    let mut mem_access_count = 0;
    
    for loop_num in 0..8 {
        if loop_num > 0 {
            vm.program.shuffle(&vm.prog_seed);
        }
        
        for _ in 0..256 {
            let loop_ctr_before = vm.loop_counter;
            let ip_before = vm.ip;
            let mem_ctr_before = vm.memory_counter;
            
            vm.step(&rom);
            
            // If memory_counter increased, a memory access occurred
            let mem_accesses_in_step = vm.memory_counter - mem_ctr_before;
            
            if mem_accesses_in_step > 0 && mem_access_count < 20 {
                // Note: We can't easily get the exact address without modifying the library
                // But we can at least show when memory accesses occur
                for i in 0..mem_accesses_in_step {
                    println!("[CPU mem_{}] loop={}, ip={}", 
                        mem_ctr_before + i, loop_ctr_before, ip_before);
                    mem_access_count += 1;
                    if mem_access_count >= 20 {
                        break;
                    }
                }
            }
        }
        
        vm.post_instructions();
        
        if mem_access_count >= 20 {
            break;
        }
    }
    
    println!("\nNote: This trace shows loop and ip for each memory access");
    println!("Compare with GPU trace to find divergence");
}

