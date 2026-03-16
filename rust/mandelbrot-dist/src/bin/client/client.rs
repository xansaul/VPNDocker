use mandelbrot_dist::models::{Message, MandelbrotTask, TaskResult};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use std::net::IpAddr;
use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hub_addr_str  = env::var("HUB_ADDR").unwrap_or("127.0.0.1:7878".into());
    let client_ip: Option<IpAddr> = env::var("CLIENT_IP")
        .ok()
        .and_then(|s| s.parse().ok());

    println!("╔══════════════════════════════════════════╗");
    println!("║           Worker Mandelbrot              ║");
    println!("╠══════════════════════════════════════════╣");
    println!("║  IP Worker : {}", client_ip.map_or("Auto".into(), |ip| ip.to_string()));
    println!("║  Hub Addr  : {}", hub_addr_str);
    println!("╚══════════════════════════════════════════╝\n");

    let mut stream = connect_with_retry(&hub_addr_str, client_ip).await?;
    let local_addr  = stream.local_addr()?;
    println!("[Worker {}] Conectado al hub.\n", local_addr);

    let mut full_data: Vec<u8> = Vec::new();
    let mut buffer = vec![0u8; 65536];

    loop {
        let n = stream.read(&mut buffer).await?;
        if n == 0 {
            println!("[Worker {}] Hub cerró la conexión.", local_addr);
            break;
        }
        full_data.extend_from_slice(&buffer[..n]);

        match serde_json::from_slice::<Message>(&full_data) {
            Ok(msg) => {
                full_data.clear();
                match msg {
                    Message::AssignTask(task) => {
                        handle_task_async(&mut stream, task, &local_addr.to_string()).await?;
                    }
                    Message::Error(e) => {
                        eprintln!("[Worker {}] Error del hub: {}", local_addr, e);
                        break;
                    }
                    _ => {}
                }
            }
            Err(e) if e.is_eof() => continue,
            Err(e) => {
                eprintln!("[Worker {}] Error deserializando: {}", local_addr, e);
                full_data.clear();
            }
        }
    }

    println!("[Worker {}] Terminando.", local_addr);
    Ok(())
}

async fn handle_task_async(
    stream:    &mut TcpStream,
    task:      MandelbrotTask,
    worker_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let rows         = task.row_end - task.row_start;
    let total_pixels = rows * task.total_width;

    println!(
        "[Worker {}] > Tarea {} (job {}) | filas {}-{} | {} px | max_iter {}",
        worker_id, task.id, &task.job_id[..8],
        task.row_start, task.row_end, total_pixels, task.max_iter
    );

    let start      = std::time::Instant::now();
    let task_clone = task.clone();

    let pixels = tokio::task::spawn_blocking(move || {
        compute_mandelbrot(&task_clone)
    }).await?;

    println!(
        "[Worker {}] Tarea {} calculada en {:.2}s ({} px)",
        worker_id, task.id, start.elapsed().as_secs_f64(), pixels.len()
    );

    let result = Message::SubmitResult(TaskResult {
        task_id:   task.id,
        job_id:    task.job_id.clone(),
        worker_id: worker_id.to_string(),
        row_start: task.row_start,
        row_end:   task.row_end,
        pixels,
    });

    let payload = serde_json::to_vec(&result)?;
    stream.write_all(&payload).await?;
    stream.flush().await?;

    println!("[Worker {}] Tarea {} enviada al hub.", worker_id, task.id);
    Ok(())
}

fn compute_mandelbrot(task: &MandelbrotTask) -> Vec<u32> {
    let mut results = Vec::with_capacity((task.row_end - task.row_start) * task.total_width);

    for py in task.row_start..task.row_end {
        let cy = task.y_start + py as f64 * task.y_step;

        for px in 0..task.total_width {
            let cx = task.x_start + px as f64 * task.x_step;
            results.push(mandelbrot_iter(cx, cy, task.max_iter));
        }
    }

    results
}


#[inline(always)]
fn mandelbrot_iter(cx: f64, cy: f64, max_iter: u32) -> u32 {
    let (mut x, mut y, mut i) = (0.0f64, 0.0f64, 0u32);
    while x * x + y * y <= 4.0 && i < max_iter {
        let x_new = x * x - y * y + cx;
        y = 2.0 * x * y + cy;
        x = x_new;
        i += 1;
    }
    i
}


async fn connect_with_retry(
    hub_addr:  &str,
    client_ip: Option<IpAddr>,
) -> Result<TcpStream, Box<dyn std::error::Error>> {
    loop {

        match client_ip {
            None => {
                match TcpStream::connect(hub_addr).await {
                    Ok(s) => {
                        println!("[Worker] Conectado al hub en {}", hub_addr);
                        return Ok(s);
                    }
                    Err(e) => {
                        eprintln!("[Worker] No se pudo conectar a {}: {}. Reintentando...", hub_addr, e);
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                }
            }
            Some(ip) => {
                match tokio::net::lookup_host(hub_addr).await {
                    Ok(mut addrs) => {
                        if let Some(hub_socket_addr) = addrs.next() {
                            use tokio::net::TcpSocket;
                            let socket = TcpSocket::new_v4()?;
                            socket.set_reuseaddr(true)?;
                            if let Err(e) = socket.bind(std::net::SocketAddr::new(ip, 0)) {
                                eprintln!("[Worker] bind fallido: {}. Reintentando...", e);
                                tokio::time::sleep(Duration::from_secs(2)).await;
                                continue;
                            }
                            match socket.connect(hub_socket_addr).await {
                                Ok(s) => {
                                    println!("[Worker] Conectado a {} via {}", hub_addr, ip);
                                    return Ok(s);
                                }
                                Err(e) => {
                                    eprintln!("[Worker] No se pudo conectar: {}. Reintentando...", e);
                                    tokio::time::sleep(Duration::from_secs(2)).await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[Worker] DNS lookup fallido para {}: {}. Reintentando...", hub_addr, e);
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                }
            }
        }
    }
}