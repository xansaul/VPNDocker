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
        x_start: -2.0,
        x_end: 0.5,
        y_start: -1.2,
        y_end: 1.2,
        width: 800,
        height: 600,
        max_iter: 1000,
    });

    let payload = serde_json::to_vec(&task)?;
    socket.write_all(&payload).await?;
    println!("Tarea enviada al worker.");

    let mut buffer = vec![0u8; 1024 * 1024]; 
    loop {
        let n = socket.read(&mut buffer).await?;
        
        if n == 0 {
            println!("El worker cerró la conexión.");
            break;
        }

        match serde_json::from_slice::<Message>(&buffer[..n]) {
            Ok(Message::SubmmitResult(result)) => {
                println!("Task ID: {}, Worker: {}", result.task_id, result.worker_id);
                println!("Pixeles calculados: {}", result.pixels.len());
            }
            Ok(_) => println!("Recibido mensaje inesperado."),
            Err(e) => {
                println!("Error al deserializar respuesta: {}", e);
                break;
            }
        }
    }

    Ok(())
}