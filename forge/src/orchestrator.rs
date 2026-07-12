//! The phase orchestrator: runs every first-phase task whose owning role has a
//! hat and whose dependencies are satisfied, in dependency order. Tasks with no
//! hat, or blocked by un-runnable dependencies, are skipped with a reason. This
//! is the loop that turns "run one task" into "run the company" (within a phase;
//! cross-phase work waits on a human gate — ADR-0005).
//!
//! State is resumable: tasks recorded as `done` in the [`State`] DB are skipped
//! (not re-run) unless `force` is set, and each outcome is persisted.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::agents::{has_hat, run_task, HatContext};
use crate::github::GitHub;
use crate::issues::slugify;
use crate::plan::CompanyPlan;
use crate::state::State;

/// Write a validated artifact as `<id>.json` under `out_dir`; returns its path.
pub fn write_artifact(out_dir: &Path, value: &Value) -> Result<PathBuf> {
    std::fs::create_dir_all(out_dir).with_context(|| format!("create {}", out_dir.display()))?;
    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("artifact");
    let file = out_dir.join(format!("{id}.json"));
    std::fs::write(&file, format!("{}\n", serde_json::to_string_pretty(value)?))
        .with_context(|| format!("write {}", file.display()))?;
    Ok(file)
}

#[derive(Default)]
pub struct RunReport {
    pub done: Vec<String>,
    pub skipped: Vec<(String, String)>, // (task_id, reason)
    pub failed: Vec<(String, String)>,
    pub prs: Vec<(String, String)>, // (task_id, pr_url)
}

pub struct RunOptions<'a> {
    pub pr: Option<(&'a GitHub, &'a str)>,
    pub state: &'a State,
    pub phase: usize,
    pub force: bool,
}

