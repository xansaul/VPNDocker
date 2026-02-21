use tokio::net::TcpSocket;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::net::{IpAddr, SocketAddr};
use std::env;
use std::time::Duration;

use connection_tcp::models::{
    Message,
    MandelbrotTask,
    TaskResult,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hub_addr_str = env::var("HUB_ADDR").unwrap_or_else(|_| "10.10.10.1:7878".to_string());
    let client_ip_str = env::var("CLIENT_IP").unwrap_or_else(|_| "10.10.10.2".to_string());

    let hub_addr: SocketAddr = hub_addr_str.parse()?;
    let client_ip: IpAddr = client_ip_str.parse()?;

    println!("Identidad configurada: {}", client_ip);

    let mut stream = loop {
        let socket = TcpSocket::new_v4()?;
        socket.set_reuseaddr(true)?;
        
        if let Err(e) = socket.bind(SocketAddr::new(client_ip, 0)) {
            println!("Error en bind (interfaz wg0 no lista?): {}. Reintentando...", e);
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }

        match socket.connect(hub_addr).await {
            Ok(s) => {
                println!("Conectado exitosamente al Hub en {}", hub_addr);
                break s;
            }
            Err(e) => {
                println!("No se pudo conectar (Hub no listo): {}. Reintentando...", e);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    };

    let local_addr = stream.local_addr()?;
    let mut buffer = vec![0u8; 65536];

    loop {
        let n = stream.read(&mut buffer).await?;
        if n == 0 { 
            println!("Conexión terminada por el Hub.");
            break; 
        }

        if let Ok(msg) = serde_json::from_slice::<Message>(&buffer[..n]) {
            match msg {
                Message::AssignTask(task) => {
                    println!("[Tarea {}] Calculando Mandelbrot...", task.id);
                    let pixels = compute_mandelbrot(&task);
                    
                    let result = Message::SubmmitResult(TaskResult {
                        task_id: task.id,
                        worker_id: local_addr.to_string(),
                        pixels,
                    });
                    
                    let response = serde_json::to_vec(&result)?;
                    stream.write_all(&response).await?;
                    println!("[Tarea {}] Resultado enviado desde {}", task.id, local_addr);
                },
                _ => {}
            }
        }
    }
    Ok(())
}

fn compute_mandelbrot(task: &MandelbrotTask) -> Vec<u32> {
    let mut results = Vec::with_capacity(task.width * task.height);
    for py in 0..task.height {
        for px in 0..task.width {
            let cx = task.x_start + (px as f64 / task.width as f64) * (task.x_end - task.x_start);
            let cy = task.y_start + (py as f64 / task.height as f64) * (task.y_end - task.y_start);
            let (mut x, mut y, mut i) = (0.0, 0.0, 0);
            while x*x + y*y <= 4.0 && i < task.max_iter {
                let temp = x*x - y*y + cx;
                y = 2.0*x*y + cy;
                x = temp;
                i += 1;
            }
            results.push(i);
        }
    }
    results
}