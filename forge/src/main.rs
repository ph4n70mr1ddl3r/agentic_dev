use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod agents;
mod config;
mod github;
mod issues;
mod llm;
mod plan;
mod render;
mod schema;
mod util;

#[derive(Parser)]
#[command(
    name = "forge",
    version,
    about = "Runs the virtual software company of AI agents"
)]
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
        /// Disable DeepSeek thinking mode. The CEO thinks by default; pass this
        /// to run the cheaper non-thinking path.
        #[arg(long)]
        no_thinking: bool,
    },
    /// Sync the CEO plan's first-phase tasks into GitHub Issues (labels,
    /// milestone, dependencies in the body). Idempotent: tasks that already
    /// have an open issue are skipped. Default is a dry run.
    Sync {
        /// Path to the CEO-authored plan.json, relative to --repo.
        #[arg(long, default_value = "docs/company/plan.json")]
        plan: PathBuf,
        /// Create the issues for real (default: dry-run, just print).
        #[arg(long)]
        write: bool,
        /// Override the GitHub repo as owner/name (default: auto-detect from
        /// git's `origin` remote).
        #[arg(long)]
        github_repo: Option<String>,
    },
    /// Run a single task's owning hat to produce a schema-validated artifact.
    /// (Currently only the Architect hat / entity artifacts are implemented.)
    Run {
        /// Task id from the plan, e.g. T3 (case-insensitive).
        task: String,
        /// Output dir for the artifact, relative to --repo (written as <entity-id>.json).
        #[arg(long, default_value = "modules/generated")]
        out: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    config::load_env();

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("forge=info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let cli = Cli::parse();
    match cli.command {
        Command::Ceo {
            brief,
            out,
            write,
            no_thinking,
        } => {
            let mut config = config::Config::from_env()?;
            // The CEO thinks by default (one-shot planning; quality > cost here).
            config.thinking = !no_thinking;
            config.finalize_reasoning();
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

            let adrs = read_adrs(&resolve(&cli.repo, std::path::Path::new("docs/adr")));

            let plan = agents::ceo::run_ceo(&llm, &brief_text, &adrs).await?;
            plan.validate()?;
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
        Command::Sync {
            plan,
            write,
            github_repo,
        } => {
            let plan_path = resolve(&cli.repo, &plan);
            let plan_text = std::fs::read_to_string(&plan_path)
                .map_err(|e| anyhow::anyhow!("reading plan {}: {e}", plan_path.display()))?;
            let company_plan: plan::CompanyPlan = serde_json::from_str(&plan_text)
                .with_context(|| format!("parse plan {}", plan_path.display()))?;

            let repo = match github_repo.as_deref() {
                Some(s) => github::parse_repo(s).ok_or_else(|| {
                    anyhow::anyhow!("invalid --github-repo {s:?} (expected owner/name)")
                })?,
                None => github::detect_repo().ok_or_else(|| {
                    anyhow::anyhow!(
                        "could not detect GitHub repo from git origin; \
                         pass --github-repo owner/name"
                    )
                })?,
            };

            tracing::info!(repo = %repo.slug(), write, "syncing first-phase issues");

            if write {
                let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
                    anyhow::anyhow!("GITHUB_TOKEN is not set (needs Issues: write)")
                })?;
                let gh = github::GitHub::new(token, repo)?;
                issues::run_sync(&company_plan, &gh, 1).await?;
            } else {
                issues::run_sync_dry(&company_plan, &repo, 1)?;
            }
            Ok(())
        }
        Command::Run { task: task_id, out } => {
            let mut config = config::Config::from_env()?;
            config.finalize_reasoning();
            tracing::info!(
                model = %config.model,
                thinking = config.thinking,
                task = %task_id,
                "running hat"
            );
            let llm = llm::Llm::new(config)?;

            let plan_path = resolve(&cli.repo, std::path::Path::new("docs/company/plan.json"));
            let plan_text = std::fs::read_to_string(&plan_path)
                .map_err(|e| anyhow::anyhow!("reading plan {}: {e}", plan_path.display()))?;
            let company_plan: plan::CompanyPlan = serde_json::from_str(&plan_text)
                .with_context(|| format!("parse plan {}", plan_path.display()))?;
            let task = company_plan
                .first_phase
                .tasks
                .iter()
                .find(|t| t.id.eq_ignore_ascii_case(&task_id))
                .ok_or_else(|| anyhow::anyhow!("task {task_id:?} not found in first phase"))?;

            let schemas_dir = resolve(&cli.repo, std::path::Path::new("platform-spec/schemas"));
            let examples_dir = resolve(&cli.repo, std::path::Path::new("platform-spec/examples"));
            let registry = schema::SchemaRegistry::load_dir(&schemas_dir)?;

            let ctx = agents::HatContext {
                llm: &llm,
                registry: &registry,
                examples_dir,
            };
            tracing::info!(task = %task.id, role = %task.role, "dispatching hat");
            let artifact = agents::run_task(task, &ctx).await?;

            let id = artifact
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("artifact");
            let out_dir = resolve(&cli.repo, &out);
            std::fs::create_dir_all(&out_dir)?;
            let file = out_dir.join(format!("{id}.json"));
            std::fs::write(
                &file,
                format!("{}\n", serde_json::to_string_pretty(&artifact)?),
            )?;
            println!("wrote {}", file.display());
            tracing::info!(artifact = %file.display(), "hat produced a schema-validated artifact");
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

/// Concatenate all accepted ADRs (excluding the template) under `dir`.
///
/// ADRs are part of the founders' seed; the CEO must plan within them. If the
/// directory can't be read we warn and return an empty string so the CEO can
/// still run against a repo that only has a brief.
fn read_adrs(dir: &std::path::Path) -> String {
    let read = || -> Result<String> {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
            .with_context(|| format!("listing ADR dir {}", dir.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension().and_then(|s| s.to_str()) == Some("md")
                    && p.file_name().and_then(|s| s.to_str()) != Some("0000-template.md")
            })
            .collect();
        paths.sort();

        let mut out = String::new();
        for p in &paths {
            let body = std::fs::read_to_string(p)
                .with_context(|| format!("reading ADR {}", p.display()))?;
            out.push_str(&body);
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
        }
        Ok(out)
    };
    match read() {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "could not read ADRs; continuing with brief only");
            String::new()
        }
    }
}
