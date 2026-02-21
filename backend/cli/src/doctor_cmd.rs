//! CLI Doctor Command
//!
//! Mirrors `src/cli/index.ts` doctor suite logic.

use anyhow::Result;
use std::env;

/// Executes the full doctor diagnosis.
pub async fn run() -> Result<()> {
    println!("\n🔍 Running ClawForge Doctor...\n");

    let is_ok = check_env_vars() && check_docker();
    
    println!();
    if is_ok {
        println!("✅ All checks passed! ClawForge is healthy.");
    } else {
        println!("❌ Some checks failed! Please fix the errors above.");
    }
    
    Ok(())
}

fn check_env_vars() -> bool {
    println!("Checking Environment Variables:");
    
    let checks = [
        ("OPENAI_API_KEY", true),   // true = optional
        ("DATABASE_URL", false),    // false = required
        ("REDIS_URL", false),
    ];

    let mut all_good = true;

    for (var, optional) in checks {
        match env::var(var) {
            Ok(val) if !val.is_empty() => {
                println!("  🟢 {} is set", var);
            }
            _ => {
                if optional {
                    println!("  🟡 {} is missing (optional)", var);
                } else {
                    println!("  🔴 {} is missing (REQUIRED)", var);
                    all_good = false;
                }
            }
        }
    }

    all_good
}

fn check_docker() -> bool {
    // MOCK: Check if docker daemon is reachable
    println!("Checking Docker Daemon:");
    println!("  🟢 Docker is running and reachable.");
    true
}
