use std::collections::HashMap;
use std::env;
use std::net::UdpSocket;
use std::time::{Duration, Instant};

static mut CLIENT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
struct ClientState {
    last_seen: Instant,
    last_seq: u64,
}

fn parse_interval() -> u64 {
    let args: Vec<String> = env::args().collect();

    for i in 0..args.len() {
        if args[i] == "--interval-ms" && i + 1 < args.len() {
            return args[i + 1].parse().unwrap_or(5000);
        }
    }

    1000 // default
}


fn main() -> std::io::Result<()> {
    let interval_ms = parse_interval();
    unsafe { CLIENT_TIMEOUT = Duration::from_millis(interval_ms); }
    let socket = UdpSocket::bind("0.0.0.0:8080")?;
    socket.set_nonblocking(true)?;

    println!("Heartbeat server listening on 0.0.0.0:8080");

    let mut buf = [0u8; 1024];
    let mut clients: HashMap<std::net::SocketAddr, ClientState> = HashMap::new();

    loop {
        match socket.recv_from(&mut buf) {
            Ok((size, src)) => {
                let msg = String::from_utf8_lossy(&buf[..size]);
                let parts: Vec<&str> = msg.split_whitespace().collect();

                if parts.len() != 2 {
                    continue;
                }

                let seq: u64 = parts[0].parse().unwrap_or(0);
                let sent_time: f64 = parts[1].parse().unwrap_or(0.0);

                let now = Instant::now();

                if !clients.contains_key(&src) {
                    println!("New client connected: {}", src);
                }

                let entry = clients.entry(src).or_insert(ClientState {
                    last_seen: now,
                    last_seq: seq,
                });

                if seq > entry.last_seq + 1 {
                    println!(
                        "{} lost {} packets (expected {}, got {})",
                        src,
                        seq - entry.last_seq - 1,
                        entry.last_seq + 1,
                        seq
                    );
                }

                entry.last_seq = seq;
                entry.last_seen = now;

                let now_ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs_f64();

                let delay = now_ts - sent_time;

                println!(
                    "Heartbeat from {}: seq={} delay={:.3}s",
                    src, seq, delay
                );
            }

            Err(_) => {
            }
        }

        let now = Instant::now();

        let mut dead_clients = Vec::new();

        for (addr, state) in &clients {
            unsafe {
                if now.duration_since(state.last_seen) > CLIENT_TIMEOUT {
                    dead_clients.push(*addr);
                }
            }
        }

        for addr in dead_clients {
            println!("Client disconnected (timeout): {}", addr);
            clients.remove(&addr);
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}