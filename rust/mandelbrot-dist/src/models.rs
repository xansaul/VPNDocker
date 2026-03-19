use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use std::time::Instant;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JobConfig {
    pub num_workers: Option<usize>,
    pub img_width: usize,
    pub img_height: usize,
    pub max_iter: u32,
    pub x_start: f64,
    pub x_end: f64,
    pub y_start: f64,
    pub y_end: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running {
        chunks_done: usize,
        chunks_total: usize,
    },
    Done {
        output_path: String,
    },
    Failed {
        reason: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MandelbrotTask {
    pub id: u32,
    pub job_id: String,
    pub x_start: f64,
    pub x_step: f64,
    pub y_start: f64,
    pub y_step: f64,
    pub row_start: usize,
    pub row_end: usize,
    pub total_width: usize,
    pub max_iter: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskResult {
    pub task_id: u32,
    pub job_id: String,
    pub worker_id: String,
    pub row_start: usize,
    pub row_end: usize,
    pub pixels: Vec<u32>,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    AssignTask(MandelbrotTask),
    SubmitResult(TaskResult),
    Error(String),
}

#[derive(Clone)]
pub struct AppState {
    pub pending_tasks: Arc<Mutex<VecDeque<MandelbrotTask>>>,
    #[allow(dead_code)]
    pub result_tx: mpsc::Sender<TaskResult>,
    pub jobs: Arc<RwLock<HashMap<String, JobState>>>,
    pub workers: Arc<RwLock<HashSet<SocketAddr>>>,
}

#[derive(Debug, Clone)]
pub struct JobState {
    pub config: JobConfig,
    pub status: JobStatus,
    pub chunks_total: usize,
    pub chunks_done: usize,
    #[allow(dead_code)]
    pub results: Vec<Option<TaskResult>>,
    pub start_time: Instant,
}

#[derive(Serialize)]
pub struct JobCreatedResponse {
    pub job_id: String,
    pub message: String,
}

#[allow(dead_code)]
#[derive(Serialize)]
pub struct JobStatusResponse {
    pub job_id: String,
    pub status: JobStatus,
}

#[derive(Serialize)]
pub struct ListJobsResponse {
    pub jobs: Vec<JobSummary>,
}

#[derive(Serialize)]
pub struct JobSummary {
    pub job_id: String,
    pub status: JobStatus,
    pub img_width: usize,
    pub img_height: usize,
    pub max_iter: u32,
    pub x_start: f64,
    pub x_end: f64,
    pub y_start: f64,
    pub y_end: f64,
}
