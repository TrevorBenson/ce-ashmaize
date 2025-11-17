// Debug VM initialization - compare what CPU computes

use ashmaize::{Rom, RomGenerationType, b2::VM};

const MB: usize = 1_024 * 1_024;

fn main() {
    println!("=== VM Initialization Debug ===\n");

    let rom = Rom::new(
        b"0",
        RomGenerationType::TwoStep {
            pre_size: 1 * MB,
            mixing_numbers: 1,
        },
        1 * MB,
    );

    let test_salts = vec![
        ("empty", b"" as &[u8]),
        ("a", b"a"),
        ("test", b"test"),
    ];

    for (name, salt) in test_salts {
        println!("Salt: '{}'", name);
        println!("  Salt bytes: {}", hex::encode(salt));
        println!("  Salt len: {}", salt.len());
        
        let vm = VM::new(&rom.digest, 256, salt);
        
        println!("  prog_seed[0..16]: {}", hex::encode(&vm.prog_seed[..16]));
        println!("  regs[0]: 0x{:016x}", vm.regs[0]);
        println!("  regs[1]: 0x{:016x}", vm.regs[1]);
        println!();
    }
    
    println!("\nROM digest: {}", hex::encode(&rom.digest.as_bytes()[..32]));
}

