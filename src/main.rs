use clap::{Parser, Subcommand};
mod config;
mod init;

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
    println!("Loading configuration...");
    let mut config = config::load_config()?;

    println!("Resolving paths...");
    config::resolve_paths(&mut config)?;

    println!("Validating configuration...");
    config::validate_config(&config)?;

    println!("Discovering environments...");
    let discovered_envs = config::discover_environments(&config)?;
    println!("Found {} environments", discovered_envs.len());

    println!("Discovering extensions...");
    let discovered_exts = config::discover_extensions(&config)?;
    println!("Found {} extensions", discovered_exts.len());

    println!("Build preparation complete. Ready to build docker-compose files.");

    // TODO: Add actual build logic here

    Ok(())
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
