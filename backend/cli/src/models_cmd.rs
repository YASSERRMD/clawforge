//! CLI Models Command
//!
//! Lists available connected LLMs.

use anyhow::Result;

pub async fn run() -> Result<()> {
    println!("\n🧠 Configured LLM Providers & Models\n");
    
    // MOCK: Fetch from config or registry
    
    println!("Provider: OpenAI");
    println!("  - gpt-4-turbo (Ctx: 128k, Price: $10/1M In)");
    println!("  - gpt-3.5-turbo (Ctx: 16k, Price: $0.5/1M In)\n");

    println!("Provider: Anthropic");
    println!("  - claude-3-opus-20240229 (Ctx: 200k, Price: $15/1M In)");
    println!("  - claude-3-haiku-20240307 (Ctx: 200k, Price: $0.25/1M In)\n");

    Ok(())
}
