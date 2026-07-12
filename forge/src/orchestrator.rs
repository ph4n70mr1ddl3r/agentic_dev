//! The phase orchestrator: runs every first-phase task whose owning role has a
//! hat and whose dependencies are satisfied, in dependency order. Tasks with no
//! hat, or blocked by un-runnable dependencies, are skipped with a reason. This
//! is the loop that turns "run one task" into "run the company" (within a phase;
//! cross-phase work waits on a human gate — ADR-0005).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::agents::{has_hat, run_task, HatContext};
use crate::issues::slugify;
use crate::plan::CompanyPlan;

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
}

impl RunReport {
    fn settled(&self, done: &HashSet<String>, id: &str) -> bool {
        done.contains(id)
            || self.skipped.iter().any(|(i, _)| i == id)
            || self.failed.iter().any(|(i, _)| i == id)
    }
}

/// Run the first phase to a fixpoint: repeatedly pick ready tasks (has hat +
/// deps done) in plan order until no more progress. Then mark anything still
/// unsettled as skipped (cycle / dead chain).
pub async fn run_phase(
    plan: &CompanyPlan,
    ctx: &HatContext<'_>,
    out_dir: &Path,
) -> Result<RunReport> {
    let tasks = &plan.first_phase.tasks;
    let mut report = RunReport::default();
    let mut done: HashSet<String> = HashSet::new();

    loop {
        let mut progress = false;
        for task in tasks {
            let id = task.id.trim().to_string();
            if report.settled(&done, &id) {
                continue;
            }

            // No hat for this role -> terminal skip.
            if !has_hat(&task.role) {
                let reason = format!("no hat for role {:?}", task.role);
                println!("skip     {id}  ({reason})");
                report.skipped.push((id, reason));
                progress = true;
                continue;
            }

            // Dependency check.
            let pending: Vec<&String> = task
                .depends_on
                .iter()
                .filter(|d| !done.contains(d.trim() as &str))
                .collect();
            if !pending.is_empty() {
                // Skip permanently if any pending dep is itself un-runnable
                // (unknown / no hat / already failed) — waiting would be forever.
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
                    report.skipped.push((id, reason));
                    progress = true;
                }
                // Otherwise: wait — its deps are runnable but not done yet.
                continue;
            }

            // Ready: dispatch.
            println!("run      {id}  [{}] ...", slugify(&task.role));
            match run_task(task, ctx).await {
                Ok(artifact) => {
                    let file = write_artifact(out_dir, &artifact)?;
                    let aid = artifact.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                    println!("done     {id}  -> {} ({aid})", file.display());
                    done.insert(id.clone());
                    report.done.push(id);
                    progress = true;
                }
                Err(e) => {
                    let msg = e.to_string();
                    println!("FAILED   {id}  ({msg})");
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
        if !report.settled(&done, &id) {
            let reason = "unresolved dependencies".to_string();
            println!("skip     {id}  ({reason})");
            report.skipped.push((id, reason));
        }
    }

    Ok(report)
}
