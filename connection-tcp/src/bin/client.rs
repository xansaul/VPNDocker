use std::io::{self, Read, Write};
use std::net::{TcpStream, SocketAddr};
use std::thread;
use std::time::Duration;
use std::env;

fn main() {

    let hub_addr_str = env::var("HUB_ADDR").unwrap_or_else(|_| "10.10.10.1:7878".to_string());
    let client_ip_str = env::var("CLIENT_IP").unwrap_or_else(|_| "10.10.10.1".to_string());

    let hub_addr: SocketAddr = hub_addr_str.parse().expect("HUB_ADDR inválida");

    println!("Identidad configurada (referencial): {}", client_ip_str);
    println!("Intentando conectar al Hub ({}) vía WireGuard...", hub_addr);

    let mut stream = loop {
        match TcpStream::connect_timeout(&hub_addr, Duration::from_secs(5)) {
            Ok(s) => {
                let local = s.local_addr().expect("No se pudo obtener la dirección local");
                println!("¡Conectado exitosamente!");
                println!("Puerto de origen asignado por el sistema: {}", local.port());
                break s;
            }
            Err(e) => {
                println!("No se pudo conectar al Hub: {}. ¿Está activo el túnel?", e);
                thread::sleep(Duration::from_secs(3));
            }
        }
    };

    let mut stream_escucha = stream.try_clone().expect("No se pudo clonar el stream");
    thread::spawn(move || {
        let mut buffer = [0; 512];
        loop {
            match stream_escucha.read(&mut buffer) {
                Ok(0) => {
                    println!("\n[SISTEMA]: El Hub cerró la conexión.");
                    std::process::exit(0);
                }
                Ok(n) => {
                    print!("\n[HUB]: {}", String::from_utf8_lossy(&buffer[..n]));
                    io::stdout().flush().unwrap();
                }
                Err(_) => break,
            }
        }
    });

    println!("Escribe un mensaje (o 'exit' para salir):");
    loop {
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() { break; }
        
        let msg = input.trim();
        if msg == "exit" { break; }
        if msg.is_empty() { continue; }

        if let Err(e) = stream.write_all(input.as_bytes()) {
            println!("Error al enviar mensaje: {}", e);
            break;
        }
    }
}