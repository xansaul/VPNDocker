use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct MandelbrotTask {
    pub id: u32,
    pub x_start: f64,
    pub x_end: f64,
    pub y_start: f64,
    pub y_end: f64,
    pub width: usize,
    pub height: usize,
    pub max_iter: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TaskResult {
    pub task_id: u32,
    pub worker_id: String,
    pub pixels: Vec<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    AssignTask(MandelbrotTask),
    SubmmitResult(TaskResult),
    Error(String),
}