use anyhow::{bail, Result};
use log::{info, warn};
use rand::RngExt;
use std::fmt::Display;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

#[derive(Copy, Clone)]
enum PackageNumber {
    Zero,
    One,
}

pub struct BadSocket {
    socket: UdpSocket,
    current_package_number: PackageNumber,
}

impl Display for PackageNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            PackageNumber::Zero => {
                write!(f, "Zero")
            }
            PackageNumber::One => {
                write!(f, "One")
            }
        }
    }
}

impl BadSocket {
    pub fn new(socket: UdpSocket) -> BadSocket {
        socket.set_read_timeout(Some(Duration::new(5, 0))).unwrap();
        BadSocket {
            socket,
            current_package_number: PackageNumber::Zero,
        }
    }

    fn send_to(&mut self, buf: &[u8], addr: SocketAddr) -> Result<()> {
        let mut rng = rand::rng();

        loop {
            let drop = rng.random_bool(0.3);
            if !drop {
                self.socket.send_to(buf, addr.clone())?;
            }
            match self.recv_ack() {
                Ok(_) => {
                    info!("Received ack {} from {}", self.current_package_number, addr);
                    return Ok(());
                }
                Err(_) => {
                    warn!("Error receiving ack",);
                }
            }
        }
    }

    pub fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        loop {
            let (size, addr) = self.socket.recv_from(buf)?;

            let pos = buf[..size]
                .iter()
                .position(|&b| b == b'\n')
                .ok_or_else(|| anyhow::anyhow!("Invalid packet format"))?;

            let header = std::str::from_utf8(&buf[..pos])?;

            if header == self.current_package_number.to_string() {
                self.send_ack(addr)?;

                buf.copy_within(pos + 1..size, 0);
                info!("Received from {}", addr);
                return Ok((size - pos - 1, addr));
            } else {
                warn!("Received invalid package: {}", header);
                self.swap_packege_number();
                self.send_ack(addr)?;
            }
        }
    }

    pub fn recv_no_timelimit(&mut self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        self.socket.set_read_timeout(None)?;
        let res = self.recv_from(buf);
        self.socket
            .set_read_timeout(Some(Duration::new(5, 0)))
            .unwrap();
        res
    }

    fn recv_ack(&mut self) -> Result<()> {
        let mut buf = [0u8; 1024];
        let (size, _addr) = self.socket.recv_from(buf.as_mut())?;
        let msg = String::from_utf8_lossy(&buf[..size]);
        let parts: Vec<&str> = msg.split_whitespace().collect();

        if parts.len() != 2 {
            bail!("Received invalid ack message");
        }
        if parts[0] != "ACK" {
            bail!("Received invalid ACK message");
        }
        if parts[1] != self.current_package_number.to_string() {
            info!("Received not suitable ACK message, retrying now");
            return self.recv_ack();
        }
        self.swap_packege_number();
        Ok(())
    }

    pub fn send_packet(&mut self, server_addr: SocketAddr, chunk: &[u8]) -> Result<()> {
        let mut packet = Vec::new();
        let header = format!("{}\n", self.current_package_number);
        packet.extend_from_slice(header.as_bytes());
        packet.extend_from_slice(chunk);

        self.send_to(&packet, server_addr)?;
        Ok(())
    }
    fn send_ack(&mut self, server_addr: SocketAddr) -> Result<()> {
        let mut packet = Vec::new();
        let header = format!("ACK {}\n", self.current_package_number);
        packet.extend_from_slice(header.as_bytes());
        self.swap_packege_number();
        self.socket.send_to(&packet, server_addr)?;
        Ok(())
    }

    fn swap_packege_number(&mut self) {
        match self.current_package_number {
            PackageNumber::Zero => {
                self.current_package_number = PackageNumber::One;
            }
            PackageNumber::One => {
                self.current_package_number = PackageNumber::Zero;
            }
        }
    }
}
