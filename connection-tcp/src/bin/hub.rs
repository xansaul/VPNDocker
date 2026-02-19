use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

type ClientMap = Arc<Mutex<HashMap<String, TcpStream>>>;

fn handle_client(mut stream: TcpStream, clients: ClientMap) {
    let addr = stream.peer_addr().unwrap().to_string();
    println!("Nuevo cliente registrado: {}", addr);

    {
        let mut clients_guard = clients.lock().unwrap();
        clients_guard.insert(addr.clone(), stream.try_clone().expect("Error al clonar stream"));
    }

    let mut buffer = [0; 512];

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                println!("Cliente {} desconectado", addr);
                clients.lock().unwrap().remove(&addr);
                break;
            }
            Ok(n) => {
                let msg = String::from_utf8_lossy(&buffer[..n]).to_string();
                println!("Mensaje de {}: {}", addr, msg.trim());

                let mut clients_guard = clients.lock().unwrap();

                for (id, client_stream) in clients_guard.iter_mut() {
                    let response = format!(
                        "Broadcast desde el Hub para {}: {}",
                        id,
                        msg
                    );

                    if let Err(e) = client_stream.write_all(response.as_bytes()) {
                        println!("Error enviando a {}: {}", id, e);
                    }
                }
            }
            Err(e) => {
                println!("Error con {}: {}", addr, e);
                clients.lock().unwrap().remove(&addr);
                break;
            }
        }
    }
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:7878")
        .expect("No se pudo iniciar el Hub");

    let clients: ClientMap = Arc::new(Mutex::new(HashMap::new()));

    println!("Hub iniciado en 0.0.0.0:7878...");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let clients_clone = Arc::clone(&clients);
                thread::spawn(move || {
                    handle_client(stream, clients_clone);
                });
            }
            Err(e) => println!("Error aceptando conexión: {}", e),
        }
    }
}
