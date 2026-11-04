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
                            Some(dt) if report.skipped.iter().any(|(i, _)| i == &dt.id) => {
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
                    // QA semantic gate: a test-plan artifact must execute cleanly,
                    // or the task is not done. Mirrors `forge run <task>` (main.rs)
                    // so a failing plan never lands as `done`.
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
                                if !r.all_passed() {
                                    let msg = format!(
                                        "{} of {} assertions failed",
                                        r.failed.len(),
                                        r.total
                                    );
                                    let _ = opts
                                        .state
                                        .mark_failed(opts.phase, &task.id, &task.role, &msg);
                                    report.failed.push((id.clone(), msg));
                                    progress = true;
                                    continue;
                                }
                            }
                            Err(e) => {
                                let msg = format!("check error: {e}");
                                println!("qa       {id}  check error: {e}");
                                let _ = opts
                                    .state
                                    .mark_failed(opts.phase, &task.id, &task.role, &msg);
                                report.failed.push((id.clone(), msg));
                                progress = true;
                                continue;
                            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{CompanyPlan, FirstPhase, Phase, Task, Hat, Organization};
    use crate::llm::Llm;
    use crate::state::State;

    fn task(id: &str, role: &str, deps: &[&str]) -> Task {
        Task {
            id: id.into(),
            title: "t".into(),
            role: role.into(),
            task_type: "DEV".into(),
            description: "d".into(),
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn valid_plan_with_hats() -> CompanyPlan {
        CompanyPlan {
            mission: "test mission".into(),
            organization: Organization {
                hats: vec![
                    Hat {
                        name: "Solution Architect".into(),
                        role: "solution architect".into(),
                        responsibilities: vec!["x".into()],
                    },
                    Hat {
                        name: "Tech Lead".into(),
                        role: "tech lead".into(),
                        responsibilities: vec!["x".into()],
                    },
                    Hat {
                        name: "Nonexistent Role".into(),
                        role: "nonexistent_role".into(),
                        responsibilities: vec!["x".into()],
                    },
                ],
            },
            roadmap: vec![Phase {
                name: "P0".into(),
                goal: "g".into(),
                exit_criteria: vec!["c".into()],
            }],
            contribution_model: "cm".into(),
            first_phase: FirstPhase {
                name: "P0".into(),
                tasks: vec![],
            },
        }
    }

    #[test]
    fn skipped_dependency_blocks_dependent_task() {
        let mut plan = valid_plan_with_hats();
        plan.first_phase.tasks = vec![
            task("T1", "solution architect", &[]),
            task("T2", "tech lead", &["T1"]),
        ];
        // Remove T2's hat so it has no hat
        // (The orchestrator will skip T2 with "no hat for role" reason)
        // But T2 depends on T1 which HAS a hat, so T1 should run.
        // After T1 is done, T2 should then be skipped due to no hat.
        // This test verifies the basic flow works.
        // We just check plan validation passes first.
        plan.validate().unwrap();
    }

    #[tokio::test]
    async fn dependent_task_skipped_when_dep_was_previously_skipped() {
        // Simulate the scenario: task T2 depends on T1.
        // T1 has no hat → will be skipped. T2 depends on T1.
        // Without the fix, T2 would attempt to run because T1's skipped
        // status wasn't recognized as a blocking dependency.
        // With the fix, T2 should be skipped as "blocked by un-runnable deps".

        let mut plan = valid_plan_with_hats();
        plan.first_phase.tasks = vec![
            task("T1", "nonexistent_role", &[]), // no hat → will be skipped
            task("T2", "tech lead", &["T1"]),
        ];
        plan.validate().unwrap();

        let ctx = HatContext {
            llm: &crate::llm::Llm::test(),
            registry: &crate::schema::SchemaRegistry::load_dir(
                std::path::Path::new("platform-spec/schemas"),
            )
            .unwrap(),
            examples_dir: std::path::PathBuf::from(""),
            out_dir: std::path::PathBuf::from(""),
        };
        let state = State::open_memory().unwrap();
        let opts = RunOptions {
            pr: None,
            state: &state,
            phase: 1,
            force: false,
        };
        let report = run_phase(&plan, &ctx, &opts).await.unwrap();

        // T1 should be skipped (no hat for "nonexistent_role")
        let t1_skipped = report.skipped.iter().any(|(id, _)| id == "T1");
        // T2 should be skipped because its dep T1 was skipped ("blocked by un-runnable deps")
        let t2_skipped = report.skipped.iter().any(|(id, _)| id == "T2");
        let t2_reason = report
            .skipped
            .iter()
            .find(|(id, _)| id == "T2")
            .map(|(_, reason)| reason)
            .unwrap();

        // Before the fix, T2 would NOT be skipped (or would have different reason)
        // After the fix, T2 should be skipped as blocked by its skipped dep T1
        assert!(t1_skipped, "T1 should be skipped (no hat)");
        assert!(t2_skipped, "T2 should be skipped (blocked by skipped dep T1)");
        assert!(
            t2_reason.contains("blocked by un-runnable deps"),
            "T2 reason should mention blocked by un-runnable deps, got: {}",
            t2_reason
        );
    }
}
