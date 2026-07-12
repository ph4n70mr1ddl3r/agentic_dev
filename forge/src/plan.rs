use serde::{Deserialize, Serialize};

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
