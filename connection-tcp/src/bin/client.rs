use std::io::{self, Read, Write};
use std::net::{TcpStream, SocketAddr, IpAddr};
use std::thread;
use std::time::Duration;
use std::env;

fn main() {
    
    let hub_addr_str = env::var("HUB_ADDR")
        .unwrap_or_else(|_| "10.10.10.1:7878".to_string());

    let client_ip_str = env::var("CLIENT_IP")
        .unwrap_or_else(|_| "10.10.10.2".to_string());

    let hub_addr: SocketAddr = hub_addr_str
        .parse()
        .expect("HUB_ADDR inválida. Formato esperado: IP:PUERTO");

    let client_ip: IpAddr = client_ip_str
        .parse()
        .expect("CLIENT_IP inválida");

    println!("===============================");
    println!("Identidad configurada: {}", client_ip);
    println!("Intentando conectar al Hub {} ...", hub_addr);
    println!("===============================");

    let mut stream = loop {
        match TcpStream::connect_timeout(&hub_addr, Duration::from_secs(5)) {
            Ok(s) => {
                let local = s.local_addr().expect("Error obteniendo IP local");

                println!("Conectado exitosamente al Hub");
                println!("IP local usada: {}", local.ip());
                println!("Puerto local asignado: {}", local.port());
                println!("===============================");

                break s;
            }
            Err(e) => {
                println!("No se pudo conectar: {}. Reintentando en 3s...", e);
                thread::sleep(Duration::from_secs(3));
            }
        }
    };

    let mut reader_stream = stream
        .try_clone()
        .expect("No se pudo clonar el stream");

    let reader_handle = thread::spawn(move || {
        let mut buffer = [0u8; 1024];

        loop {
            match reader_stream.read(&mut buffer) {
                Ok(0) => {
                    println!("\nEl Hub cerró la conexión.");
                    break;
                }
                Ok(n) => {
                    let msg_str = String::from_utf8_lossy(&buffer[..n]).to_string();

                    if msg_str.starts_with("SOLVE:") {
                        let parts: Vec<&str> = msg_str.trim().split_whitespace().collect();
                        if parts.len() >= 4 {
                            if let (Ok(a), Ok(b)) = (parts[1].parse::<u16>(), parts[3].parse::<u16>()) {
                                let sum = a + b;
                                println!("\n[RETO RECIBIDO]: {} + {} = ?", a, b);
                                let response = format!("RESULT: {}", sum);
                                
                                if let Err(e) = reader_stream.write_all(response.as_bytes()) {
                                    println!("Error respondiendo reto: {}", e);
                                } else {
                                    println!("[RETO ENVIADO]: {}", sum);
                                }
                                continue; 
                            }
                        }
                    }

                    print!("\n[HUB]: {}", msg_str);
                    io::stdout().flush().unwrap();
                }
                Err(e) => {
                    println!("\nError leyendo del Hub: {}", e);
                    break;
                }
            }
        }
    });

    println!("Escribe mensajes para enviar (exit para salir):");

    loop {
        let mut input = String::new();

        if io::stdin().read_line(&mut input).is_err() {
            println!("Error leyendo entrada.");
            break;
        }

        let trimmed = input.trim();

        if trimmed == "exit" {
            println!("Cerrando conexión...");
            break;
        }

        if trimmed.is_empty() {
            continue;
        }

        if let Err(e) = stream.write_all(input.as_bytes()) {
            println!("Error enviando mensaje: {}", e);
            break;
        }
    }

    let _ = reader_handle.join();

    println!("Cliente finalizado.");
}
