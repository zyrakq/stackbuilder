mod merger;
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

    // Test the merger functionality
fn test_merger(config: &config::Config) -> anyhow::Result<()> {
    use merger::{ComposeMerger, merge_compose_files, merge_yaml_values, load_compose_file};

    println!("Testing merger functionality...");

    // Test 1: Basic base merge
    let base_file = "components/base/docker-compose.yml";
    let base_value = load_compose_file(base_file)?;
    println!("✓ Loaded base file: {}", base_file);

    // Test 2: Base + Environment merge
    let merger = ComposeMerger::new(
        "components/base".to_string(),
        "components/environments".to_string(),
        vec!["components/extensions".to_string()],
    );

    let result_env = merge_compose_files(&merger, Some("dev"), &[])?;
    println!("✓ Merged base + environment dev");

    // Test 3: Base + Extensions merge
    let result_ext = merge_compose_files(&merger, None, &["monitoring".to_string()])?;
    println!("✓ Merged base + monitoring extension");

    // Test 4: Full merge: Base + Environment + Extensions
    let result_full = merge_compose_files(&merger, Some("dev"), &["monitoring".to_string(), "security".to_string()])?;
    println!("✓ Merged base + dev + monitoring + security");

    // Print key parts of the full result to verify overrides
    if let Some(services) = result_full.get("services") {
        println!("Services in final merge:");
        for (service_name, service_config) in services.as_mapping().unwrap() {
            println!("  - {}", service_name.as_str().unwrap());
        }
    }

    Ok(())
}
    test_merger(&config)?;

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
