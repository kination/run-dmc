
mod container;
mod oci;

use clap::{Parser, Subcommand};

#[derive(Subcommand)]
enum Command {
    Run {
        config: String
    }
}

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}


fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Run { config} => {
            container::run(&config).unwrap();
        }
    }
}
