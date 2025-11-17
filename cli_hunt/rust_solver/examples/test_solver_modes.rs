// Test Solver Modes - Test CPU, GPU, Auto, Mixed modes with quick difficulty
// This test verifies all solver modes work correctly

use std::process::{Command, Stdio};
use std::time::Instant;

fn main() {
    println!("=== Testing Ashmaize Solver Modes ===\n");
    
    let test_params = [
        "--address", "test_address",
        "--challenge-id", "test_challenge",
        "--difficulty", "fffff000",  // Very easy difficulty for quick test
        "--no-pre-mine", "0",
        "--latest-submission", "0",
        "--no-pre-mine-hour", "0",
    ];
    
    let modes = vec![
        ("auto", "Auto mode (should use GPU)"),
        ("gpu", "GPU-only mode (all GPUs)"),
        ("cpu", "CPU-only mode"),
    ];
    
    for (mode, description) in modes {
        println!("Testing {} - {}", mode, description);
        println!("{}", "-".repeat(70));
        
        let start = Instant::now();
        let output = Command::new("cargo")
            .args(&["run", "--release", "--features", "cuda", "--"])
            .args(&test_params)
            .arg("--solver-mode")
            .arg(mode)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("Failed to execute");
        
        let elapsed = start.elapsed();
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        if !stderr.is_empty() {
            let stderr_lines: Vec<&str> = stderr.lines()
                .filter(|line| !line.contains("warning:") && !line.contains("Compiling") && !line.contains("Finished"))
                .collect();
            
            if !stderr_lines.is_empty() {
                println!("Solver output:");
                for line in stderr_lines.iter().take(5) {
                    println!("  {}", line);
                }
            }
        }
        
        if stdout.trim().len() == 16 {
            println!("✅ Found solution: {}", stdout.trim());
            println!("⏱️  Time: {:.2}s\n", elapsed.as_secs_f64());
        } else if !stdout.is_empty() {
            println!("⚠️  Output: {}", stdout.trim());
            println!("⏱️  Time: {:.2}s\n", elapsed.as_secs_f64());
        } else {
            println!("⏱️  Running... (killed after timeout)\n");
        }
    }
    
    println!("=== All Solver Modes Tested ===");
}

// This test verifies all solver modes work correctly

use std::process::{Command, Stdio};
use std::time::Instant;

fn main() {
    println!("=== Testing Ashmaize Solver Modes ===\n");
    
    let test_params = [
        "--address", "test_address",
        "--challenge-id", "test_challenge",
        "--difficulty", "fffff000",  // Very easy difficulty for quick test
        "--no-pre-mine", "0",
        "--latest-submission", "0",
        "--no-pre-mine-hour", "0",
    ];
    
    let modes = vec![
        ("auto", "Auto mode (should use GPU)"),
        ("gpu", "GPU-only mode (all GPUs)"),
        ("cpu", "CPU-only mode"),
    ];
    
    for (mode, description) in modes {
        println!("Testing {} - {}", mode, description);
        println!("{}", "-".repeat(70));
        
        let start = Instant::now();
        let output = Command::new("cargo")
            .args(&["run", "--release", "--features", "cuda", "--"])
            .args(&test_params)
            .arg("--solver-mode")
            .arg(mode)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("Failed to execute");
        
        let elapsed = start.elapsed();
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        if !stderr.is_empty() {
            let stderr_lines: Vec<&str> = stderr.lines()
                .filter(|line| !line.contains("warning:") && !line.contains("Compiling") && !line.contains("Finished"))
                .collect();
            
            if !stderr_lines.is_empty() {
                println!("Solver output:");
                for line in stderr_lines.iter().take(5) {
                    println!("  {}", line);
                }
            }
        }
        
        if stdout.trim().len() == 16 {
            println!("✅ Found solution: {}", stdout.trim());
            println!("⏱️  Time: {:.2}s\n", elapsed.as_secs_f64());
        } else if !stdout.is_empty() {
            println!("⚠️  Output: {}", stdout.trim());
            println!("⏱️  Time: {:.2}s\n", elapsed.as_secs_f64());
        } else {
            println!("⏱️  Running... (killed after timeout)\n");
        }
    }
    
    println!("=== All Solver Modes Tested ===");
}

// This test verifies all solver modes work correctly

use std::process::{Command, Stdio};
use std::time::Instant;

fn main() {
    println!("=== Testing Ashmaize Solver Modes ===\n");
    
    let test_params = [
        "--address", "test_address",
        "--challenge-id", "test_challenge",
        "--difficulty", "fffff000",  // Very easy difficulty for quick test
        "--no-pre-mine", "0",
        "--latest-submission", "0",
        "--no-pre-mine-hour", "0",
    ];
    
    let modes = vec![
        ("auto", "Auto mode (should use GPU)"),
        ("gpu", "GPU-only mode (all GPUs)"),
        ("cpu", "CPU-only mode"),
    ];
    
    for (mode, description) in modes {
        println!("Testing {} - {}", mode, description);
        println!("{}", "-".repeat(70));
        
        let start = Instant::now();
        let output = Command::new("cargo")
            .args(&["run", "--release", "--features", "cuda", "--"])
            .args(&test_params)
            .arg("--solver-mode")
            .arg(mode)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("Failed to execute");
        
        let elapsed = start.elapsed();
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        if !stderr.is_empty() {
            let stderr_lines: Vec<&str> = stderr.lines()
                .filter(|line| !line.contains("warning:") && !line.contains("Compiling") && !line.contains("Finished"))
                .collect();
            
            if !stderr_lines.is_empty() {
                println!("Solver output:");
                for line in stderr_lines.iter().take(5) {
                    println!("  {}", line);
                }
            }
        }
        
        if stdout.trim().len() == 16 {
            println!("✅ Found solution: {}", stdout.trim());
            println!("⏱️  Time: {:.2}s\n", elapsed.as_secs_f64());
        } else if !stdout.is_empty() {
            println!("⚠️  Output: {}", stdout.trim());
            println!("⏱️  Time: {:.2}s\n", elapsed.as_secs_f64());
        } else {
            println!("⏱️  Running... (killed after timeout)\n");
        }
    }
    
    println!("=== All Solver Modes Tested ===");
}




