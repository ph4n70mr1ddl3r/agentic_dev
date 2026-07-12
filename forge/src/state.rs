//! Resumable per-task state (SQLite). Persists which tasks are done/failed so a
//! run picks up where the last one left off instead of redoing everything, and
//! `forge status` can report progress. Lives at `<repo>/.forge/state.db`.

use std::collections::HashSet;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

pub struct State {
    conn: Connection,
}

/// Persisted state of one task within a phase.
#[derive(Debug, Clone, Default)]
pub struct TaskState {
    pub status: String, // "done" | "failed" | "skipped"
    pub role: String,
    pub artifact: Option<String>,
    pub artifact_path: Option<String>,
    pub detail: Option<String>, // error (failed) or reason (skipped)
    pub attempts: u32,
    pub updated_at: u64,
}

impl State {
    /// Open (creating the db + schema if needed) at `db_path`.
    pub fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("create state dir {}", parent.display()))?;
            }
        }
        let conn = Connection::open(db_path)
            .with_context(|| format!("open state db {}", db_path.display()))?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS task_state (
                phase_id      INTEGER NOT NULL,
                task_id       TEXT NOT NULL,
                status        TEXT NOT NULL,
                role          TEXT NOT NULL DEFAULT '',
                artifact      TEXT,
                artifact_path TEXT,
                detail        TEXT,
                attempts      INTEGER NOT NULL DEFAULT 0,
                updated_at    INTEGER NOT NULL,
                PRIMARY KEY (phase_id, task_id)
            );
            CREATE TABLE IF NOT EXISTS phase_gate (
                phase_id    INTEGER PRIMARY KEY,
                status      TEXT NOT NULL,
                approved_at INTEGER,
                note        TEXT
            );",
        )?;
        Ok(Self { conn })
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    fn upsert(&self, phase: usize, task_id: &str, st: &TaskState) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO task_state
             (phase_id, task_id, status, role, artifact, artifact_path, detail, attempts, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                phase as i64,
                task_id,
                st.status,
                st.role,
                st.artifact,
                st.artifact_path,
                st.detail,
                st.attempts as i64,
                st.updated_at as i64,
            ],
        )?;
        Ok(())
    }

    pub fn mark_done(
        &self,
        phase: usize,
        task_id: &str,
        role: &str,
        artifact: &str,
        path: &str,
    ) -> Result<()> {
        self.upsert(
            phase,
            task_id,
            &TaskState {
                status: "done".into(),
                role: role.into(),
                artifact: Some(artifact.into()),
                artifact_path: Some(path.into()),
                detail: None,
                attempts: 0,
                updated_at: Self::now(),
            },
        )
    }

    pub fn mark_failed(&self, phase: usize, task_id: &str, role: &str, detail: &str) -> Result<()> {
        self.upsert(
            phase,
            task_id,
            &TaskState {
                status: "failed".into(),
                role: role.into(),
                detail: Some(detail.into()),
                updated_at: Self::now(),
                ..Default::default()
            },
        )
    }

    pub fn mark_skipped(
        &self,
        phase: usize,
        task_id: &str,
        role: &str,
        reason: &str,
    ) -> Result<()> {
        self.upsert(
            phase,
            task_id,
            &TaskState {
                status: "skipped".into(),
                role: role.into(),
                detail: Some(reason.into()),
                updated_at: Self::now(),
                ..Default::default()
            },
        )
    }

    /// Task ids recorded as `done` for this phase — seeds the orchestrator's
    /// satisfied-dependency set across runs (the resumability primitive).
    pub fn done_set(&self, phase: usize) -> Result<HashSet<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT task_id FROM task_state WHERE phase_id = ?1 AND status = 'done'")?;
        let rows = stmt.query_map(params![phase as i64], |r| r.get::<_, String>(0))?;
        let mut set = HashSet::new();
        for r in rows {
            set.insert(r?);
        }
        Ok(set)
    }

    /// All recorded rows for a phase, ordered by task id.
    pub fn list(&self, phase: usize) -> Result<Vec<(String, TaskState)>> {
        let mut stmt = self.conn.prepare(
            "SELECT task_id, status, role, artifact, artifact_path, detail, attempts, updated_at
             FROM task_state WHERE phase_id = ?1 ORDER BY task_id",
        )?;
        let rows = stmt.query_map(params![phase as i64], |r| {
            Ok((
                r.get::<_, String>(0)?,
                TaskState {
                    status: r.get::<_, String>(1)?,
                    role: r.get::<_, String>(2)?,
                    artifact: r.get::<_, Option<String>>(3)?,
                    artifact_path: r.get::<_, Option<String>>(4)?,
                    detail: r.get::<_, Option<String>>(5)?,
                    attempts: r.get::<_, i64>(6)? as u32,
                    updated_at: r.get::<_, i64>(7)? as u64,
                },
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Clear persisted state for one task (for a future `forge reset`).
    #[allow(dead_code)]
    pub fn reset_task(&self, phase: usize, task_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM task_state WHERE phase_id = ?1 AND task_id = ?2",
            params![phase as i64, task_id],
        )?;
        Ok(())
    }

    /// Clear persisted state for a whole phase (for a future `forge reset`).
    #[allow(dead_code)]
    pub fn reset_phase(&self, phase: usize) -> Result<()> {
        self.conn.execute(
            "DELETE FROM task_state WHERE phase_id = ?1",
            params![phase as i64],
        )?;
        Ok(())
    }
}

/// Approval state of a phase gate (ADR-0005).
#[derive(Debug, Clone)]
pub struct GateState {
    pub status: String, // "open" | "approved"
    pub note: Option<String>,
}

impl State {
    pub fn gate_status(&self, phase: usize) -> Result<Option<GateState>> {
        let mut stmt = self
            .conn
            .prepare("SELECT status, note FROM phase_gate WHERE phase_id = ?1")?;
        let mut rows = stmt.query_map(params![phase as i64], |r| {
            Ok(GateState {
                status: r.get::<_, String>(0)?,
                note: r.get::<_, Option<String>>(1)?,
            })
        })?;
        match rows.next() {
            Some(r) => Ok(Some(r?)),
            None => Ok(None),
        }
    }

    /// Mark a phase approved (human gate passed). ADR-0005: no later-phase work
    /// begins before the prior gate is approved.
    pub fn approve_phase(&self, phase: usize, note: Option<&str>) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO phase_gate (phase_id, status, approved_at, note)
             VALUES (?1, 'approved', ?2, ?3)",
            params![phase as i64, Self::now() as i64, note],
        )?;
        Ok(())
    }
}

#[cfg(test)]
impl State {
    pub fn open_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn done_failed_roundtrip_and_set() {
        let s = State::open_memory().unwrap();
        assert!(s.done_set(1).unwrap().is_empty());
        s.mark_done(
            1,
            "T3",
            "Solution Architect",
            "company",
            "erp/modules/generated/company.json",
        )
        .unwrap();
        s.mark_failed(1, "T7", "QA Engineer", "boom").unwrap();

        let done = s.done_set(1).unwrap();
        assert!(done.contains("T3"));
        assert!(!done.contains("T7"));

        let list = s.list(1).unwrap();
        assert_eq!(list.len(), 2);
        let t7 = list.iter().find(|(id, _)| id == "T7").unwrap();
        assert_eq!(t7.1.status, "failed");
        assert_eq!(t7.1.detail.as_deref(), Some("boom"));
    }

    #[test]
    fn reset_clears() {
        let s = State::open_memory().unwrap();
        s.mark_done(1, "T1", "Domain Modeler (Financials)", "ref", "p")
            .unwrap();
        assert!(s.done_set(1).unwrap().contains("T1"));
        s.reset_task(1, "T1").unwrap();
        assert!(!s.done_set(1).unwrap().contains("T1"));
    }

    #[test]
    fn gate_approve_roundtrip() {
        let s = State::open_memory().unwrap();
        assert!(s.gate_status(1).unwrap().is_none());
        s.approve_phase(1, Some("looks good")).unwrap();
        let g = s.gate_status(1).unwrap().unwrap();
        assert_eq!(g.status, "approved");
        assert_eq!(g.note.as_deref(), Some("looks good"));
        // gates are per-phase
        assert!(s.gate_status(2).unwrap().is_none());
    }

    #[test]
    fn phases_are_isolated() {
        let s = State::open_memory().unwrap();
        s.mark_done(1, "T1", "r", "a", "p").unwrap();
        s.mark_done(2, "T1", "r", "a", "p").unwrap();
        assert_eq!(s.done_set(1).unwrap().len(), 1);
        assert_eq!(s.done_set(2).unwrap().len(), 1);
        s.reset_phase(1).unwrap();
        assert!(s.done_set(1).unwrap().is_empty());
        assert_eq!(s.done_set(2).unwrap().len(), 1);
    }
}
