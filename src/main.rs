mod merger;
mod env_merger;
mod error;
use clap::{Parser, Subcommand};
mod config;
mod init;
mod build;
mod file_copier;
mod build_cleaner;

#[cfg(test)]
mod tests;

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

use crate::error::{StackBuilderError, Result};

fn run_build() -> Result<()> {
    build::execute_build()
}

fn run_init(args: &init::InitArgs) -> Result<()> {
    init::run_init(args)
}

fn print_error(error: &StackBuilderError) {
    eprintln!("Error: {}", error);
    
    // Print suggestion if available
    if let Some(suggestion) = error.suggestion() {
        eprintln!("\nSuggestion: {}", suggestion);
    }
    
    // Add context for common error patterns
    if error.suggests_init() {
        eprintln!("\nTo create a new project, run:");
        eprintln!("  stackbuilder init");
    }
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init(args) => run_init(&args),
        Commands::Build => run_build(),
    };

    if let Err(error) = result {
        print_error(&error);
        std::process::exit(error.exit_code());
    }
}
