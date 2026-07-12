use std::path::Path;

use anyhow::Result;

use crate::plan::CompanyPlan;

/// Render the CEO's plan into markdown files + a raw plan.json under `dir`.
pub fn render(plan: &CompanyPlan, dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join("organization.md"), organization_md(plan))?;
    std::fs::write(dir.join("roadmap.md"), roadmap_md(plan))?;
    std::fs::write(dir.join("contribution-model.md"), contribution_md(plan))?;
    std::fs::write(dir.join("first-phase-tasks.md"), tasks_md(plan))?;
    std::fs::write(dir.join("plan.json"), serde_json::to_string_pretty(plan)?)?;
    Ok(())
}

const BYLINE: &str = "_Authored by the CEO hat via `forge`._\n";

/// Escape a string for safe use inside a GitHub-flavored markdown table cell:
/// a raw pipe would end the cell and a newline would break the table row.
fn md_cell(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', "<br>")
}

fn organization_md(p: &CompanyPlan) -> String {
    let mut s = String::new();
    s.push_str("# Organization\n\n");
    s.push_str(BYLINE);
    s.push_str(&format!("\n**Mission:** {}\n\n## Hats\n\n", p.mission.trim()));
    s.push_str("| Hat | Role | Responsibilities |\n|---|---|---|\n");
    for h in &p.organization.hats {
        s.push_str(&format!(
            "| **{}** | {} | {} |\n",
            md_cell(&h.name),
            md_cell(&h.role),
            md_cell(&h.responsibilities.join("; ")),
        ));
    }
    s
}

fn roadmap_md(p: &CompanyPlan) -> String {
    let mut s = String::new();
    s.push_str("# Roadmap\n\n");
    s.push_str(BYLINE);
    s.push_str("\n| # | Phase | Goal | Exit criteria |\n|---|---|---|---|\n");
    for (i, ph) in p.roadmap.iter().enumerate() {
        s.push_str(&format!(
            "| {} | **{}** | {} | {} |\n",
            i,
            md_cell(&ph.name),
            md_cell(ph.goal.trim()),
            md_cell(&ph.exit_criteria.join("; ")),
        ));
    }
    s
}

fn contribution_md(p: &CompanyPlan) -> String {
    format!(
        "# Contribution Model\n\n{}\n{}\n",
        BYLINE,
        p.contribution_model.trim()
    )
}

fn tasks_md(p: &CompanyPlan) -> String {
    let mut s = String::new();
    s.push_str(&format!("# First Phase: {}\n\n", p.first_phase.name.trim()));
    s.push_str(BYLINE);
    s.push_str("\n| ID | Title | Role | Type | Depends on |\n|---|---|---|---|---|\n");
    for t in &p.first_phase.tasks {
        let deps = if t.depends_on.is_empty() {
            "—".to_string()
        } else {
            t.depends_on.join(", ")
        };
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            md_cell(&t.id),
            md_cell(&t.title),
            md_cell(&t.role),
            md_cell(&t.task_type),
            md_cell(&deps),
        ));
    }
    s.push_str("\n## Details\n\n");
    for t in &p.first_phase.tasks {
        s.push_str(&format!("### {} — {}\n", t.id, t.title));
        s.push_str(&format!("- **Role:** {}\n- **Type:** {}\n", t.role, t.task_type));
        if !t.depends_on.is_empty() {
            s.push_str(&format!("- **Depends on:** {}\n", t.depends_on.join(", ")));
        }
        s.push_str(&format!("\n{}\n\n", t.description.trim()));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{CompanyPlan, FirstPhase, Hat, Organization, Phase};

    #[test]
    fn md_cell_escapes_pipe_and_newline() {
        assert_eq!(md_cell("a|b"), "a\\|b");
        assert_eq!(md_cell("a\nb"), "a<br>b");
        assert_eq!(md_cell("plain"), "plain");
    }

    fn plan_with_responsibility(resp: &str) -> CompanyPlan {
        CompanyPlan {
            mission: "m".into(),
            organization: Organization {
                hats: vec![Hat {
                    name: "CEO".into(),
                    role: "chief".into(),
                    responsibilities: vec![resp.into()],
                }],
            },
            roadmap: vec![Phase {
                name: "P".into(),
                goal: "g".into(),
                exit_criteria: vec!["c".into()],
            }],
            contribution_model: "cm".into(),
            first_phase: FirstPhase { name: "P".into(), tasks: vec![] },
        }
    }

    #[test]
    fn organization_table_escapes_pipe() {
        let md = organization_md(&plan_with_responsibility("do | thing"));
        assert!(md.contains("do \\| thing"), "pipe should be escaped: {md}");
        assert!(!md.contains("do | thing"), "raw pipe leaked into table: {md}");
    }

    #[test]
    fn organization_table_escapes_newline() {
        let md = organization_md(&plan_with_responsibility("line1\nline2"));
        assert!(md.contains("line1<br>line2"), "newline should be escaped: {md}");
    }
}
