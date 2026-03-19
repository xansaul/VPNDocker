mod api;
mod tasks;
mod tcp;

use mandelbrot_dist::models::{AppState, TaskResult};

use std::env;
use std::collections::{VecDeque, HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use crate::tasks::result_collector;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rest_port = env::var("REST_PORT").unwrap_or("8080".into());
    let tcp_port  = env::var("TCP_PORT").unwrap_or("7878".into());



    println!("╔══════════════════════════════════════════╗");
    println!("║       Hub Mandelbrot — REST + TCP        ║");
    println!("║══════════════════════════════════════════║\n");
    println!("║  TCP workers   : 0.0.0.0:{}", tcp_port);
    println!("║  REST API      : 0.0.0.0:{}", rest_port);
    println!("╚══════════════════════════════════════════╝\n");


    let (result_tx, result_rx) = mpsc::channel::<TaskResult>(256);

    let state = AppState {
        pending_tasks: Arc::new(Mutex::new(VecDeque::new())),
        result_tx,
        jobs: Arc::new(RwLock::new(HashMap::new())),
        workers: Arc::new(RwLock::new(HashSet::new())),
    };

    {
        let jobs_map = Arc::clone(&state.jobs);
        tokio::spawn(result_collector(result_rx, jobs_map));
    }

    {
        let pending = Arc::clone(&state.pending_tasks);
        let tx      = state.result_tx.clone();
        let workers = Arc::clone(&state.workers);
        let addr    = format!("0.0.0.0:{}", tcp_port);
        tokio::spawn(async move {
            tcp::tcp_accept_loop(addr, pending, tx, workers).await;
        });
    }

    api::start_api(state, &rest_port).await?;

    Ok(())
}
