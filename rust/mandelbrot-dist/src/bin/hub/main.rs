mod models;
mod api;
mod tasks;


use std::env;
use std::collections::{VecDeque, HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use models::{AppState, TaskResult};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rest_port = env::var("REST_PORT").unwrap_or("8080".into());

    println!("╔══════════════════════════════════════════╗");
    println!("║       Hub Mandelbrot — REST API          ║");
    println!("╚══════════════════════════════════════════╝\n");

    let (result_tx, _result_rx) = mpsc::channel::<TaskResult>(256);
    let state = AppState {
        pending_tasks: Arc::new(Mutex::new(VecDeque::new())),
        result_tx,
        jobs: Arc::new(RwLock::new(HashMap::new())),
        workers: Arc::new(RwLock::new(HashSet::new())),
    };

    api::start_api(state, &rest_port).await?;

    Ok(())
}
