mod container;
mod logging;
mod oci;
mod state;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::Result;

#[derive(Parser)]
#[command(name = "rundmc", about = "Custom OCI runtime in Rust")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long)]
    root: Option<PathBuf>,

    #[arg(short, long)]
    log: Option<PathBuf>,

    #[arg(long)]
    log_format: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Create container")]
    Create {
        #[arg(short, long)]
        bundle: PathBuf,
        #[arg(required = true)]
        container_id: String,
        #[arg(long)]
        pid_file: Option<PathBuf>,
    },
    #[command(about = "Start container payload")]
    Start {
        #[arg(required = true)]
        container_id: String,
    },
    #[command(about = "Query the state of container")]
    State {
        #[arg(required = true)]
        container_id: String,
    },
    #[command(about = "Send a signal to container's init process")]
    Kill {
        #[arg(required = true)]
        container_id: String,
        #[arg(required = true)]
        signal: String,
        #[arg(long)]
        all: bool,
    },
    #[command(about = "Delete container")]
    Delete {
        #[arg(required = true)]
        container_id: String,
        #[arg(long)]
        force: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let log_config = logging::LogConfig::from_env()
        .with_file(cli.log.clone());
    logging::init_with_config(&log_config);

    log::info!(
        "rundmc invoked: command={:?}, root={:?}",
        std::env::args().nth(1).unwrap_or_default(),
        cli.root
    );
    log::info!("All args: {:?}", std::env::args().collect::<Vec<_>>());
    log::info!("Working directory: {:?}", std::env::current_dir());

    // Handle log file output (OCI spec: --log for structured logging)
    if let Some(ref log_path) = cli.log {
        log::debug!("OCI log output configured: {:?}", log_path);
        // TODO: Future - send structured JSON logs to this file
        // For now, all logs go to stderr
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
        Commands::Kill { container_id, signal, all } => {
            container::kill(&container_id, &signal, &root_dir, all)?;
        }
        Commands::Delete { container_id, force } => {
            container::delete(&container_id, &root_dir, force)?;
        }
    }
    Ok(())
}
