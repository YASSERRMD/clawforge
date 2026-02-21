//! Cron tool: CRUD operations on scheduled tasks from within agent sessions.
//!
//! Mirrors `src/tools/cron-tool.ts`.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

/// A cron job managed by the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CronJob {
    pub id: String,
    pub name: String,
    /// Cron expression (e.g., "0 9 * * 1-5").
    pub schedule: String,
    /// Task description to run on schedule.
    pub task: String,
    /// Agent/session that owns this job.
    pub owner: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    /// Run count since creation.
    pub run_count: u64,
}

/// Input to create a cron job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCronInput {
    pub name: String,
    pub schedule: String,
    pub task: String,
    pub owner: String,
}

/// Input to update an existing cron job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCronInput {
    pub id: String,
    pub name: Option<String>,
    pub schedule: Option<String>,
    pub task: Option<String>,
    pub enabled: Option<bool>,
}

/// Cron tool action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum CronToolInput {
    Create(CreateCronInput),
    Update(UpdateCronInput),
    Delete { id: String },
    List { owner: Option<String> },
    Get { id: String },
}

/// Cron tool output.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CronToolOutput {
    pub success: bool,
    pub job: Option<CronJob>,
    pub jobs: Option<Vec<CronJob>>,
    pub message: Option<String>,
}

/// Backend trait for cron job persistence.
#[async_trait]
pub trait CronBackend: Send + Sync {
    async fn create(&self, job: CronJob) -> Result<CronJob>;
    async fn update(&self, input: UpdateCronInput) -> Result<Option<CronJob>>;
    async fn delete(&self, id: &str) -> Result<bool>;
    async fn list(&self, owner: Option<&str>) -> Result<Vec<CronJob>>;
    async fn get(&self, id: &str) -> Result<Option<CronJob>>;
}

/// In-memory cron backend for testing.
pub struct InMemoryCronBackend {
    jobs: Arc<RwLock<HashMap<String, CronJob>>>,
}

impl InMemoryCronBackend {
    pub fn new() -> Self {
        Self { jobs: Arc::new(RwLock::new(HashMap::new())) }
    }
}

impl Default for InMemoryCronBackend {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl CronBackend for InMemoryCronBackend {
    async fn create(&self, job: CronJob) -> Result<CronJob> {
        self.jobs.write().await.insert(job.id.clone(), job.clone());
        Ok(job)
    }

    async fn update(&self, input: UpdateCronInput) -> Result<Option<CronJob>> {
        if let Some(job) = self.jobs.write().await.get_mut(&input.id) {
            if let Some(name) = input.name { job.name = name; }
            if let Some(schedule) = input.schedule { job.schedule = schedule; }
            if let Some(task) = input.task { job.task = task; }
            if let Some(enabled) = input.enabled { job.enabled = enabled; }
            Ok(Some(job.clone()))
        } else {
            Ok(None)
        }
    }

    async fn delete(&self, id: &str) -> Result<bool> {
        Ok(self.jobs.write().await.remove(id).is_some())
    }

    async fn list(&self, owner: Option<&str>) -> Result<Vec<CronJob>> {
        let jobs = self.jobs.read().await;
        Ok(jobs.values()
            .filter(|j| owner.map(|o| j.owner == o).unwrap_or(true))
            .cloned().collect())
    }

    async fn get(&self, id: &str) -> Result<Option<CronJob>> {
        Ok(self.jobs.read().await.get(id).cloned())
    }
}

/// Execute a cron tool action.
pub async fn run_cron_tool(backend: &dyn CronBackend, input: CronToolInput) -> Result<CronToolOutput> {
    match input {
        CronToolInput::Create(create) => {
            let job = CronJob {
                id: Uuid::new_v4().to_string(),
                name: create.name,
                schedule: create.schedule,
                task: create.task,
                owner: create.owner,
                enabled: true,
                created_at: Utc::now(),
                last_run: None,
                next_run: None,
                run_count: 0,
            };
            let created = backend.create(job).await?;
            info!(id = %created.id, schedule = %created.schedule, "Cron job created");
            Ok(CronToolOutput { success: true, job: Some(created), jobs: None, message: None })
        }
        CronToolInput::Update(update) => {
            let job = backend.update(update).await?;
            let found = job.is_some();
            Ok(CronToolOutput {
                success: found,
                job,
                jobs: None,
                message: if !found { Some("Job not found".to_string()) } else { None },
            })
        }
        CronToolInput::Delete { id } => {
            let deleted = backend.delete(&id).await?;
            debug!(id = %id, deleted = deleted, "Cron job delete");
            Ok(CronToolOutput {
                success: deleted,
                job: None,
                jobs: None,
                message: if !deleted { Some("Job not found".to_string()) } else { None },
            })
        }
        CronToolInput::List { owner } => {
            let jobs = backend.list(owner.as_deref()).await?;
            Ok(CronToolOutput { success: true, job: None, jobs: Some(jobs), message: None })
        }
        CronToolInput::Get { id } => {
            let job = backend.get(&id).await?;
            Ok(CronToolOutput {
                success: job.is_some(),
                job,
                jobs: None,
                message: None,
            })
        }
    }
}
