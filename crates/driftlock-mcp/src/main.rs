#![allow(missing_docs)]

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "driftlock-mcp", version, about = "Driftlock MCP stdio server")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run MCP over stdio.
    Stdio {
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Stdio { repo } => run_stdio(repo),
    }
}

fn run_stdio(repo: PathBuf) -> Result<()> {
    #[cfg(feature = "rmcp-sdk")]
    {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        runtime.block_on(driftlock_mcp::rmcp_adapter::serve_rmcp_stdio(repo))
    }
    #[cfg(all(feature = "manual-stdio", not(feature = "rmcp-sdk")))]
    {
        return driftlock_mcp::manual_stdio::serve(repo);
    }
    #[cfg(not(any(feature = "rmcp-sdk", feature = "manual-stdio")))]
    {
        let _ = repo;
        anyhow::bail!("driftlock-mcp built without a transport feature");
    }
}
