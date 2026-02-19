use std::io::{self, Read, Write};
use std::net::{TcpStream, SocketAddr, TcpListener};
use std::thread;
use std::time::Duration;
use std::env;
use std::net::IpAddr;

fn main() {

    let hub_addr_str = env::var("HUB_ADDR")
        .unwrap_or_else(|_| "10.10.10.1:7878".to_string());

    let client_ip_str = env::var("CLIENT_IP")
        .unwrap_or_else(|_| "10.10.10.2".to_string());

    let hub_addr: SocketAddr = hub_addr_str
        .parse()
        .expect("HUB_ADDR inválida");

    let client_ip: IpAddr = client_ip_str
        .parse()
        .expect("CLIENT_IP inválida");

    println!("Identidad configurada: {}", client_ip);
    println!("Intentando conectar al Hub ({}) vía WireGuard...", hub_addr);

    let mut stream = loop {

        match TcpStream::connect_timeout(&hub_addr, Duration::from_secs(5)) {
            Ok(s) => {
                let local = s.local_addr().expect("Error al obtener IP local");

                println!("¡Conectado exitosamente!");
                println!("IP local usada: {}", local.ip());
                println!("Puerto local asignado: {}", local.port());

                break s;
            }
            Err(e) => {
                println!("No se pudo conectar al Hub: {}. Reintentando...", e);
                thread::sleep(Duration::from_secs(3));
            }
        }
    };

    let mut stream_escucha = stream.try_clone().expect("Error al clonar stream");

    thread::spawn(move || {
        let mut buffer = [0; 512];
        loop {
            match stream_escucha.read(&mut buffer) {
                Ok(0) => {
                    println!("\n[SISTEMA]: Conexión cerrada por el Hub.");
                    std::process::exit(0);
                }
                Ok(n) => {
                    print!("\n[HUB]: {}", String::from_utf8_lossy(&buffer[..n]));
                    io::stdout().flush().unwrap();
                }
                Err(e) => {
                    println!("\nError leyendo del Hub: {}", e);
                    break;
                }
            }
        }
    });

    println!("Escribe mensajes para enviar (o 'exit' para salir):");

    loop {
        let mut input = String::new();

        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        let trimmed = input.trim();

        if trimmed == "exit" {
            break;
        }

        if trimmed.is_empty() {
            continue;
        }

        if let Err(e) = stream.write_all(input.as_bytes()) {
            println!("Error al enviar: {}", e);
            break;
        }
    }
}
