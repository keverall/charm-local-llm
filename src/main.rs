use charm_local_llm::cli::Cli;
use charm_local_llm::commands;
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    commands::execute(cli).await
}
