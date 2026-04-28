use crate::checksum::compute_checksum;
use crate::socket::BadSocket;
use anyhow::{bail, Result};
use log::info;
use std::net::SocketAddr;

pub struct FileObject {
    pub path: String,
    pub data: Vec<u8>,
}

pub fn send_file(socket: &mut BadSocket, server_addr: SocketAddr, file: &FileObject) -> Result<()> {
    const CHUNK_SIZE: usize = 10 * 1024;
    let total_packets = (file.data.len() + CHUNK_SIZE - 1) / CHUNK_SIZE;

    let meta = format!("{} {}", total_packets, file.path);
    socket.send_packet(server_addr, meta.as_bytes())?;

    for i in 0..total_packets {
        let start = i * CHUNK_SIZE;
        let end = usize::min(start + CHUNK_SIZE, file.data.len());
        let chunk = &file.data[start..end];

        socket.send_packet(server_addr, chunk)?;
    }
    let checksum = compute_checksum(&file.data);
    info!("Sent checksum: {}", checksum);
    socket.send_packet(server_addr, checksum.to_string().as_bytes())?;

    Ok(())
}

pub fn receive_file(socket: &mut BadSocket) -> Result<FileObject> {
    let mut buf = [0u8; 65536];

    let (size, sender) = socket.recv_no_timelimit(&mut buf)?;
    let msg = String::from_utf8_lossy(&buf[..size]);

    let parts = msg.split_once(" ");
    if parts.is_none() {
        bail!("Received invalid message: {:?}", msg);
    }

    let file_name = parts.unwrap().1.to_string();
    let total_packets: usize = parts.unwrap().0.parse()?;

    let mut data = Vec::new();

    for _ in 0..total_packets {
        loop {
            let (size, addr) = socket.recv_no_timelimit(&mut buf)?;

            if addr != sender {
                continue;
            }
            data.extend_from_slice(&buf[..size]);
            break;
        }
    }
    loop {
        let (size, addr) = socket.recv_no_timelimit(&mut buf)?;

        if addr != sender {
            continue;
        }
        let checksum = String::from_utf8(buf[..size].to_vec())?.parse::<u16>()?;

        if checksum != compute_checksum(&data) {
            bail!("Received invalid checksum: {}", checksum);
        }else{
            info!("Received checksum: {}", checksum);
        }

        break;
    }

    Ok(FileObject {
        path: file_name,
        data,
    })
}
