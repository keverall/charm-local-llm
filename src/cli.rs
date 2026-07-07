use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "kcharm")]
#[command(about = "Setup and optimize KCharm for local Ollama LLMs (CachyOS RTX 4090)", long_about = None)]
#[command(version = "0.1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start the Ollama environment (SOD - Start of Day)
    Start(StartArgs),
    /// Stop the Ollama environment (EOD - End of Day)
    Stop(StopArgs),
    /// Show status of services and models
    Status(StatusArgs),
    /// Manage models (list, ensure, remove)
    Models(ModelsArgs),
    /// Manage Ollama systemd service
    Service(ServiceArgs),
    /// Manage Qdrant vector database
    Qdrant(QdrantArgs),
    /// Manage Crush coding assistant config
    Crush(CrushArgs),
    /// Manage Kilocode config and context
    Kilo(KiloArgs),
}

#[derive(Parser)]
pub struct StartArgs {
    /// Dry run (simulate without making changes)
    #[arg(long)]
    pub dry_run: bool,

    /// Platform override (auto, macos, cachyos, linux)
    #[arg(long)]
    pub platform_override: Option<String>,

    /// Project root directory
    #[arg(long)]
    pub project_root: Option<PathBuf>,
}

#[derive(Parser)]
pub struct StopArgs {
    /// Dry run
    #[arg(long)]
    pub dry_run: bool,

    /// Platform override
    #[arg(long)]
    pub platform_override: Option<String>,
}

#[derive(Parser)]
pub struct StatusArgs {
    /// Show JSON output
    #[arg(long)]
    pub json: bool,
}

#[derive(Subcommand)]
pub enum ModelsAction {
    /// List available models
    List,
    /// Ensure a model is present (pull or create from modelfile)
    Ensure {
        /// Model name (e.g. qwen3-coder:30b-gpu)
        model: String,
    },
    /// Remove a model
    Remove {
        /// Model name
        model: String,
    },
}

#[derive(Parser)]
pub struct ModelsArgs {
    #[command(subcommand)]
    pub action: ModelsAction,
}

#[derive(Subcommand)]
pub enum ServiceAction {
    /// Start the Ollama systemd service
    Start,
    /// Stop the Ollama systemd service
    Stop,
    /// Restart the Ollama systemd service
    Restart,
    /// Show status of the Ollama service
    Status,
}

#[derive(Parser)]
pub struct ServiceArgs {
    #[command(subcommand)]
    pub action: ServiceAction,

    /// Dry run
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Subcommand)]
pub enum QdrantAction {
    /// Start Qdrant via docker-compose
    Start,
    /// Stop Qdrant via docker-compose
    Stop,
    /// Check Qdrant status
    Status,
}

#[derive(Parser)]
pub struct QdrantArgs {
    #[command(subcommand)]
    pub action: QdrantAction,

    /// Dry run
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Subcommand)]
pub enum CrushAction {
    /// Generate or update ~/.config/crush/crush.json for local Ollama
    Init,
    /// Show current Crush config status
    Status,
    /// Generate CRUSH.md project context file
    Context,
}

#[derive(Parser)]
pub struct CrushArgs {
    #[command(subcommand)]
    pub action: CrushAction,

    /// Project root directory (for CRUSH.md)
    #[arg(long)]
    pub project_root: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum KiloAction {
    /// Remove unsupported indexing block from ~/.config/kilo/kilo.json
    Init,
    /// Show current Kilo config status
    Status,
    /// Generate AGENTS.md project context file
    Context,
}

#[derive(Parser)]
pub struct KiloArgs {
    #[command(subcommand)]
    pub action: KiloAction,

    /// Project root directory (for AGENTS.md)
    #[arg(long)]
    pub project_root: Option<PathBuf>,
}
