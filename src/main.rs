mod container;
mod oci;
mod state;

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
        #[arg(long)]
        pid_file: Option<PathBuf>,
    },
    #[command(about = "Start a container payload")]
    Start {
        #[arg(required = true)]
        container_id: String,
    },
    #[command(about = "Query the state of a container")]
    State {
        #[arg(required = true)]
        container_id: String,
    },
    #[command(about = "Send a signal to the container's init process")]
    Kill {
        #[arg(required = true)]
        container_id: String,
        #[arg(required = true)]
        signal: String,
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

    let root_dir = cli.root.unwrap_or_else(|| PathBuf::from("/run/rundmc"));

    match cli.command {
        Commands::Create { bundle, container_id, pid_file } => {
            container::create(bundle, container_id, root_dir, pid_file)?;
        }
        Commands::Start { container_id } => {
            container::start(&container_id, &root_dir)?;
        }
        Commands::State { container_id } => {
            container::state(&container_id, &root_dir)?;
        }
        Commands::Kill { container_id, signal } => {
            container::kill(&container_id, &signal, &root_dir)?;
        }
        Commands::Delete { container_id } => {
            container::delete(&container_id, &root_dir)?;
        }
    }
    Ok(())
}
