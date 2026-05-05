use std::env;
use std::net::{IpAddr, SocketAddr, TcpListener};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        eprintln!("Usage: {} <IP> <start_port> <end_port>", args[0]);
        std::process::exit(1);
    }

    let ip: IpAddr = args[1].parse()?;
    let start: u16 = args[2].parse()?;
    let end: u16 = args[3].parse()?;

    if start > end {
        eprintln!("Invalid range: start_port > end_port");
        std::process::exit(1);
    }

    println!("Scanning {} ports {}-{}...", ip, start, end);

    for port in start..=end {
        let addr = SocketAddr::new(ip, port);

        match TcpListener::bind(addr) {
            Ok(listener) => {
                println!("FREE: {}", port);
                drop(listener);
            }
            Err(_) => {
                println!("OCCUPIED: {}", port);
            }
        }
    }

    Ok(())
}