/// Run the phase to a fixpoint. Seeds satisfied deps from persisted `done` tasks
/// (unless `force`), then repeatedly runs ready tasks in plan order, persisting
/// each outcome. Anything left unsettled is a cycle / dead chain.
pub async fn run_phase(
    plan: &CompanyPlan,
    ctx: &HatContext<'_>,
    opts: &RunOptions<'_>,
) -> Result<RunReport> {
    let tasks = &plan.first_phase.tasks;
    let mut report = RunReport::default();

    // Seed satisfied deps from prior runs (the resumability primitive).
    let mut done: HashSet<String> = if opts.force {
        HashSet::new()
    } else {
        opts.state.done_set(opts.phase)?
    };
    if !opts.force {
        let resumed: Vec<&str> = tasks
            .iter()
            .map(|t| t.id.as_str())
            .filter(|id| done.contains(*id))
            .collect();
        if !resumed.is_empty() {
            println!(
                "resume   {} already done: {}",
                resumed.len(),
                resumed.join(", ")
            );
        }
    }
    let mut produced: HashSet<String> = HashSet::new();

    loop {
        let mut progress = false;
        for task in tasks {
            let id = task.id.trim().to_string();
            let settled = done.contains(&id)
                || produced.contains(&id)
                || report.skipped.iter().any(|(i, _)| i == &id)
                || report.failed.iter().any(|(i, _)| i == &id);
            if settled {
                continue;
            }

            // No hat for this role -> terminal skip.
            if !has_hat(&task.role) {
                let reason = format!("no hat for role {:?}", task.role);
                println!("skip     {id}  ({reason})");
                report.skipped.push((id.clone(), reason.clone()));
                let _ = opts
                    .state
                    .mark_skipped(opts.phase, &task.id, &task.role, &reason);
                progress = true;
                continue;
            }

            // Dependency check (across both this run and prior persisted runs).
            let pending: Vec<&String> = task
                .depends_on
                .iter()
                .filter(|d| {
                    let dt = d.trim();
                    !done.contains(dt) && !produced.contains(dt)
                })
                .collect();
            if !pending.is_empty() {
                let dead: Vec<String> = pending
                    .iter()
                    .filter_map(|d| {
                        let dt = tasks.iter().find(|t| t.id.trim() == d.trim());
                        match dt {
                            None => Some(d.trim().to_string()),
                            Some(dt) if !has_hat(&dt.role) => Some(d.trim().to_string()),
                            Some(dt) if report.failed.iter().any(|(i, _)| i == &dt.id) => {
                                Some(d.trim().to_string())
                            }
                            _ => None,
                        }
                    })
                    .collect();
                if !dead.is_empty() {
                    let reason = format!("blocked by un-runnable deps: {}", dead.join(", "));
                    println!("skip     {id}  ({reason})");
                    report.skipped.push((id.clone(), reason.clone()));
                    let _ = opts
                        .state
                        .mark_skipped(opts.phase, &task.id, &task.role, &reason);
                    progress = true;
                }
                continue;
            }

            // Ready: dispatch.
            println!("run      {id}  [{}] ...", slugify(&task.role));
            match run_task(task, ctx).await {
                Ok(artifact) => {
                    let file = write_artifact(&ctx.out_dir, &artifact)?;
                    let aid = artifact
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?")
                        .to_string();
                    println!("done     {id}  -> {} ({aid})", file.display());
                    if opts.pr.is_none() {
                        if let Some((name, md)) = crate::agents::render_companion(&artifact) {
                            let _ = std::fs::write(ctx.out_dir.join(&name), md);
                            println!("wrote    {id}  companion {name}");
                        }
                    }
                    if artifact.get("$kind").and_then(|v| v.as_str()) == Some("test-plan") {
                        match crate::agents::qa::check_plan(ctx.registry, &artifact) {
                            Ok(r) => {
                                println!(
                                    "qa       {id}  {}/{} assertions passed",
                                    r.passed, r.total
                                );
                                for f in &r.failed {
                                    println!(
                                        "  FAIL    {} (expected {}, was {})",
                                        f.name, f.expected, f.actual
                                    );
                                }
                            }
                            Err(e) => println!("qa       {id}  check error: {e}"),
                        }
                    }
                    if let Some((gh, base)) = opts.pr {
                        let kind = crate::pr::kind_for_role(&task.role);
                        let path = file.to_str().unwrap_or("");
                        let inp = crate::pr::PrInput {
                            task,
                            artifact_path: path,
                            artifact_id: &aid,
                            artifact_kind: kind,
                            base_branch: base,
                        };
                        match crate::pr::publish_as_pr(gh, &inp).await {
                            Ok(o) => {
                                println!("pr       {id}  #{} {} ({})", o.number, o.url, o.branch);
                                report.prs.push((id.clone(), o.url));
                            }
                            Err(e) => {
                                println!("pr-fail  {id}  ({e})");
                            }
                        }
                    }
                    let _ = opts.state.mark_done(
                        opts.phase,
                        &task.id,
                        &task.role,
                        &aid,
                        &file.to_string_lossy(),
                    );
                    produced.insert(id.clone());
                    done.insert(id.clone());
                    report.done.push(id);
                    progress = true;
                }
                Err(e) => {
                    let msg = e.to_string();
                    println!("FAILED   {id}  ({msg})");
                    let _ = opts
                        .state
                        .mark_failed(opts.phase, &task.id, &task.role, &msg);
                    report.failed.push((id, msg));
                    progress = true;
                }
            }
        }
        if !progress {
            break;
        }
    }

    // Anything still unsettled is waiting on deps that never resolved.
    for task in tasks {
        let id = task.id.trim().to_string();
        let settled = done.contains(&id)
            || produced.contains(&id)
            || report.skipped.iter().any(|(i, _)| i == &id)
            || report.failed.iter().any(|(i, _)| i == &id);
        if !settled {
            let reason = "unresolved dependencies".to_string();
            println!("skip     {id}  ({reason})");
            report.skipped.push((id, reason));
        }
    }

    Ok(report)
}
