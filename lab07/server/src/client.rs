use std::net::UdpSocket;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::str;

fn main() -> std::io::Result<()> {
    let server_addr = "127.0.0.1:8080";

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(Duration::from_secs(1)))?;

    println!("PING {}:", server_addr);

    let mut rtts = Vec::new();
    let mut received = 0;

    for seq in 1..=10 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        let message = format!("Ping {} {:.6}", seq, now);

        let start = Instant::now();
        socket.send_to(message.as_bytes(), server_addr)?;

        let mut buf = [0u8; 1024];

        match socket.recv_from(&mut buf) {
            Ok((size, _)) => {
                let elapsed = start.elapsed().as_secs_f64();
                rtts.push(elapsed);
                received += 1;

                let reply = str::from_utf8(&buf[..size]).unwrap_or("<invalid utf8>");

                println!(
                    "{} bytes from {}: seq={} time={:.3} sec",
                    size, server_addr, seq, elapsed
                );

                println!("reply: {}", reply);
            }
            Err(_) => {
                println!("Request timed out (seq={})", seq);
            }
        }

        std::thread::sleep(Duration::from_secs(1));
    }

    let sent = 10;
    let lost = sent - received;
    let loss_percent = (lost as f64 / sent as f64) * 100.0;

    println!("\n--- {} ping statistics ---", server_addr);
    println!(
        "{} packets transmitted, {} received, {:.0}% packet loss",
        sent, received, loss_percent
    );

    if !rtts.is_empty() {
        let min = rtts.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = rtts.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let avg = rtts.iter().sum::<f64>() / rtts.len() as f64;

        println!(
            "rtt min/avg/max = {:.3}/{:.3}/{:.3} sec",
            min, avg, max
        );
    }

    Ok(())
}