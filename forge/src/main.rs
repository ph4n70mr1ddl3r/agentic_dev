use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod agents;
mod config;
mod git;
mod github;
mod issues;
mod llm;
mod orchestrator;
mod plan;
mod pr;
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
    /// Run a hat to produce a schema-validated artifact. With a task id, runs
    /// that one task; without, orchestrates the whole first phase (DAG-aware,
    /// runs every task whose role has a hat and whose deps are satisfied).
    Run {
        /// Task id from the plan, e.g. T3 (case-insensitive). Omit to run the phase.
        task: Option<String>,
        /// Output dir for artifacts, relative to --repo (written as <id>.json).
        #[arg(long, default_value = "modules/generated")]
        out: PathBuf,
        /// Open a GitHub PR for each artifact (needs GITHUB_TOKEN; clean tree).
        #[arg(long)]
        pr: bool,
        /// Base branch for PRs (default: main).
        #[arg(long, default_value = "main")]
        base: String,
    },
    /// Execute a test-plan artifact: validate each assertion's sample against its
    /// schema and report (exits non-zero on any failure).
    Check {
        /// Path to a test-plan JSON file, relative to --repo.
        plan: PathBuf,
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
        Command::Run {
            task: task_id,
            out,
            pr,
            base,
        } => {
            let mut config = config::Config::from_env()?;
            config.finalize_reasoning();
            tracing::info!(
                model = %config.model,
                thinking = config.thinking,
                "running hat"
            );
            let llm = llm::Llm::new(config)?;

            let plan_path = resolve(&cli.repo, std::path::Path::new("docs/company/plan.json"));
            let plan_text = std::fs::read_to_string(&plan_path)
                .map_err(|e| anyhow::anyhow!("reading plan {}: {e}", plan_path.display()))?;
            let company_plan: plan::CompanyPlan = serde_json::from_str(&plan_text)
                .with_context(|| format!("parse plan {}", plan_path.display()))?;

            let schemas_dir = resolve(&cli.repo, std::path::Path::new("platform-spec/schemas"));
            let examples_dir = resolve(&cli.repo, std::path::Path::new("platform-spec/examples"));
            let registry = schema::SchemaRegistry::load_dir(&schemas_dir)?;
            let out_dir = resolve(&cli.repo, &out);

            // --pr: require a token + a clean tree so each PR is scoped to one artifact.
            let gh = if pr {
                if !git::is_clean()? {
                    return Err(anyhow::anyhow!(
                        "--pr requires a clean working tree; commit or stash first"
                    ));
                }
                let token = std::env::var("GITHUB_TOKEN").map_err(|_| {
                    anyhow::anyhow!(
                        "GITHUB_TOKEN is not set (needed for --pr; e.g. $(gh auth token))"
                    )
                })?;
                let repo = github::detect_repo().ok_or_else(|| {
                    anyhow::anyhow!("could not detect GitHub repo from git origin")
                })?;
                Some(github::GitHub::new(token, repo)?)
            } else {
                None
            };
            let pr_ctx = gh.as_ref().map(|g| (g, base.as_str()));
            if pr {
                tracing::info!(base = %base, branch = %git::current_branch()?, "PR mode");
            }

            let ctx = agents::HatContext {
                llm: &llm,
                registry: &registry,
                examples_dir,
                out_dir: out_dir.clone(),
            };

            match task_id {
                Some(task_id) => {
                    let task = company_plan
                        .first_phase
                        .tasks
                        .iter()
                        .find(|t| t.id.eq_ignore_ascii_case(&task_id))
                        .ok_or_else(|| {
                            anyhow::anyhow!("task {task_id:?} not found in first phase")
                        })?;
                    if !agents::has_hat(&task.role) {
                        return Err(anyhow::anyhow!(
                            "task {} has role {:?} with no implemented hat",
                            task.id,
                            task.role
                        ));
                    }
                    tracing::info!(task = %task.id, role = %task.role, "dispatching hat");
                    let artifact = agents::run_task(task, &ctx).await?;
                    let artifact_id = artifact
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("artifact")
                        .to_string();
                    let file = orchestrator::write_artifact(&out_dir, &artifact)?;
                    println!("wrote {}", file.display());
                    tracing::info!(
                        artifact = %file.display(),
                        "hat produced a schema-validated artifact"
                    );
                    if gh.is_none() {
                        if let Some((name, md)) = agents::render_companion(&artifact) {
                            let _ = std::fs::write(out_dir.join(&name), md);
                            println!("wrote companion {}", out_dir.join(name).display());
                        }
                    }
                    if let Some((g, b)) = pr_ctx {
                        let kind = pr::kind_for_role(&task.role);
                        let inp = pr::PrInput {
                            task,
                            artifact_path: file
                                .to_str()
                                .with_context(|| format!("non-utf8 path {}", file.display()))?,
                            artifact_id: &artifact_id,
                            artifact_kind: kind,
                            base_branch: b,
                        };
                        let outcome = pr::publish_as_pr(g, &inp).await?;
                        println!(
                            "pr       #{} {} ({})",
                            outcome.number, outcome.url, outcome.branch
                        );
                    }
                }
                None => {
                    tracing::info!("orchestrating first phase (DAG-aware)");
                    let report = orchestrator::run_phase(&company_plan, &ctx, pr_ctx).await?;
                    println!(
                        "\nphase run: {} done, {} skipped, {} failed, {} prs",
                        report.done.len(),
                        report.skipped.len(),
                        report.failed.len(),
                        report.prs.len()
                    );
                    if !report.failed.is_empty() {
                        return Err(anyhow::anyhow!("{} task(s) failed", report.failed.len()));
                    }
                }
            }
            Ok(())
        }
        Command::Check { plan } => {
            let schemas_dir = resolve(&cli.repo, std::path::Path::new("platform-spec/schemas"));
            let registry = schema::SchemaRegistry::load_dir(&schemas_dir)?;
            let plan_path = resolve(&cli.repo, &plan);
            let text = std::fs::read_to_string(&plan_path)
                .map_err(|e| anyhow::anyhow!("reading plan {}: {e}", plan_path.display()))?;
            let plan_value: serde_json::Value = serde_json::from_str(&text)
                .with_context(|| format!("parse {}", plan_path.display()))?;
            let report = agents::qa::check_plan(&registry, &plan_value)?;
            println!(
                "{}: {}/{} assertions passed",
                plan_path.display(),
                report.passed,
                report.total
            );
            for f in &report.failed {
                println!(
                    "  FAIL  {} (expected {}, was {})",
                    f.name, f.expected, f.actual
                );
                if let Some(first) = f.errors.lines().next() {
                    if !first.is_empty() {
                        println!("        {first}");
                    }
                }
            }
            if report.all_passed() {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "{} assertion(s) failed",
                    report.failed.len()
                ))
            }
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
