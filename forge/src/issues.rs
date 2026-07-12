//! Turn the CEO-authored `plan.json` into GitHub Issues (the `sync` command).
//!
//! Pure transformation logic (slugify, titles, bodies, labels) is unit-tested;
//! the live GitHub calls live in [`crate::github`] and are exercised by the
//! `sync --write` command.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use anyhow::Result;

use crate::github::{CreateIssue, GitHub, Repo};
use crate::plan::{CompanyPlan, Task};

/// HTML comment marker embedded in every synced issue body, so re-runs are
/// idempotent: present => the task already has an open issue.
pub fn task_marker(id: &str) -> String {
    format!("<!-- forge:task:{id} -->")
}

const MARKER_PREFIX: &str = "<!-- forge:task:";

/// Extract the task id from a `<!-- forge:task:T1 -->` marker in a body.
pub fn extract_task_id(body: &str) -> Option<String> {
    let start = body.find(MARKER_PREFIX)?;
    let rest = &body[start + MARKER_PREFIX.len()..];
    let end = rest.find(" -->")?;
    Some(rest[..end].trim().to_string())
}

/// Lowercase, hyphen-separated slug for use in labels/URLs.
pub fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_dash = true; // suppresses a leading dash
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

pub fn build_title(task: &Task) -> String {
    format!("[{}] {}", task.id, task.title.trim())
}

pub fn build_body(task: &Task, phase_name: &str) -> String {
    let deps_str = if task.depends_on.is_empty() {
        "—".to_string()
    } else {
        task.depends_on.join(", ")
    };
    format!(
        "_Authored by the CEO hat via `forge`; synced from `plan.json`._\n\n\
         - **Role:** {role}\n\
         - **Type:** {ty}\n\
         - **Phase:** {phase}\n\
         - **Depends on:** {deps}\n\n\
         {desc}\n\n\
         {marker}\n",
        role = task.role,
        ty = task.task_type,
        phase = phase_name.trim(),
        deps = deps_str,
        desc = task.description.trim(),
        marker = task_marker(&task.id),
    )
}

/// Labels to attach: phase, the `forge` marker, the task type, and the role.
pub fn task_labels(task: &Task, phase_label: &str) -> Vec<String> {
    let mut labels = vec![
        phase_label.to_string(),
        "forge".to_string(),
        format!("type-{}", slugify(&task.task_type)),
        format!("role-{}", slugify(&task.role)),
    ];
    labels.sort();
    labels.dedup();
    labels
}

/// Hex color (no `#`) for a label name, for visual grouping in the UI.
pub fn label_color(name: &str) -> &'static str {
    if name.starts_with("phase-") {
        "6366f1" // indigo
    } else if name == "forge" {
        "0e8a8a" // teal
    } else if name.starts_with("role-") {
        "57606a" // gray
    } else if name.starts_with("type-") {
        match name {
            "type-spec" => "0969da",     // blue
            "type-code" => "1f883d",     // green
            "type-doc" => "bf8700",      // gold
            "type-test" => "d1242f",     // red
            "type-metadata" => "8250df", // purple
            _ => "57606a",
        }
    } else {
        "57606a"
    }
}

pub struct PlannedIssue<'a> {
    pub task: &'a Task,
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
}

/// Build the list of issues to create from the plan's first phase.
pub fn plan_issues<'a>(plan: &'a CompanyPlan, phase_index: usize) -> Vec<PlannedIssue<'a>> {
    let phase_label = format!("phase-{}", phase_index);
    let phase_name = &plan.first_phase.name;
    plan.first_phase
        .tasks
        .iter()
        .map(|t| PlannedIssue {
            task: t,
            title: build_title(t),
            body: build_body(t, phase_name),
            labels: task_labels(t, &phase_label),
        })
        .collect()
}

