mod client;

use std::net::UdpSocket;
use std::str;
use rand::{Rng, RngExt};

fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:8080")?;
    println!("UDP server listening on 0.0.0.0:8080");

    let mut buf = [0u8; 1024];
    let mut rng = rand::rng();

    loop {
        let (size, src) = socket.recv_from(&mut buf)?;

        let msg = match str::from_utf8(&buf[..size]) {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Invalid UTF-8 from {}", src);
                continue;
            }
        };

        println!("Received from {}: {}", src, msg);

        let drop = rng.random_bool(0.2);
        if drop {
            println!("Simulating packet loss, dropping packet from {}", src);
            continue;
        }

        let response = msg.to_uppercase();

        socket.send_to(response.as_bytes(), src)?;
        println!("Sent to {}: {}", src, response);
    }
}