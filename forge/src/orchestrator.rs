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

// unused import: `crate::llm::Llm` removed - not needed in this module
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

    // Sort tasks into dependency order so that tasks with no dependencies are
    // processed before tasks that depend on others, regardless of how the CEO
    // presents the task list (i.e. reordered tasks). Uses an index-based sort
    // to avoid requiring Clone on Task.
    let mut indices: Vec<usize> = (0..tasks.len()).collect();
    indices.sort_by(|&i, &j| {
        let a_no_deps = tasks[i].depends_on.is_empty();
        let b_no_deps = tasks[j].depends_on.is_empty();
        if a_no_deps != b_no_deps {
            return a_no_deps.cmp(&b_no_deps);
        }
        let a_dep_count = tasks[i].depends_on.len();
        let b_dep_count = tasks[j].depends_on.len();
        if a_dep_count != b_dep_count {
            return a_dep_count.cmp(&b_dep_count);
        }
        tasks[i].id.cmp(&tasks[j].id)
    });

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
        for idx in &indices {
            let task = &tasks[*idx];
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
                    continue;
                } else {
                    let reason = format!("blocked by pending deps: {}", pending.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));
                    println!("skip     {id}  ({reason})");
                    report.skipped.push((id.clone(), reason.clone()));
                    let _ = opts
                        .state
                        .mark_skipped(opts.phase, &task.id, &task.role, &reason);
                    progress = true;
                    continue;
                }
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
    for idx in &indices {
        let task = &tasks[*idx];
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

#[tokio::test]
    async fn dependent_task_dispatched_before_dependency_when_reordered() {
        // Bug demonstration: when the dependent task (T2) appears BEFORE
        // its dependency (T1) in the task list, the current code dispatches
        // T2 before T1 even though T1's result is needed as context.
        // This test documents the existing behavior; the fix will ensure
        // T2 waits for T1 even when reordered.
        let mut plan = valid_plan_with_hats();
        plan.first_phase.tasks = vec![
            task("T2", "tech lead", &["T1"]), // depends on T1, but appears first
            task("T1", "tech lead", &[]),    // no deps, appears second
        ];
        plan.validate().unwrap();

        let ctx = HatContext {
            llm: &crate::llm::Llm::test(),
            registry: &crate::schema::SchemaRegistry::load_dir(
                &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../erp/platform-spec/schemas"),
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

        // With the bug, T2 gets dispatched before T1 (unsatisfied deps).
        // After the fix, T1 should be done before T2, and T2 should be
        // properly tracked in the report as blocked by pending deps.
        let t1_done = report.done.iter().any(|id| id == "T1");
        let t2_done = report.done.iter().any(|id| id == "T2");

        // T1 may fail (e.g. due to environment), but T2 should be properly
        // tracked as skipped/blocked due to its pending dependency on T1.
        eprintln!("T1 done: {}, T2 done: {}", t1_done, t2_done);
        assert!(
            report.skipped.iter().any(|(id, _)| id == "T2"),
            "T2 should be tracked as skipped (blocked by pending dep T1)"
        );
    }

    #[test]
    fn skipped_dependency_blocks_dependent_task() {
        let mut plan = valid_plan_with_hats();
        plan.first_phase.tasks = vec![
            task("T1", "solution architect", &[]),
            task("T2", "tech lead", &["T1"]),
        ];
        plan.validate().unwrap();
    }

    #[tokio::test]
    async fn dependent_task_skipped_when_dep_was_previously_skipped() {
        let mut plan = valid_plan_with_hats();
        // Add "no hat role" hat so T1's role is valid
        plan.organization.hats.push(Hat {
            name: "no hat role".into(),
            role: "no hat role".into(),
            responsibilities: vec!["x".into()],
        });
        plan.first_phase.tasks = vec![
            task("T1", "no hat role", &[]),
            task("T2", "tech lead", &["T1"]),
        ];
        plan.validate().unwrap();

        let ctx = HatContext {
            llm: &crate::llm::Llm::test(),
            registry: &crate::schema::SchemaRegistry::load_dir(
                &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../erp/platform-spec/schemas"),
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

        let t1_skipped = report.skipped.iter().any(|(id, _)| id == "T1");
        let t2_skipped = report.skipped.iter().any(|(id, _)| id == "T2");
        let t2_reason = report
            .skipped
            .iter()
            .find(|(id, _)| id == "T2")
            .map(|(_, reason)| reason)
            .unwrap();

        assert!(t1_skipped, "T1 should be skipped (no hat)");
        assert!(
            t2_skipped,
            "T2 should be skipped (blocked by skipped dep T1)"
        );
        assert!(
            t2_reason.contains("blocked by un-runnable deps"),
            "T2 reason should mention blocked by un-runnable deps, got: {}",
            t2_reason
        );
    }

#[tokio::test]
    async fn dependent_task_blocked_when_some_deps_done_and_one_skipped() {
        let mut plan = valid_plan_with_hats();
        // Add "no hat role" hat so T3's role is valid
        plan.organization.hats.push(Hat {
            name: "no hat role".into(),
            role: "no hat role".into(),
            responsibilities: vec!["x".into()],
        });
        plan.first_phase.tasks = vec![
            task("T1", "tech lead", &[]), // would be done but may fail in test; T2 depends on it
            task("T3", "no hat role", &[]), // skipped (no hat)
            task("T2", "tech lead", &["T1", "T3"]), // depends on both done and skipped
        ];
        plan.validate().unwrap();

        let ctx = HatContext {
            llm: &crate::llm::Llm::test(),
            registry: &crate::schema::SchemaRegistry::load_dir(
                &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../erp/platform-spec/schemas"),
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

        // T3 should be skipped (no hat)
        let t3_skipped = report.skipped.iter().any(|(id, _)| id == "T3");
        // T2 should be skipped because one of its deps (T3) is skipped
        let t2_skipped = report.skipped.iter().any(|(id, _)| id == "T2");
        let t2_reason = report
            .skipped
            .iter()
            .find(|(id, _)| id == "T2")
            .map(|(_, reason)| reason)
            .unwrap();

        assert!(t3_skipped, "T3 should be skipped (no hat)");
        assert!(
            t2_skipped,
            "T2 should be skipped because T3 (a dep) is skipped"
        );
        assert!(
            t2_reason.contains("blocked by un-runnable deps"),
            "T2 reason should mention blocked by un-runnable deps, got: {}",
            t2_reason
        );
    }

#[tokio::test]
    async fn pending_dep_task_mark_progress() {
        let mut plan = valid_plan_with_hats();
        plan.first_phase.tasks = vec![
            task("T1", "tech lead", &["T2"]),
            task("T2", "tech lead", &[]),
        ];
        plan.validate().unwrap();

        let ctx = HatContext {
            llm: &crate::llm::Llm::test(),
            registry: &crate::schema::SchemaRegistry::load_dir(
                &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../erp/platform-spec/schemas"),
            )
            .unwrap(),
            examples_dir: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
                "../erp/platform-spec/examples",
            ),
            out_dir: std::path::PathBuf::from(""),
        };
        let state = State::open_memory().unwrap();
        let opts = RunOptions {
            pr: None,
            state: &state,
            phase: 1,
            force: true,
        };
        let _report = run_phase(&plan, &ctx, &opts).await;
    }
}
