mod container;
mod oci;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::Result;

#[derive(Parser)]
#[command(name = "rundmc", about = "A custom OCI runtime in Rust")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long)]
    root: Option<PathBuf>,

    #[arg(short, long)]
    log: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Create a container")]
    Create {
        #[arg(short, long)]
        bundle: PathBuf,
        #[arg(required = true)]
        container_id: String,
    },
    #[command(about = "Start a container payload")]
    Start {
        #[arg(required = true)]
        container_id: String,
    },
    #[command(about = "Delete a container")]
    Delete {
        #[arg(required = true)]
        container_id: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging (simple placeholder for now)
    if let Some(log_path) = cli.log {
        eprintln!("Logging to {:?}", log_path);
    }

    match cli.command {
        Commands::Create { bundle, container_id } => {
            container::create(bundle, container_id)?;
        }
        Commands::Start { container_id } => {
            println!("Starting container: {}", container_id);
            // TODO: Signal init process to start
        }
        Commands::Delete { container_id } => {
            println!("Deleting container: {}", container_id);
            // TODO: Cleanup resources
        }
    }
    Ok(())
}
