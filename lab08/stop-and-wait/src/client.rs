mod checksum;
mod protocol;
mod socket;

use crate::protocol::{receive_file, send_file, FileObject};
use crate::socket::BadSocket;
use anyhow::{Context, Result};
use log::info;
use std::fs::metadata;
use std::fs::File;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, UdpSocket};
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("debug"));
    let mut mode = String::new();
    let mut file_name = String::new();

    print!("Enter mode (upload/download): ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut mode)?;
    let mode = mode.trim().to_lowercase();

    print!("Enter file name: ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut file_name)?;
    let file_name = file_name.trim().to_string();

    let server_addr: SocketAddr = "127.0.0.1:8080".parse()?;

    let mut socket = BadSocket::new(UdpSocket::bind("0.0.0.0:0")?);

    match mode.as_str() {
        "upload" => {
            let mut file = File::open(&file_name)?;
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;
            upload_file(
                &mut socket,
                server_addr,
                FileObject {
                    path: file_name,
                    data,
                },
            )?;
        }
        "download" => {
            let file = download_file(&mut socket, server_addr, &file_name)?;
            let mut f = File::create(&file.path)?;
            f.write_all(&file.data)?;

            info!("File {} saved", file.path);
        }
        _ => {
            println!("Invalid mode. Please choose either 'upload' or 'download'.");
        }
    }

    Ok(())
}

fn upload_file(
    socket: &mut BadSocket,
    server_addr: SocketAddr,
    file_object: FileObject,
) -> Result<()> {
    info!("Uploading file: {}", file_object.path);

    let request = format!("UPLOAD");
    socket.send_packet(server_addr, request.as_bytes())?;

    send_file(socket, server_addr, &file_object)
}

fn download_file(
    socket: &mut BadSocket,
    server_addr: SocketAddr,
    file_name: &str,
) -> Result<FileObject> {
    info!("Requesting download of file: {}", file_name);

    let request = format!("DOWNLOAD {}", file_name);
    socket.send_packet(server_addr, request.as_bytes())?;

    receive_file(socket)
}
