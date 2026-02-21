use connection_tcp::models::{Message, MandelbrotTask};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:7878").await?;

    println!("Hub iniciado en la VPN (10.10.10.1)");

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
        width: 100,
        height: 100,
        max_iter: 256,
    });

    let payload = serde_json::to_vec(&task)?;
    socket.write_all(&payload).await?;
    println!("Tarea enviada al worker.");

    let mut full_data = Vec::new();
    let mut buffer = vec![0u8; 8192];

    loop {
        let n = socket.read(&mut buffer).await?;
        if n == 0 { break; }
        
        full_data.extend_from_slice(&buffer[..n]);

        if let Ok(Message::SubmmitResult(result)) = serde_json::from_slice::<Message>(&full_data) {
            println!("¡Resultado recibido! Task ID: {}, Pixeles: {}", result.task_id, result.pixels.len());
            full_data.clear();
            break;
        }
    }
    Ok(())
}