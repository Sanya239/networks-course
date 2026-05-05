use anyhow::Result;
use eframe::{egui, Frame};
use egui::Ui;
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const BROADCAST_PORT: u16 = 8239;

#[derive(Clone)]
struct AppState {
    peers: Arc<Mutex<HashMap<SocketAddr, Instant>>>,
    heartbeat_interval: Arc<Mutex<u64>>,
}

fn create_socket(port: u16) -> UdpSocket {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).unwrap();
    socket.set_reuse_address(true).unwrap();
    socket.set_reuse_port(true).unwrap();

    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    socket.bind(&addr.into()).unwrap();

    socket.into()
}

fn start_network(port: u16, state: AppState) -> Result<()> {
    let socket = UdpSocket::bind(format!("0.0.0.0:{}", port))?;
    socket.set_broadcast(true)?;

    let recv_socket = create_socket(BROADCAST_PORT);
    let reply_socket = socket.try_clone()?;

    let peers = state.peers.clone();

    thread::spawn(move || {
        let mut buf = [0u8; 1024];

        loop {
            let (size, addr) = match recv_socket.recv_from(&mut buf) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let msg = String::from_utf8_lossy(&buf[..size]);
            let parts: Vec<&str> = msg.split_whitespace().collect();
            if parts.len() != 2 {
                continue;
            }

            let msg_port: u16 = match parts[1].parse() {
                Ok(p) => p,
                Err(_) => continue,
            };

            let peer = SocketAddr::new(addr.ip(), msg_port);

            if peer.port() == port {
                continue;
            }

            match parts[0] {
                "HELLO" | "HEARTBEAT" => {
                    let mut peers = peers.lock().unwrap();
                    let is_new = !peers.contains_key(&peer);
                    peers.insert(peer, Instant::now());

                    if is_new {
                        println!("New peer: {}", peer);
                        let _ = do_broadcast(port, &reply_socket);
                    }
                }
                "BYE" => {
                    let mut peers = peers.lock().unwrap();
                    peers.remove(&peer);
                }
                _ => {}
            }
        }
    });

    let hb_socket = socket.try_clone()?;
    let interval = state.heartbeat_interval.clone();

    thread::spawn(move || {
        loop {
            let interval_val = *interval.lock().unwrap();
            thread::sleep(Duration::from_secs(interval_val));

            let msg = format!("HEARTBEAT {}", port);
            let _ = hb_socket.send_to(
                msg.as_bytes(),
                format!("255.255.255.255:{}", BROADCAST_PORT),
            );
        }
    });

    let peers_cleanup = state.peers.clone();
    let interval = state.heartbeat_interval.clone();

    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(1));

            let timeout = 3 * *interval.lock().unwrap();
            let now = Instant::now();

            let mut peers = peers_cleanup.lock().unwrap();
            peers.retain(|peer, last_seen| {
                if now.duration_since(*last_seen).as_secs() > timeout {
                    println!("Peer timed out: {}", peer);
                    false
                } else {
                    true
                }
            });
        }
    });

    do_broadcast(port, &socket)?;

    Ok(())
}

fn do_broadcast(port: u16, socket: &UdpSocket) -> Result<()> {
    let msg = format!("HELLO {}", port);
    socket.send_to(
        msg.as_bytes(),
        format!("255.255.255.255:{}", BROADCAST_PORT),
    )?;
    Ok(())
}

struct MyApp {
    port: u16,
    state: AppState,
    input_interval: String,
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut Ui, frame: &mut Frame) {}

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Peer Discovery");

            ui.horizontal(|ui| {
                ui.label("Heartbeat interval (sec):");
                ui.text_edit_singleline(&mut self.input_interval);

                if ui.button("Apply").clicked() {
                    if let Ok(val) = self.input_interval.parse::<u64>() {
                        *self.state.heartbeat_interval.lock().unwrap() = val;
                    }
                }
            });

            ui.separator();

            ui.label("Peers:");

            let peers = self.state.peers.lock().unwrap();
            for p in peers.keys() {
                ui.label(format!("{}", p));
            }

            ui.separator();

            if ui.button("Exit").clicked() {
                let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
                socket.set_broadcast(true).unwrap();

                let msg = format!("BYE {}", self.port);
                let _ = socket.send_to(
                    msg.as_bytes(),
                    format!("255.255.255.255:{}", BROADCAST_PORT),
                );

                std::process::exit(0);
            }
        });
    }
}

fn main() -> Result<()> {
    let port: u16 = std::env::args()
        .nth(1)
        .expect("port required")
        .parse()
        .unwrap();

    let state = AppState {
        peers: Arc::new(Mutex::new(HashMap::new())),
        heartbeat_interval: Arc::new(Mutex::new(2)),
    };

    start_network(port, state.clone())?;

    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "Peer Discovery",
        options,
        Box::new(|_cc| {
            Ok(Box::new(MyApp {
                port,
                state,
                input_interval: "2".to_string(),
            }))
        }),
    )
    .unwrap();

    Ok(())
}
