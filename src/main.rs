use clap::{Parser, Subcommand};

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
    Init,
    /// Build the stack
    Build,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            println!("Init command executed");
        }
        Commands::Build => {
            println!("Build command executed");
        }
    }

    Ok(())
}