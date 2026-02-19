use std::io::{self, Read, Write};
use std::net::{TcpStream, SocketAddr};
use std::thread;
use std::time::Duration;
use socket2::{Socket, Domain, Type, Protocol};
use std::env;

fn main() {
    

    let hub_addr_str = env::var("HUB_ADDR").unwrap_or_else(|_| "10.10.10.1:7878".to_string());
    let client_ip_str = env::var("CLIENT_IP").expect("CLIENT_IP no definida en el .env");

    let hub_addr: SocketAddr = hub_addr_str.parse().expect("HUB_ADDR inválida");
    let local_addr: SocketAddr = format!("{}:0", client_ip_str).parse().expect("CLIENT_IP inválida");

    println!("Identidad configurada: {}", client_ip_str);
    println!("Intentando conectar al Hub ({}) vía WireGuard...", hub_addr);

    let mut stream = loop {
        let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP)).unwrap();
        
        if let Err(_) = socket.bind(&local_addr.into()) {
            println!("Error: La IP {} no está activa en el sistema. ¿Subiste WireGuard?", client_ip_str);
            thread::sleep(Duration::from_secs(3));
            continue;
        }

        match socket.connect(&hub_addr.into()) {
            Ok(_) => {
                println!("¡Conectado exitosamente!");
                break TcpStream::from(socket);
            }
            Err(e) => {
                println!("No se pudo conectar al Hub: {}. Reintentando...", e);
                thread::sleep(Duration::from_secs(3));
            }
        }
    };

    let mut stream_escucha = stream.try_clone().unwrap();
    thread::spawn(move || {
        let mut buffer = [0; 512];
        while let Ok(n) = stream_escucha.read(&mut buffer) {
            if n == 0 { break; }
            println!("\n[HUB]: {}", String::from_utf8_lossy(&buffer[..n]));
        }
    });

    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if input.trim() == "exit" { break; }
        let _ = stream.write_all(input.as_bytes());
    }
}