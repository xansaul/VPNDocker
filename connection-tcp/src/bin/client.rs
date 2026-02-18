use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::thread;
use std::env;
use std::time::Duration;

fn main() {
    let hub_addr = env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:7878".to_string());
    
    println!("Conectando al Hub en: {}...", hub_addr);

    let mut stream = loop {
        match TcpStream::connect(&hub_addr) {
            Ok(s) => break s,
            Err(_) => {
                println!("Hub no disponible, reintentando en 2 segundos...");
                thread::sleep(Duration::from_secs(2));
            }
        }
    };

    println!("¡Conectado exitosamente!");

    let mut stream_escucha = stream.try_clone().expect("Error al clonar stream");
    thread::spawn(move || {
        let mut buffer = [0; 512];
        loop {
            match stream_escucha.read(&mut buffer) {
                Ok(0) => {
                    println!("\n[INFO] Conexión cerrada por el Hub.");
                    std::process::exit(0);
                }
                Ok(n) => {
                    let msg = String::from_utf8_lossy(&buffer[..n]);
                    println!("\n[HUB DICE]: {}", msg);
                }
                Err(e) => {
                    println!("Error de lectura: {}", e);
                    break;
                }
            }
        }
    });

    println!("Escribe un mensaje para enviar al Hub (o 'exit' para salir):");
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Error al leer teclado");
        let input = input.trim();

        if input == "exit" {
            break;
        }

        if !input.is_empty() {
            if let Err(e) = stream.write_all(input.as_bytes()) {
                println!("Error al enviar: {}", e);
                break;
            }
        }
    }
}