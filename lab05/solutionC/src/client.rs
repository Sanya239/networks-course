use socket2::{Domain, Protocol, Socket, Type};
use std::net::{SocketAddr, UdpSocket};
use std::{env, str};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(anyhow::anyhow!("Usage: client <port>"));
    }

    let port = &args[1];
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;

    socket.set_reuse_address(true)?;

    let addr: SocketAddr = format!("0.0.0.0:{port}").parse().unwrap();
    socket.bind(&addr.into())?;
    let socket: UdpSocket = socket.into();
    println!("UDP time client listening on port {}", port);

    let mut buf = [0u8; 1024];

    loop {
        let (size, sender) = socket.recv_from(&mut buf)?;

        let msg = str::from_utf8(&buf[..size]).unwrap_or("<invalid utf8>");

        println!("{} -> {}", sender, msg);
    }
}
