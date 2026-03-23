use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: client <server_ip> <port> <filename>");
        return;
    }

    let server_ip = &args[1];
    let port = &args[2];
    let filename = &args[3];

    let addr = format!("{}:{}", server_ip, port);
    let mut stream = TcpStream::connect(&addr).expect(&format!("Could not connect to {}", addr));

    let request = format!(
        "GET /{} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        filename, server_ip
    );

    stream
        .write_all(request.as_bytes())
        .expect("Failed to send request");

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("Failed to read response");

    println!("{}", response);
}
