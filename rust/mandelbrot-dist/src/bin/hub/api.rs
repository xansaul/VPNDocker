use axum::{
    routing::{get, post},
    Router, 
    Json, 
    extract::{State, Path}, 
    http::StatusCode,
};
use std::net::SocketAddr;
use uuid::Uuid;

use mandelbrot_dist::models::{AppState, JobConfig, JobStatus, JobCreatedResponse, ListJobsResponse, JobSummary, JobState};

use crate::tasks::divide_into_chunks;

pub async fn start_api(state: AppState, rest_port: &str) -> std::io::Result<()> {
    let app = Router::new()
        .route("/health",      get(|| async { "OK" }))
        .route("/jobs",        post(create_job))
        .route("/jobs",        get(list_jobs))
        .route("/jobs/:id",    get(get_job_status))
        .with_state(state);

    let rest_addr: SocketAddr = format!("0.0.0.0:{}", rest_port).parse().unwrap();
    println!("[REST] Escuchando en http://{}", rest_addr);
    
    let listener = tokio::net::TcpListener::bind(rest_addr).await?;
    axum::serve(listener, app).await
}

async fn create_job(
    State(state): State<AppState>,
    Json(config): Json<JobConfig>,
) -> (StatusCode, Json<JobCreatedResponse>) {
    let job_id = Uuid::new_v4().to_string();
    
    let workers_count = match config.num_workers {
        Some(n) if n > 0 => n,
        _ => {
            let workers = state.workers.read().await;
            if workers.is_empty() { 1 } else { workers.len() }
        }
    };

    let num_chunks = workers_count * 4;
    let tasks = divide_into_chunks(&job_id, config.img_width, config.img_height, config.max_iter, num_chunks);

    {
        let mut jobs = state.jobs.write().await;
        jobs.insert(job_id.clone(), JobState {
            config:       config.clone(),
            status:       JobStatus::Queued,
            chunks_total: num_chunks,
            chunks_done:  0,
            results:      (0..num_chunks).map(|_| None).collect(),
        });
    }

    {
        let mut queue = state.pending_tasks.lock().await;
        for task in tasks { queue.push_back(task); }
    }

    println!("[REST] Job creado: {} | {}x{} | {} chunks | max_iter {}",
        job_id, config.img_width, config.img_height, num_chunks, config.max_iter);
    {
        let mut jobs = state.jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            job.status = JobStatus::Running { chunks_done:  0, chunks_total: num_chunks };
        }
    }

    (
        StatusCode::CREATED,
        Json(JobCreatedResponse {
            job_id: job_id.clone(),
            message: format!("Job {} creada con {} chunks. Workers disponibles: {}", job_id, num_chunks, workers_count),
        }),
    )
}

async fn get_job_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let jobs = state.jobs.read().await;
    match jobs.get(&job_id) {
        Some(job) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "job_id": job_id,
                "status": job.status,
                "chunks_done":  job.chunks_done,
                "chunks_total": job.chunks_total,
                "img_width":    job.config.img_width,
                "img_height":   job.config.img_height,
                "max_iter":     job.config.max_iter,
            })),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("Job {} no encontrado", job_id) })),
        ),
    }
}

async fn list_jobs(State(state): State<AppState>) -> Json<ListJobsResponse> {
    let jobs = state.jobs.read().await;
    let list = jobs.iter().map(|(id, job)| JobSummary {
        job_id:     id.clone(),
        status:     job.status.clone(),
        img_width:  job.config.img_width,
        img_height: job.config.img_height,
        max_iter:   job.config.max_iter,
    }).collect();

    Json(ListJobsResponse { jobs: list })
}
