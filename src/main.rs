use clap::Parser;
use rusty_awa::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Cli::parse().run()?;
    Ok(())
}
