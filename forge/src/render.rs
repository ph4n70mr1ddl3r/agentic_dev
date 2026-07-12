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

fn organization_md(p: &CompanyPlan) -> String {
    let mut s = String::new();
    s.push_str("# Organization\n\n");
    s.push_str(BYLINE);
    s.push_str(&format!("\n**Mission:** {}\n\n## Hats\n\n", p.mission.trim()));
    s.push_str("| Hat | Role | Responsibilities |\n|---|---|---|\n");
    for h in &p.organization.hats {
        s.push_str(&format!(
            "| **{}** | {} | {} |\n",
            h.name,
            h.role,
            h.responsibilities.join("; ")
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
            ph.name,
            ph.goal.trim(),
            ph.exit_criteria.join("; ")
        ));
    }
    s
}

fn contribution_md(p: &CompanyPlan) -> String {
    format!("# Contribution Model\n\n{}\n", p.contribution_model.trim())
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
            t.id, t.title, t.role, t.task_type, deps
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
