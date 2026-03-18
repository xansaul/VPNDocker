use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{timeout, Duration};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use std::collections::{VecDeque, HashSet};

use mandelbrot_dist::models::{MandelbrotTask, TaskResult, Message};
use crate::tasks::calculate_timeout;

pub async fn tcp_accept_loop(
    addr: String,
    pending: Arc<Mutex<VecDeque<MandelbrotTask>>>,
    result_tx: mpsc::Sender<TaskResult>,
    workers: Arc<RwLock<HashSet<SocketAddr>>>,
) {
    let listener = TcpListener::bind(&addr).await
        .expect("No se pudo bindear el puerto TCP");
    println!("[TCP] Escuchando workers en {}", addr);

    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                println!("[TCP] Worker conectado: {}", addr);
                let p  = Arc::clone(&pending);
                let tx = result_tx.clone();
                let w  = Arc::clone(&workers);
                tokio::spawn(async move {
                    handle_worker(socket, addr, p, tx, w).await;
                });
            }
            Err(e) => eprintln!("[TCP] Error aceptando conexión: {}", e),
        }
    }
}

async fn handle_worker(
    mut socket: TcpStream,
    addr:       SocketAddr,
    pending:    Arc<Mutex<VecDeque<MandelbrotTask>>>,
    result_tx:  mpsc::Sender<TaskResult>,
    workers:    Arc<RwLock<HashSet<SocketAddr>>>,
) {
    {
        let mut w = workers.write().await;
        w.insert(addr);
        println!("[TCP] Worker {} registrado. Total workers: {}", addr, w.len());
    }
    
    loop {
        let task = {
            let mut queue = pending.lock().await;
            queue.pop_front()
        };

        let task = match task {
            Some(t) => t,
            None => {
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
        };

        let task_id = task.id;
        let job_id  = task.job_id.clone();

        let payload = match serde_json::to_vec(&Message::AssignTask(task.clone())) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[Worker {}] Error serializando tarea: {}", addr, e);
                pending.lock().await.push_back(task);
                continue; 
            }
        };

        let send_ok = timeout(
            Duration::from_secs(10),
            socket.write_all(&payload),
        ).await;

        if send_ok.is_err() || send_ok.unwrap().is_err() {
            eprintln!("[Worker {}] Error enviando tarea {}. Regresando a cola.", addr, task_id);
            pending.lock().await.push_back(task);
            break; 
        }

        println!("[Worker {}] -> Tarea {} enviada (job {})", addr, task_id, &job_id[..8]);

        let timeout_secs = calculate_timeout(&task);
        let mut full_data: Vec<u8> = Vec::new();
        let mut buffer = vec![0u8; 65536];

        let result = timeout(
            Duration::from_secs(timeout_secs),
            async {
                loop {
                    let n = socket.read(&mut buffer).await?;
                    if n == 0 {
                        return Err::<TaskResult, std::io::Error>(
                            std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "Worker desconectado")
                        );
                    }
                    full_data.extend_from_slice(&buffer[..n]);

                    if let Ok(Message::SubmitResult(r)) = serde_json::from_slice::<Message>(&full_data) {
                        return Ok(r);
                    }
                }
            }
        ).await;

        match result {
            Ok(Ok(task_result)) => {
                println!("[Worker {}] Resultado tarea {} recibido.", addr, task_id);
                let _ = result_tx.send(task_result).await;
            }
            Ok(Err(e)) => {
                eprintln!("[Worker {}] Fallo en conexión/lectura tarea {}: {}.", addr, task_id, e);
                pending.lock().await.push_back(task);
                break; 
            }
            Err(_) => {
                eprintln!("[Worker {}] Timeout tarea {}. Regresando a cola.", addr, task_id);
                pending.lock().await.push_back(task);
                continue; 
            }
        }
    }
    
    {
        let mut w = workers.write().await;
        w.remove(&addr);
        println!("[TCP] Worker {} desconectado y desregistrado.", addr);
    }
}