//! PR write-back: turn a produced artifact into a reviewable GitHub PR.
//!
//! Flow per artifact: from the base branch, create `forge/<task>-<artifact>`,
//! commit just the artifact file, push, open a PR (title + body carrying the
//! task context and the schema it was validated against), then return to the
//! base branch so the next task starts clean. On any failure we still try to
//! restore the base branch (so a crash doesn't strand the user on a forge
//! branch). This closes the ADR-0005 loop: branches = work, PRs = review.

use anyhow::{anyhow, Result};

use crate::git;
use crate::github::GitHub;
use crate::issues::slugify;
use crate::plan::Task;

pub struct PrInput<'a> {
    pub task: &'a Task,
    /// Repo-relative path of the artifact file (as `git add` expects it).
    pub artifact_path: &'a str,
    pub artifact_id: &'a str,
    pub artifact_kind: &'a str, // "entity" | "workflow" | ...
    pub base_branch: &'a str,
}

#[derive(Debug)]
pub struct PrOutcome {
    pub number: u64,
    pub url: String,
    pub branch: String,
}

/// Map a task role to its artifact kind label.
pub fn kind_for_role(role: &str) -> &'static str {
    let r = role.trim().to_ascii_lowercase();
    if r.starts_with("domain modeler") {
        return "reference";
    }
    match r.as_str() {
        "solution architect" | "architect" => "entity",
        "tech lead" | "tech-lead" => "workflow",
        _ => "artifact",
    }
}

pub fn branch_name(task_id: &str, artifact_id: &str) -> String {
    format!("forge/{}-{}", slugify(task_id), slugify(artifact_id))
}

pub fn pr_title(task: &Task, kind: &str, artifact_id: &str) -> String {
    format!("forge: {} — {} {}", task.id, kind, artifact_id)
}

fn schema_for_kind(kind: &str) -> &'static str {
    match kind {
        "entity" => "entity.schema.json",
        "workflow" => "workflow.schema.json",
        "reference" => "domain-reference.schema.json",
        _ => "platform-spec",
    }
}

pub fn pr_body(task: &Task, kind: &str, artifact_id: &str) -> String {
    let deps = if task.depends_on.is_empty() {
        "—".to_string()
    } else {
        task.depends_on.join(", ")
    };
    format!(
        "_Authored by the **{role}** hat via `forge`; schema-validated._\n\n\
         - **Task:** {id} — {title}\n\
         - **Artifact:** {kind} `{aid}` (validated against `{schema}`)\n\
         - **Type:** {ty}\n\
         - **Depends on:** {deps}\n\n\
         Machine-generated and mechanically reviewed against the platform-spec schema. \
         Human / domain review of the content is the remaining gate (ADR-0005).",
        role = task.role,
        id = task.id,
        title = task.title.trim(),
        kind = kind,
        aid = artifact_id,
        schema = schema_for_kind(kind),
        ty = task.task_type,
        deps = deps,
    )
}

fn commit_message(task: &Task, kind: &str, artifact_id: &str) -> String {
    format!(
        "forge: {id} — {kind} {aid}\n\nAuthored by the {role} hat via forge; schema-validated.",
        id = task.id,
        kind = kind,
        aid = artifact_id,
        role = task.role,
    )
}

/// Publish `inp`'s artifact as a PR. The caller must have already written the
/// artifact to disk and be on `base_branch` (with the artifact as the only
/// pending change).
pub async fn publish_as_pr(gh: &GitHub, inp: &PrInput<'_>) -> Result<PrOutcome> {
    let branch = branch_name(&inp.task.id, inp.artifact_id);
    // Always try to return to the base branch, even on failure.
    let restore = || {
        let _ = git::checkout(inp.base_branch);
    };

    if git::branch_exists(&branch)? {
        // Stale local branch from a prior attempt — drop and recreate from base.
        git::delete_local_branch(&branch)?;
    }

    if let Err(e) = git::create_branch_from(inp.base_branch, &branch) {
        restore();
        return Err(e);
    }
    if let Err(e) = git::add(inp.artifact_path) {
        restore();
        return Err(e);
    }
    let committed = match git::commit(&commit_message(
        inp.task,
        inp.artifact_kind,
        inp.artifact_id,
    )) {
        Ok(c) => c,
        Err(e) => {
            restore();
            return Err(e);
        }
    };
    if !committed {
        restore();
        return Err(anyhow!(
            "no changes to commit for {} (artifact identical to base?)",
            inp.task.id
        ));
    }
    if let Err(e) = git::push(&branch, "origin") {
        restore();
        return Err(e);
    }

    let title = pr_title(inp.task, inp.artifact_kind, inp.artifact_id);
    let body = pr_body(inp.task, inp.artifact_kind, inp.artifact_id);
    let pr = match gh
        .create_pull_request(&title, &body, &branch, inp.base_branch)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            restore();
            return Err(e);
        }
    };

    let outcome = PrOutcome {
        number: pr.number,
        url: pr.url,
        branch,
    };
    restore();
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, role: &str) -> Task {
        Task {
            id: id.into(),
            title: "Do the thing".into(),
            role: role.into(),
            task_type: "spec".into(),
            description: "d".into(),
            depends_on: vec!["T0".into()],
        }
    }

    #[test]
    fn branch_and_title_naming() {
        assert_eq!(
            branch_name("T3", "company-lifecycle"),
            "forge/t3-company-lifecycle"
        );
        let t = task("T3", "Solution Architect");
        assert_eq!(
            pr_title(&t, "entity", "company"),
            "forge: T3 — entity company"
        );
    }

    #[test]
    fn body_carries_context_and_schema() {
        let t = task("T4", "Tech Lead");
        let body = pr_body(&t, "workflow", "company-lifecycle");
        assert!(body.contains("**Tech Lead**"));
        assert!(body.contains("Task:** T4"));
        assert!(body.contains("workflow `company-lifecycle`"));
        assert!(body.contains("workflow.schema.json"));
        assert!(body.contains("Depends on:** T0"));
    }

    #[test]
    fn kind_for_role_maps() {
        assert_eq!(kind_for_role("Solution Architect"), "entity");
        assert_eq!(kind_for_role("Tech Lead"), "workflow");
        assert_eq!(kind_for_role("Domain Modeler (Financials)"), "reference");
        assert_eq!(kind_for_role("Domain Modeler (Supply Chain)"), "reference");
        assert_eq!(kind_for_role("QA Engineer"), "artifact");
    }
}