/// Deduplicated union of all labels across the planned issues.
pub fn all_labels(planned: &[PlannedIssue<'_>]) -> Vec<String> {
    let mut set: Vec<String> = Vec::new();
    for p in planned {
        for l in &p.labels {
            if !set.contains(l) {
                set.push(l.clone());
            }
        }
    }
    set
}

/// Dry run: print what would be created. No network, no token needed.
pub fn run_sync_dry(plan: &CompanyPlan, repo: &Repo, phase_index: usize) -> Result<()> {
    let planned = plan_issues(plan, phase_index);
    let labels = all_labels(&planned);
    println!("GitHub repo : {}", repo.slug());
    println!("Milestone   : {}", plan.first_phase.name.trim());
    println!("Labels      : {}", labels.join(", "));
    println!();
    println!(
        "Would create {} issues (dry-run; pass --write to create):",
        planned.len()
    );
    for p in &planned {
        let deps = if p.task.depends_on.is_empty() {
            "—".to_string()
        } else {
            p.task.depends_on.join(",")
        };
        let ty = slugify(&p.task.task_type);
        println!(
            "  {}  [{:<7}] {}  (deps: {})",
            p.task.id, ty, p.task.title, deps
        );
    }
    Ok(())
}

/// Create the issues for real. Idempotent: skips tasks whose marker already
/// appears in an open issue's body.
pub async fn run_sync(plan: &CompanyPlan, gh: &GitHub, phase_index: usize) -> Result<()> {
    let planned = plan_issues(plan, phase_index);
    let labels = all_labels(&planned);

    tracing::info!(repo = %gh.repo_slug(), labels = labels.len(), "ensuring labels");
    let mut label_ok: HashSet<String> = HashSet::new();
    for l in &labels {
        match gh.ensure_label(l, label_color(l)).await {
            Ok(()) => {
                label_ok.insert(l.clone());
            }
            Err(e) => tracing::warn!(label = %l, error = %e, "could not ensure label; will omit"),
        }
    }

    let milestone = match gh.ensure_milestone(&plan.first_phase.name).await {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(error = %e, "could not ensure milestone; issues will have none");
            None
        }
    };

    let issues = gh.list_open_issues().await?;
    let mut existing: HashMap<String, (u64, String)> = HashMap::new();
    for i in &issues {
        if let Some(b) = &i.body {
            if let Some(id) = extract_task_id(b) {
                existing.insert(id, (i.number, i.title.clone()));
            }
        }
    }
    tracing::info!(
        open_issues = issues.len(),
        known_tasks = existing.len(),
        "scanned"
    );

    for p in &planned {
        if let Some((num, etitle)) = existing.get(&p.task.id) {
            println!("skip     #{num:<6} {etitle}  (already open)");
            continue;
        }
        let labels_ok: Vec<String> = p
            .labels
            .iter()
            .filter(|l| label_ok.contains(*l))
            .cloned()
            .collect();
        let req = CreateIssue {
            title: &p.title,
            body: &p.body,
            labels: labels_ok.clone(),
            milestone,
        };
        match gh.create_issue(&req).await {
            Ok(num) => {
                println!(
                    "created  #{num:<6} {} {} [{}]",
                    p.task.id,
                    p.task.title,
                    labels_ok.join(",")
                );
                tracing::info!(issue = num, task = %p.task.id, "created issue");
            }
            Err(e) => {
                tracing::error!(task = %p.task.id, error = %e, "failed to create issue");
                println!("FAILED   {}  {}  ({})", p.task.id, p.task.title, e);
            }
        }
        // Dodge GitHub's secondary rate limit when bursting several creates.
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{FirstPhase, Hat, Organization, Phase};

    fn task(id: &str, role: &str, ty: &str, deps: &[&str]) -> Task {
        Task {
            id: id.into(),
            title: format!("title {id}"),
            role: role.into(),
            task_type: ty.into(),
            description: format!("desc {id}"),
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn plan_with(tasks: Vec<Task>) -> CompanyPlan {
        CompanyPlan {
            mission: "m".into(),
            organization: Organization {
                hats: vec![Hat {
                    name: "CEO".into(),
                    role: "r".into(),
                    responsibilities: vec!["x".into()],
                }],
            },
            roadmap: vec![Phase {
                name: "P1".into(),
                goal: "g".into(),
                exit_criteria: vec!["c".into()],
            }],
            contribution_model: "cm".into(),
            first_phase: FirstPhase {
                name: "P1".into(),
                tasks,
            },
        }
    }

    #[test]
    fn slugify_cases() {
        assert_eq!(
            slugify("Domain Modeler (Financials)"),
            "domain-modeler-financials"
        );
        assert_eq!(slugify("  --CEO--  "), "ceo");
        assert_eq!(slugify("Tech Lead"), "tech-lead");
        assert_eq!(slugify(""), "");
    }

    #[test]
    fn marker_roundtrip_and_extract() {
        assert_eq!(task_marker("T1"), "<!-- forge:task:T1 -->");
        let body = format!("text\n{}\nmore", task_marker("T7"));
        assert_eq!(extract_task_id(&body), Some("T7".to_string()));
        assert_eq!(extract_task_id("no marker"), None);
    }

    #[test]
    fn title_and_body_shape() {
        let p = plan_with(vec![task("T1", "Solution Architect", "spec", &[])]);
        let pi = &plan_issues(&p, 1)[0];
        assert_eq!(pi.title, "[T1] title T1");
        assert!(pi.body.contains("<!-- forge:task:T1 -->"));
        assert!(pi.body.contains("- **Role:** Solution Architect"));
        assert!(pi.body.contains("- **Phase:** P1"));
        assert!(pi.body.contains("- **Depends on:** —"));
    }

    #[test]
    fn labels_cover_all_groups() {
        let p = plan_with(vec![task("T1", "QA Engineer", "test", &["T0"])]);
        let pi = &plan_issues(&p, 3)[0];
        assert!(pi.labels.contains(&"phase-3".to_string()));
        assert!(pi.labels.contains(&"forge".to_string()));
        assert!(pi.labels.contains(&"type-test".to_string()));
        assert!(pi.labels.contains(&"role-qa-engineer".to_string()));
    }

    #[test]
    fn all_labels_dedupes_shared_labels() {
        let p = plan_with(vec![
            task("T1", "CEO", "spec", &[]),
            task("T2", "CEO", "code", &[]),
        ]);
        let planned = plan_issues(&p, 1);
        let labels = all_labels(&planned);
        assert_eq!(labels.iter().filter(|l| *l == "phase-1").count(), 1);
        assert_eq!(labels.iter().filter(|l| *l == "forge").count(), 1);
        assert!(labels.contains(&"type-spec".to_string()));
        assert!(labels.contains(&"type-code".to_string()));
    }
}
