use pnet_packet::icmp::{
    destination_unreachable::DestinationUnreachablePacket,
    echo_reply::EchoReplyPacket,
    echo_request::MutableEchoRequestPacket,
    time_exceeded::TimeExceededPacket,
    IcmpCode,
    IcmpPacket,
    IcmpTypes,
};
use pnet_packet::ip::IpNextHeaderProtocols;
use pnet_packet::MutablePacket;
use pnet_packet::Packet;

use socket2::{Domain, Protocol, SockAddr, Socket, Type};

use std::env;
use std::mem::MaybeUninit;
use std::net::{SocketAddr, ToSocketAddrs};
use std::process;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::{Duration, Instant};

const ICMP_PACKET_SIZE: usize = 64;

fn checksum(packet: &[u8]) -> u16 {
    let mut sum = 0u32;

    let mut chunks = packet.chunks_exact(2);

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

fn resolve_host(host: &str) -> SocketAddr {
    let addr = format!("{}:0", host);

    addr.to_socket_addrs()
        .unwrap_or_else(|_| {
            eprintln!("Cannot resolve host");
            process::exit(1);
        })
        .find(|a| a.is_ipv4())
        .unwrap_or_else(|| {
            eprintln!("No IPv4 address found");
            process::exit(1);
        })
}

fn icmp_error_description(icmp_type: u8, code: u8) -> String {
    match icmp_type {
        3 => match code {
            0 => "Destination network unreachable".to_string(),
            1 => "Destination host unreachable".to_string(),
            2 => "Destination protocol unreachable".to_string(),
            3 => "Destination port unreachable".to_string(),
            _ => format!("Destination unreachable (code {})", code),
        },
        11 => match code {
            0 => "TTL expired".to_string(),
            1 => "Fragment reassembly time exceeded".to_string(),
            _ => format!("Time exceeded (code {})", code),
        },
        _ => format!("ICMP error type={}, code={}", icmp_type, code),
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cargo run -- <host>");
        process::exit(1);
    }

    let host = &args[1];
    let target = resolve_host(host);

    println!("PING {} ({})", host, target.ip());

    let socket = Socket::new(
        Domain::IPV4,
        Type::RAW,
        Some(Protocol::from(IpNextHeaderProtocols::Icmp.0 as i32)),
    )?;

    socket.set_read_timeout(Some(Duration::from_secs(1)))?;

    let sockaddr = SockAddr::from(target);

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    ctrlc::set_handler(move || {
        running_clone.store(false, Ordering::SeqCst);
    })
        .expect("Error setting Ctrl-C handler");

    let mut seq: u16 = 1;

    let mut sent = 0u32;
    let mut received = 0u32;

    let mut rtts: Vec<f64> = Vec::new();

    while running.load(Ordering::SeqCst) {
        let mut buffer = [0u8; ICMP_PACKET_SIZE];

        let mut packet = MutableEchoRequestPacket::new(&mut buffer).unwrap();

        packet.set_icmp_type(IcmpTypes::EchoRequest);
        packet.set_sequence_number(seq);
        packet.set_identifier(process::id() as u16);

        let now = Instant::now();

        let timestamp = now.elapsed().as_nanos().to_be_bytes();

        let payload_len = timestamp.len().min(packet.payload().len());

        packet.payload_mut()[..payload_len]
            .copy_from_slice(&timestamp[..payload_len]);

        packet.set_checksum(0);

        let checksum = checksum(packet.packet());

        packet.set_checksum(checksum);

        socket.send_to(packet.packet(), &sockaddr)?;

        sent += 1;

        let mut recv_buffer = [MaybeUninit::<u8>::uninit(); 1024];

        match socket.recv_from(&mut recv_buffer) {
            Ok((size, addr)) => {
                let recv_buffer: &[u8] = unsafe {
                    std::slice::from_raw_parts(
                        recv_buffer.as_ptr() as *const u8,
                        size,
                    )
                };

                let ip_header_len = (recv_buffer[0] & 0x0f) * 4;

                let icmp_slice = &recv_buffer[ip_header_len as usize..size];

                if let Some(icmp_packet) = IcmpPacket::new(icmp_slice) {
                    match icmp_packet.get_icmp_type() {
                        IcmpTypes::EchoReply => {
                            if let Some(reply) = EchoReplyPacket::new(icmp_slice) {
                                let rtt = now.elapsed().as_secs_f64() * 1000.0;

                                received += 1;
                                rtts.push(rtt);

                                let min = rtts.iter().cloned().fold(f64::INFINITY, f64::min);

                                let max = rtts
                                    .iter()
                                    .cloned()
                                    .fold(f64::NEG_INFINITY, f64::max);

                                let avg =
                                    rtts.iter().sum::<f64>() / rtts.len() as f64;

                                println!(
                                    "{} bytes from {}: icmp_seq={} time={:.2} ms",
                                    size,
                                    addr.as_socket().unwrap().ip(),
                                    reply.get_sequence_number(),
                                    rtt
                                );

                                println!(
                                    "RTT min/avg/max = {:.2}/{:.2}/{:.2} ms",
                                    min, avg, max
                                );
                            }
                        }

                        IcmpTypes::DestinationUnreachable => {
                            if let Some(packet) =
                                DestinationUnreachablePacket::new(icmp_slice)
                            {
                                let code: IcmpCode = packet.get_icmp_code();

                                println!(
                                    "ICMP error: {}",
                                    icmp_error_description(3, code.0)
                                );
                            }
                        }

                        IcmpTypes::TimeExceeded => {
                            if let Some(packet) =
                                TimeExceededPacket::new(icmp_slice)
                            {
                                let code = packet.get_icmp_code();

                                println!(
                                    "ICMP error: {}",
                                    icmp_error_description(11, code.0)
                                );
                            }
                        }

                        other => {
                            println!("Received ICMP packet: {:?}", other);
                        }
                    }
                }
            }

            Err(_) => {
                println!("Request timeout for icmp_seq={}", seq);
            }
        }

        seq += 1;

        thread::sleep(Duration::from_secs(1));
    }

    println!("\n--- {} ping statistics ---", host);

    let loss = if sent == 0 {
        0.0
    } else {
        ((sent - received) as f64 / sent as f64) * 100.0
    };

    println!(
        "{} packets transmitted, {} packets received, {:.1}% packet loss",
        sent,
        received,
        loss
    );

    if !rtts.is_empty() {
        let min = rtts.iter().cloned().fold(f64::INFINITY, f64::min);

        let max = rtts
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        let avg = rtts.iter().sum::<f64>() / rtts.len() as f64;

        println!(
            "round-trip min/avg/max = {:.2}/{:.2}/{:.2} ms",
            min, avg, max
        );
    }

    Ok(())
}