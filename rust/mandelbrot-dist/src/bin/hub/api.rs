use axum::{
    Json, Router, extract::{Path, State}, http::StatusCode, response::Html, routing::{get, post}
};
use std::net::SocketAddr;
use uuid::Uuid;

use mandelbrot_dist::models::{AppState, JobConfig, JobStatus, JobCreatedResponse, ListJobsResponse, JobSummary, JobState};

use crate::tasks::divide_into_chunks;

use tower_http::services::ServeDir;

pub async fn start_api(state: AppState, rest_port: &str) -> std::io::Result<()> {
    let app = Router::new()
        .route("/health",      get(|| async { "OK" }))
        .route("/jobs",        post(create_job))
        .route("/jobs",        get(list_jobs))
        .route("/gallery",     get(image_gallery))
        .route("/jobs/:id",    get(get_job_status))
        .nest_service("/images", ServeDir::new("output"))
        .with_state(state);

    let rest_addr: SocketAddr = format!("0.0.0.0:{}", rest_port).parse().unwrap();
    println!("[REST] Escuchando en http://{}", rest_addr);
    println!("[REST] Imágenes disponibles en http://{}/images", rest_addr);
    
    let listener = tokio::net::TcpListener::bind(rest_addr).await?;
    axum::serve(listener, app).await
}


async fn image_gallery() -> Html<String> {
    let mut html = String::from("<html><head><title>Galería Mandelbrot</title><meta charset=\"UTF-8\">");

    html.push_str("<style>
        body { font-family: system-ui, sans-serif; background: #121212; color: #ffffff; padding: 2rem; }
        h1 { text-align: center; margin-bottom: 2rem; }
        .gallery { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 20px; }
        .card { background: #1e1e1e; padding: 15px; border-radius: 12px; text-align: center; box-shadow: 0 4px 6px rgba(0,0,0,0.3); }
        .card img { max-width: 100%; height: auto; border-radius: 8px; transition: transform 0.2s; }
        .card img:hover { transform: scale(1.05); }
        .card a { color: #64b5f6; text-decoration: none; font-size: 0.9em; display: block; margin-top: 10px; word-break: break-all; }
    </style></head><body>");
    
    html.push_str("<h1>Galería de Fractales Generados</h1>");
    html.push_str("<div class=\"gallery\">");


    if let Ok(entries) = std::fs::read_dir("output") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |e| e == "png") {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    let img_url = format!("/images/{}", filename);
                    
                    html.push_str(&format!(
                        "<div class=\"card\">
                            <a href=\"{img_url}\" target=\"_blank\">
                                <img src=\"{img_url}\" alt=\"{filename}\" loading=\"lazy\" />
                            </a>
                            <a href=\"{img_url}\" target=\"_blank\">Ver Tamaño Completo<br><small>{filename}</small></a>
                        </div>",
                        img_url = img_url,
                        filename = filename
                    ));
                }
            }
        }
    } else {
        html.push_str("<p style='text-align: center; grid-column: 1 / -1;'>No hay imágenes generadas todavía. ¡Lanza algunos workers!</p>");
    }

    html.push_str("</div></body></html>");
    Html(html)
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
    let tasks = divide_into_chunks(&job_id, &config, num_chunks);

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
                "x_start":      job.config.x_start,
                "x_end":        job.config.x_end,
                "y_start":      job.config.y_start,
                "y_end":        job.config.y_end,
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
        x_start:    job.config.x_start,
        x_end:      job.config.x_end,
        y_start:    job.config.y_start,
        y_end:      job.config.y_end,
    }).collect();

    Json(ListJobsResponse { jobs: list })
}
