use mandelbrot_dist::models::{MandelbrotTask, TaskResult, JobState, JobStatus, JobConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};


pub fn divide_into_chunks(
    job_id:       &str,
    config:       &JobConfig,
    num_chunks:   usize,
) -> Vec<MandelbrotTask> {
    let x_start = config.x_start;
    let y_start = config.y_start;
    let total_width = config.img_width;

    let rows_per_chunk = config.img_height / num_chunks;
    let mut tasks = Vec::with_capacity(num_chunks);

    let x_step = (config.x_end - x_start) / total_width as f64;
    let y_step = (config.y_end - y_start) / config.img_height as f64;

    for i in 0..num_chunks {
        let row_start = i * rows_per_chunk;
        let row_end   = if i == num_chunks - 1 { config.img_height } else { (i + 1) * rows_per_chunk };

        tasks.push(MandelbrotTask {
            id:           i as u32,
            job_id:       job_id.to_string(),
            x_start,
            x_step,        
            y_start,
            y_step,        
            row_start,
            row_end,
            total_width,
            max_iter:     config.max_iter,
        });
    }
    tasks
}
pub fn calculate_timeout(task: &MandelbrotTask) -> u64 {
    let pixels  = (task.row_end - task.row_start) * task.total_width;
    let estimate = (pixels as u64 * task.max_iter as u64) / 10_000_000;
    estimate.clamp(30, 7200)
}

pub async fn result_collector(
    mut result_rx:     mpsc::Receiver<TaskResult>,
    jobs:              Arc<RwLock<HashMap<String, JobState>>>,
) {
    println!("[Colector] Iniciado, esperando resultados...");

    while let Some(result) = result_rx.recv().await {
        let job_id    = result.job_id.clone();
        let task_id   = result.task_id as usize;
        let worker_id = result.worker_id.clone();

        let mut jobs_lock = jobs.write().await;
        if let Some(job) = jobs_lock.get_mut(&job_id) {
            
            if job.results[task_id].is_none() {
                job.results[task_id] = Some(result);
                job.chunks_done += 1;

                println!(
                    "[Colector] Chunk {}/{} del job {} recibido (Worker: {})",
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
                        println!("[Colector] Ensamblando imagen final para {}...", &job_id_c[..8]);
                        let result = tokio::task::spawn_blocking(move || {
                            assemble_and_save(&results, width, height, max_iter, &out_path)
                        }).await;

                        let mut jobs_lock = jobs_ref.write().await;
                        if let Some(job) = jobs_lock.get_mut(&job_id_c) {
                            match result {
                                Ok(Ok(())) => {
                                    println!("[Colector] ¡Imagen completada con éxito!: {}", output_path);
                                    job.status = JobStatus::Done { output_path };
                                }
                                Ok(Err(e)) => {
                                    job.status = JobStatus::Failed { reason: e.to_string() };
                                }
                                Err(e) => {
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
                println!("[Colector] Ignorando resultado duplicado de tarea {} (Job: {})", task_id, &job_id[..8]);
            }
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