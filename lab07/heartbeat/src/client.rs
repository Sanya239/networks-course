use std::net::UdpSocket;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::thread::sleep;

fn main() -> std::io::Result<()> {
    let server_addr = "127.0.0.1:8080";

    let socket = UdpSocket::bind("0.0.0.0:0")?;

    println!("Starting heartbeat client...");

    let mut seq: u64 = 1;

    loop {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        let message = format!("{} {:.6}", seq, now);

        socket.send_to(message.as_bytes(), server_addr)?;

        println!("Sent heartbeat seq={}", seq);

        seq += 1;

        sleep(Duration::from_secs(1));
    }
}