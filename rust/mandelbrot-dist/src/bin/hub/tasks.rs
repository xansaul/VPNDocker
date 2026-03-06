use mandelbrot_dist::models::{MandelbrotTask, TaskResult, JobState, JobStatus};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};


pub fn divide_into_chunks(
    job_id:       &str,
    total_width:  usize,
    total_height: usize,
    max_iter:     u32,
    num_chunks:   usize,
) -> Vec<MandelbrotTask> {
    let rows_per_chunk = total_height / num_chunks;
    let mut tasks = Vec::with_capacity(num_chunks);

    for i in 0..num_chunks {
        let row_start = i * rows_per_chunk;
        let row_end   = if i == num_chunks - 1 { total_height } else { (i + 1) * rows_per_chunk };

        tasks.push(MandelbrotTask {
            id:           i as u32,
            job_id:       job_id.to_string(),
            x_start:     -2.5,
            x_end:        1.0,
            y_start:     -1.2,
            y_end:        1.2,
            row_start,
            row_end,
            total_width,
            total_height,
            max_iter,
        });
    }
    tasks
}
pub fn calculate_timeout(task: &MandelbrotTask) -> u64 {
    let pixels  = (task.row_end - task.row_start) * task.total_width;
    let estimate = (pixels as u64 * task.max_iter as u64) / 10_000_000;
    estimate.clamp(30, 600)
}

pub async fn result_collector(
    mut result_rx:     mpsc::Receiver<TaskResult>,
    jobs:              Arc<RwLock<HashMap<String, JobState>>>,
    _num_chunks_factor: usize,
) {
    println!("[Colector] Iniciado, esperando resultados...");

    while let Some(result) = result_rx.recv().await {
        let job_id    = result.job_id.clone();
        let task_id   = result.task_id as usize;
        let worker_id = result.worker_id.clone();

        let mut jobs_lock = jobs.write().await;
        if let Some(job) = jobs_lock.get_mut(&job_id) {
            if task_id < job.results.len() {
                job.results[task_id] = Some(result);
            }
            job.chunks_done += 1;

            println!(
                "[Colector] Chunk {}/{} del job {} (worker: {})",
                job.chunks_done, job.chunks_total, &job_id[..8], worker_id
            );

            if job.chunks_done == job.chunks_total {
                let output_path = format!("output/{}.png", job_id);
                let results: Vec<TaskResult> = job.results
                    .iter()
                    .flatten()
                    .cloned()
                    .collect();

                let width     = job.config.img_width;
                let height    = job.config.img_height;
                let max_iter  = job.config.max_iter;
                let out_path  = output_path.clone();
                let job_id_c  = job_id.clone();
                let jobs_ref  = Arc::clone(&jobs);

                drop(jobs_lock);

                tokio::spawn(async move {
                    println!("[Colector] Ensamblando imagen para job {}...", &job_id_c[..8]);
                    let result = tokio::task::spawn_blocking(move || {
                        assemble_and_save(&results, width, height, max_iter, &out_path)
                    }).await;

                    let mut jobs_lock = jobs_ref.write().await;
                    if let Some(job) = jobs_lock.get_mut(&job_id_c) {
                        match result {
                            Ok(Ok(())) => {
                                println!("[Colector] Imagen guardada: {}", output_path);
                                job.status = JobStatus::Done { output_path };
                            }
                            Ok(Err(e)) => {
                                eprintln!("[Colector] Error ensamblando imagen: {}", e);
                                job.status = JobStatus::Failed { reason: e.to_string() };
                            }
                            Err(e) => {
                                eprintln!("[Colector] Thread de ensamblado falló: {}", e);
                                job.status = JobStatus::Failed { reason: e.to_string() };
                            }
                        }
                    }
                });
            } else {
                job.status = JobStatus::Running {
                    chunks_done:  job.chunks_done,
                    chunks_total: job.chunks_total,
                };
            }
        } else {
            eprintln!("[Colector] Job {} no encontrado al recibir chunk {}", job_id, task_id);
        }
    }
}

fn assemble_and_save(
    results:  &[TaskResult],
    width:    usize,
    height:   usize,
    max_iter: u32,
    path:     &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::fs::create_dir_all("output")?;
    let mut pixel_data = vec![0u32; width * height];

    for result in results {
        let offset = result.row_start * width;
        for (i, &val) in result.pixels.iter().enumerate() {
            if offset + i < pixel_data.len() {
                pixel_data[offset + i] = val;
            }
        }
    }

    let img = image::ImageBuffer::from_fn(width as u32, height as u32, |x, y| {
        let idx = (y as usize) * width + (x as usize);
        iter_to_color(pixel_data[idx], max_iter)
    });

    img.save(path)?;
    Ok(())
}

fn iter_to_color(iter: u32, max_iter: u32) -> image::Rgb<u8> {
    if iter == max_iter {
        return image::Rgb([0, 0, 0]);
    }
    let t = iter as f64 / max_iter as f64;
    let r = (9.0  * (1.0 - t) * t * t * t * 255.0) as u8;
    let g = (15.0 * (1.0 - t) * (1.0 - t) * t * t * 255.0) as u8;
    let b = (8.5  * (1.0 - t) * (1.0 - t) * (1.0 - t) * t * 255.0) as u8;
    image::Rgb([r, g, b])
}