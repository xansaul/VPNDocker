use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use models::Message;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type ClientMap = Arc<Mutex<HashMap<String, TcpStream>>>;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:7878").await?;
    println!("Hub iniciado en la VPN (10.10.10.1) [cite: 20]");

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("Nuevo worker conectado: {}", addr);

        tokio::spawn(async move {
            if let Err(e) = handle_worker(socket).await {
                println!("Error con el worker {}: {}", addr, e);
            }
        });
    }
}


async fn handle_worker(mut socket: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    
    let task = Message::AssignTask(MandelbrotTask {
        id: 1,
        x_start: -2.0, x_end: 0.5,
        y_start: -1.2, y_end: 1.2,
        width: 800, height: 600,
        max_iter: 1000,
    });

    let payload = serde_json::to_vec(&task)?;
    socket.write_all(&payload).await?;
    Ok(())
}