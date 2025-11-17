// Test using ashmaize's internal Blake2b (which uses the blake2 crate)

fn main() {
    println!("=== Test CPU's blake2 via ashmaize ===\n");
    
    // The exact input bytes we traced
    let input_hex = "ed786b899eaaaa5796ea65b0feb6bce8";
    let input = hex::decode(input_hex).unwrap();
    
    println!("Input (16 bytes): {}", input_hex);
    println!("Input length: {} bytes", input.len());
    
    // We can't directly call Blake2b512::digest here, but we know:
    // - The GPU's blake2b produced: 0x5694e7110ab9f4d3 for chunk 2
    // - The b2sum tool produced: 0x5694e7110ab9f4d3 for chunk 2
    // - So the correct answer is: 0x5694e7110ab9f4d3
    
    println!("\n=== KNOWN FACTS ===");
    println!("1. GPU Blake2b chunk 2:  0x5694e7110ab9f4d3");
    println!("2. b2sum chunk 2:        0x5694e7110ab9f4d3");
    println!("3. CPU instr 0 result:   0x5555698eee0fbaef");
    
    println!("\n=== CONCLUSION ===");
    println!("The CPU VM is NOT computing Blake2b with src1=0x57aaaa9e896b78ed and src2=0xe8bcb6feb065ea96");
    println!("OR the CPU VM is executing a DIFFERENT instruction first!");
    println!("\nNext step: Trace what the CPU VM ACTUALLY does for the first instruction.");
}
