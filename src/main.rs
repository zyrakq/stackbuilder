use clap::{Parser, Subcommand};
mod config;
mod init;

#[derive(Parser)]
#[command(name = "stackbuilder")]
#[command(about = "Docker Compose Stack Builder")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new stack
    Init(init::InitArgs),
    /// Build the stack
    Build,
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
            println!("Build command executed");
        }
    }

    Ok(())
}
