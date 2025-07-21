// Simple library usage example showing the current working functionality
// This demonstrates how to use oxidowl as a library with the CLI functionality

use std::process::Command;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("OxidOwl Library Usage Example");
    println!("=============================");
    
    // First, let's check that our ontology file exists
    if !Path::new("greenhouse.owx").exists() {
        println!("Error: greenhouse.owx not found in current directory");
        println!("Please make sure the ontology file is present.");
        return Ok(());
    }
    
    println!("\n1. Testing subclasses query via CLI interface:");
    let output = Command::new("./target/release/oxidowl")
        .args(&[
            "query",
            "--input", "greenhouse.owx",
            "--query", "subclasses: Plant",
            "--namespace", "http://www.smolang.org/greenhouseDT#"
        ])
        .output()?;
    
    println!("Exit status: {}", output.status);
    println!("Output: {}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    println!("\n2. Testing disjoint union query via CLI interface:");
    let output = Command::new("./target/release/oxidowl")
        .args(&[
            "query",
            "--input", "greenhouse.owx", 
            "--query", "Operational or Maintenance or Overheating or Underheating",
            "--namespace", "http://www.smolang.org/greenhouseDT#"
        ])
        .output()?;
    
    println!("Exit status: {}", output.status);
    println!("Output: {}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
    }

    println!("\n3. Testing equivalent classes query via CLI interface:");
    let output = Command::new("./target/release/oxidowl")
        .args(&[
            "query",
            "--input", "greenhouse.owx",
            "--query", "equivalent-classes: Plant",
            "--namespace", "http://www.smolang.org/greenhouseDT#"
        ])
        .output()?;
    
    println!("Exit status: {}", output.status);
    println!("Output: {}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    println!("\nLibrary interface example completed!");
    println!("Note: This demonstrates the CLI functionality via library calls.");
    println!("More direct library interface methods are under development.");
    
    Ok(())
}
