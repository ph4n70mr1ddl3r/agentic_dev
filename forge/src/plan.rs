use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use std::collections::HashSet;

/// The structured company plan produced by the CEO hat.
#[derive(Debug, Serialize, Deserialize)]
pub struct CompanyPlan {
    pub mission: String,
    pub organization: Organization,
    pub roadmap: Vec<Phase>,
    pub contribution_model: String,
    pub first_phase: FirstPhase,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Organization {
    pub hats: Vec<Hat>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Hat {
    pub name: String,
    pub role: String,
    pub responsibilities: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Phase {
    pub name: String,
    pub goal: String,
    pub exit_criteria: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FirstPhase {
    pub name: String,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub role: String,
    #[serde(rename = "type")]
    pub task_type: String,
    pub description: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

impl CompanyPlan {
    /// Validate the structural integrity of a CEO-produced plan.
    ///
    /// This is the harness's first line of defense against a weak model: the
    /// JSON parsed, but we still assert it is a *well-formed* plan before we
    /// render or act on it. See ADR-0005 (structured I/O + mechanical review).
    pub fn validate(&self) -> Result<()> {
        let mut errs: Vec<String> = Vec::new();

        if self.mission.trim().is_empty() {
            errs.push("mission is empty".into());
        }

        if self.organization.hats.is_empty() {
            errs.push("organization has no hats".into());
        }
        let mut hat_names = HashSet::new();
        for h in &self.organization.hats {
            if h.name.trim().is_empty() {
                errs.push(format!("a hat has an empty name (role: {})", h.role));
            } else if !hat_names.insert(h.name.trim().to_ascii_lowercase()) {
                errs.push(format!("duplicate hat name: {}", h.name));
            }
        }

        if self.roadmap.is_empty() {
            errs.push("roadmap has no phases".into());
        }
        for (i, ph) in self.roadmap.iter().enumerate() {
            let label = ph.name.trim();
            if label.is_empty() {
                errs.push(format!("phase #{} has no name", i));
            }
            if ph.goal.trim().is_empty() {
                errs.push(format!("phase #{} ({}) has no goal", i, label));
            }
            if ph.exit_criteria.is_empty() {
                errs.push(format!("phase #{} ({}) has no exit criteria", i, label));
            }
        }

        if self.first_phase.name.trim().is_empty() {
            errs.push("first_phase has no name".into());
        }
        let mut task_ids: HashSet<String> = HashSet::new();
        for t in &self.first_phase.tasks {
            if t.id.trim().is_empty() {
                errs.push(format!("task {:?} has no id", t.title));
            } else if !task_ids.insert(t.id.trim().to_string()) {
                errs.push(format!("duplicate task id: {}", t.id));
            }
        }
        for t in &self.first_phase.tasks {
            for d in &t.depends_on {
                if !task_ids.contains(d.trim()) {
                    errs.push(format!("task {} depends on unknown task id: {}", t.id, d));
                }
            }
        }

        if errs.is_empty() {
            Ok(())
        } else {
            bail!("CEO plan failed validation:\n  - {}", errs.join("\n  - "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hat(name: &str) -> Hat {
        Hat {
            name: name.into(),
            role: "role".into(),
            responsibilities: vec!["x".into()],
        }
    }

    fn task(id: &str, deps: &[&str]) -> Task {
        Task {
            id: id.into(),
            title: "t".into(),
            role: "r".into(),
            task_type: "DEV".into(),
            description: "d".into(),
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn valid_plan() -> CompanyPlan {
        CompanyPlan {
            mission: "m".into(),
            organization: Organization { hats: vec![hat("CEO")] },
            roadmap: vec![Phase {
                name: "P0".into(),
                goal: "g".into(),
                exit_criteria: vec!["c".into()],
            }],
            contribution_model: "cm".into(),
            first_phase: FirstPhase {
                name: "P0".into(),
                tasks: vec![task("T1", &[]), task("T2", &["T1"])],
            },
        }
    }

    #[test]
    fn valid_plan_passes() {
        assert!(valid_plan().validate().is_ok());
    }

    #[test]
    fn duplicate_task_id_fails() {
        let mut p = valid_plan();
        p.first_phase.tasks.push(task("T1", &[]));
        assert!(p.validate().is_err());
    }

    #[test]
    fn unknown_dependency_fails() {
        let mut p = valid_plan();
        p.first_phase.tasks.push(task("T3", &["T99"]));
        assert!(p.validate().is_err());
    }

    #[test]
    fn empty_hats_fails() {
        let mut p = valid_plan();
        p.organization.hats.clear();
        assert!(p.validate().is_err());
    }

    #[test]
    fn duplicate_hat_name_is_case_insensitive() {
        let mut p = valid_plan();
        p.organization.hats.push(hat("ceo"));
        assert!(p.validate().is_err());
    }
}
