use socket2::{Domain, Protocol, SockAddr, Socket, Type};

use std::env;
use std::mem::MaybeUninit;
use std::net::{SocketAddr, ToSocketAddrs};
use std::process;
use std::time::{Duration, Instant};
use anyhow::Result;
const ICMP_ECHO_REQUEST: u8 = 8;
const ICMP_ECHO_REPLY: u8 = 0;
const ICMP_TIME_EXCEEDED: u8 = 11;

const MAX_HOPS: u32 = 30;
const PACKET_SIZE: usize = 64;

fn checksum(data: &[u8]) -> u16 {
    let mut sum = 0u32;

    let mut chunks = data.chunks_exact(2);

    for chunk in &mut chunks {
        let value = u16::from_be_bytes([chunk[0], chunk[1]]) as u32;

        sum += value;
    }

    if let Some(&byte) = chunks.remainder().first() {
        sum += (byte as u32) << 8;
    }

    while (sum >> 16) != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }

    !(sum as u16)
}

fn resolve_host(host: &str) -> Result<SocketAddr> {
    let addr = format!("{}:0", host);

    Ok(addr.to_socket_addrs()?
        .find(|a| a.is_ipv4())
        .unwrap_or_else(|| {
            eprintln!("No IPv4 address found");

            process::exit(1);
        }))
}

fn build_icmp_packet(seq: u16, identifier: u16) -> [u8; PACKET_SIZE] {
    let mut packet = [0u8; PACKET_SIZE];

    packet[0] = ICMP_ECHO_REQUEST;
    packet[1] = 0;

    packet[4..6].copy_from_slice(&identifier.to_be_bytes());
    packet[6..8].copy_from_slice(&seq.to_be_bytes());

    let checksum = checksum(&packet);

    packet[2..4].copy_from_slice(&checksum.to_be_bytes());

    packet
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage:");
        eprintln!("sudo cargo run -- <host> [probes_per_hop]");

        process::exit(1);
    }

    let host = &args[1];

    let probes_per_hop: u32 = if args.len() >= 3 {
        args[2].parse().unwrap_or(3)
    } else {
        3
    };

    let target = resolve_host(host)?;

    println!(
        "traceroute to {} ({}), {} hops max",
        host,
        target.ip(),
        MAX_HOPS
    );

    let socket = Socket::new(
        Domain::IPV4,
        Type::RAW,
        Some(Protocol::ICMPV4),
    )?;

    socket.set_read_timeout(Some(Duration::from_secs(2)))?;

    let sockaddr = SockAddr::from(target);

    let identifier = process::id() as u16;

    let mut sequence: u16 = 1;

    for ttl in 1..=MAX_HOPS {
        socket.set_ttl_v4(ttl)?;

        print!("{:<3}", ttl);

        let mut destination_reached = false;

        for _ in 0..probes_per_hop {
            let packet = build_icmp_packet(sequence, identifier);

            let start = Instant::now();

            socket.send_to(&packet, &sockaddr)?;

            let mut recv_buffer = [MaybeUninit::<u8>::uninit(); 1024];

            match socket.recv_from(&mut recv_buffer) {
                Ok((size, addr)) => {
                    let recv_buffer: &[u8] = unsafe {
                        std::slice::from_raw_parts(
                            recv_buffer.as_ptr() as *const u8,
                            size,
                        )
                    };

                    let rtt = start.elapsed().as_secs_f64() * 1000.0;

                    let ip_header_len =
                        ((recv_buffer[0] & 0x0f) * 4) as usize;

                    if size < ip_header_len + 8 {
                        print!(" malformed ");
                        continue;
                    }

                    let icmp_packet = &recv_buffer[ip_header_len..];

                    let icmp_type = icmp_packet[0];

                    let ip = addr.as_socket().unwrap().ip();

                    match icmp_type {
                        ICMP_TIME_EXCEEDED => {
                            print!(" {} ({:.2} ms)", ip, rtt);
                        }

                        ICMP_ECHO_REPLY => {
                            print!(" {} ({:.2} ms)", ip, rtt);

                            destination_reached = true;
                        }

                        _ => {
                            print!(
                                " {} [ICMP type {}] ({:.2} ms)",
                                ip,
                                icmp_type,
                                rtt
                            );
                        }
                    }
                }

                Err(_) => {
                    print!(" *");
                }
            }

            sequence += 1;
        }

        println!();

        if destination_reached {
            break;
        }
    }

    Ok(())
}