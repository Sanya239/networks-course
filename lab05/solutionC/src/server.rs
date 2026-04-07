use chrono::Local;
use std::net::UdpSocket;
use std::time::Duration;
use std::{env, thread};
fn current_time_string() -> String {
    let now = Local::now();

    now.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(anyhow::anyhow!("Usage: client <port>"));
    }

    let port = &args[1];

    let socket = UdpSocket::bind("0.0.0.0:0")?;

    socket.set_broadcast(true)?;

    println!("UDP time broadcast server started");

    let broadcast_addr = format!("255.255.255.255:{port}");

    loop {
        let message = current_time_string();

        socket.send_to(message.as_bytes(), &broadcast_addr)?;

        println!("Sent: {}", message);

        thread::sleep(Duration::from_secs(1));
    }
}
