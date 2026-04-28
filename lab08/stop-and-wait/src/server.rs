mod checksum;
mod protocol;
mod socket;

use crate::protocol::{receive_file, send_file, FileObject};
use crate::socket::BadSocket;
use anyhow::Result;
use log::{info, warn};
use std::fs::File;
use std::io::{Read, Write};
use std::net::{SocketAddr, UdpSocket};

fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("debug"));
    let socket = UdpSocket::bind("0.0.0.0:8080")?;
    let mut socket = BadSocket::new(socket);

    println!("Server started on 0.0.0.0:8080");

    let mut buf = [0u8; 65536];

    loop {
        let (size, addr) = socket.recv_no_timelimit(&mut buf)?;
        let msg = String::from_utf8_lossy(&buf[..size]).to_string();

        info!("Received command from {}: {}", addr, msg);

        if msg.starts_with("UPLOAD") {
            handle_upload(&mut socket, addr)?;
        } else if msg.starts_with("DOWNLOAD") {
            let parts: Vec<&str> = msg.split_whitespace().collect();
            if parts.len() != 2 {
                warn!("Invalid DOWNLOAD command");
                continue;
            }
            let file_name = parts[1];
            handle_download(&mut socket, addr, file_name)?;
        } else {
            warn!("Unknown command: {}", msg);
        }
    }
}

fn handle_upload(socket: &mut BadSocket, addr: SocketAddr) -> Result<()> {
    info!("Receiving file from {}", addr);

    let file = receive_file(socket)?;

    let mut f = File::create(&file.path)?;
    f.write_all(&file.data)?;

    info!("File {} saved", file.path);

    Ok(())
}

fn handle_download(socket: &mut BadSocket, addr: SocketAddr, file_name: &str) -> Result<()> {
    info!("Sending file {} to {}", file_name, addr);

    let mut file = File::open(file_name)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    let file_object = FileObject {
        path: file_name.to_string(),
        data,
    };

    send_file(socket, addr, &file_object)
}
