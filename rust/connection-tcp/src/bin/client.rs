use tokio::net::TcpSocket;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::net::{IpAddr, SocketAddr};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hub_addr_str = env::var("HUB_ADDR")
        .unwrap_or_else(|_| "10.10.10.1:7878".to_string());
    let client_ip_str = env::var("CLIENT_IP")
        .unwrap_or_else(|_| "10.10.10.2".to_string());

    let hub_addr: SocketAddr = hub_addr_str.parse()?;
    let client_ip: IpAddr = client_ip_str.parse()?;

    let socket = if client_ip.is_ipv4() {
        TcpSocket::new_v4()?
    } else {
        TcpSocket::new_v6()?
    };

    socket.bind(SocketAddr::new(client_ip, 0))?; 

    println!("Conectando desde la IP asignada: {}", client_ip);

    let mut stream = socket.connect(hub_addr).await?;
    println!("Conexión establecida con el Hub {}", hub_addr_str);

    let mut buffer = vec![0u8; 10000];

    loop {
        let n = stream.read(&mut buffer).await?;
        if n == 0 { 
            println!("Conexión cerrada por el servidor.");
            break; 
        }

        let msg: Message = serde_json::from_slice(&buffer[..n])?;
        
        match msg {
            Message::AssignTask(task) => {
                println!("[Tarea {}] Iniciando cálculo de Mandelbrot...", task.id);
                let pixels = compute_mandelbrot(&task);
                
                let result = Message::SubmmitResult(TaskResult {
                    task_id: task.id,
                    worker_id: client_ip_str.clone(),
                    pixels,
                });
                
                let response = serde_json::to_vec(&result)?;
                stream.write_all(&response).await?;
                println!("[Tarea {}] Resultado enviado satisfactoriamente.", task.id);
            },
            _ => {}
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
            
            let mut x = 0.0;
            let mut y = 0.0;
            let mut i = 0;
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