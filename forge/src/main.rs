use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

mod agents;
mod config;
mod llm;
mod plan;
mod render;

#[derive(Parser)]
#[command(name = "forge", version, about = "Runs the virtual software company of AI agents")]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Path to the erp product repo (where the brief and company docs live).
    #[arg(long, global = true, default_value = "erp")]
    repo: PathBuf,
}

#[derive(Subcommand)]
enum Command {
    /// Run the CEO hat: read the brief and produce the company plan.
    Ceo {
        /// Path to the company brief, relative to --repo.
        #[arg(long, default_value = "docs/company-brief.md")]
        brief: PathBuf,
        /// Output dir for the CEO-authored company docs, relative to --repo.
        #[arg(long, default_value = "docs/company")]
        out: PathBuf,
        /// Write the plan into the repo (otherwise print JSON to stdout).
        #[arg(long)]
        write: bool,
        /// Enable DeepSeek thinking mode for this run (slower/more tokens,
        /// may improve plan quality).
        #[arg(long)]
        thinking: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("forge=info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let cli = Cli::parse();
    match cli.command {
        Command::Ceo {
            brief,
            out,
            write,
            thinking,
        } => {
            let mut config = config::Config::from_env()?;
            if thinking {
                config.thinking = true;
            }
            tracing::info!(
                model = %config.model,
                base = %config.base_url,
                thinking = config.thinking,
                repo = %cli.repo.display(),
                "running CEO hat"
            );
            let llm = llm::Llm::new(config)?;

            let brief_path = resolve(&cli.repo, &brief);
            let brief_text = std::fs::read_to_string(&brief_path)
                .map_err(|e| anyhow::anyhow!("reading brief {}: {e}", brief_path.display()))?;

            let plan = agents::ceo::run_ceo(&llm, &brief_text).await?;
            tracing::info!(
                hats = plan.organization.hats.len(),
                phases = plan.roadmap.len(),
                first_phase_tasks = plan.first_phase.tasks.len(),
                "CEO plan produced"
            );

            if write {
                let out_dir = resolve(&cli.repo, &out);
                render::render(&plan, &out_dir)?;
                tracing::info!(dir = %out_dir.display(), "wrote company plan");
            } else {
                println!("{}", serde_json::to_string_pretty(&plan)?);
            }
            Ok(())
        }
    }
}

fn resolve(repo: &std::path::Path, p: &std::path::Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        repo.join(p)
    }
}
