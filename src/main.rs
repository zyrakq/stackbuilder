mod merger;
use clap::{Parser, Subcommand};
mod config;
mod init;
mod build;

#[derive(Parser)]
#[command(name = "stackbuilder")]
#[command(version)]
#[command(about = "A tool for building docker-compose files from modular components")]
#[command(long_about = "Stackbuilder is a CLI tool designed to build docker-compose files from modular components.\n\nExamples:\n  stackbuilder init --name my-project\n  stackbuilder build --config ./config.yml")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new stackbuilder project with default configuration and folder structure
    Init(init::InitArgs),
    /// Build docker-compose files by merging base, environment and extension components
    Build,
}

fn run_build() -> anyhow::Result<()> {
    build::execute_build()
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init(args) => {
            if let Err(e) = init::run_init(&args) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Build => {
            if let Err(e) = run_build() {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
