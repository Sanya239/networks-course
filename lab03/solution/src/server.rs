mod client;

use std::fs::File;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Condvar, Mutex};
use std::{fs, thread};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let port = args
        .get(1)
        .expect("Usage: Cargo run <port> <max_threads>")
        .parse::<u16>()
        .expect("Port must be a number");

    let max_connections = args
        .get(2)
        .expect("Usage: Cargo run <port> <max_threads>")
        .parse::<u16>()
        .expect("Number of connections must be a number");
    let listener = TcpListener::bind(format!("localhost:{}", port)).expect("Could not bind");

    println!("Server listening on localhost:{}", port);

    let active_connections = Arc::new((Mutex::new(0), Condvar::new()));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let active_connections = Arc::clone(&active_connections);

                {
                    let (lock, cvar) = &*active_connections;
                    let mut count = lock.lock().unwrap();
                    while *count >= max_connections {
                        count = cvar.wait(count).unwrap();
                    }
                    *count += 1;
                }

                thread::spawn(move || {
                    handle_client(stream);

                    let (lock, cvar) = &*active_connections;
                    let mut count = lock.lock().unwrap();
                    *count -= 1;
                    cvar.notify_one();
                });
            }
            Err(e) => {
                eprintln!("Failed to accept connection: {}", e);
            }
        }
    }
}

fn handle_client(mut stream: TcpStream) {
    println!("Incoming connection from: {}", stream.peer_addr().unwrap());
    println!("Our port is: {}", stream.local_addr().unwrap());
    let mut request = String::new();
    let mut buff = [0; 1024];
    loop {
        let bytes_read = stream.read(&mut buff);
        match bytes_read {
            Ok(bytes_read) => {
                request.push_str(&String::from_utf8_lossy(&buff[0..bytes_read]));
                if request.contains("\r\n\r\n") {
                    let line = &request[..request.find("\r\n").unwrap()];
                    handle_request(stream, line);
                    return;
                }
            }
            Err(e) => {
                eprintln!("Failed: {}", e);
            }
        }
    }
}

fn handle_request(mut stream: TcpStream, request: &str) {
    if !request.starts_with("GET") {
        stream
            .write_all("HTTP/1.1 405 Method not allowed".as_bytes())
            .unwrap();
        eprintln!("Requested method is not allowed");
        return;
    }
    let filename = request.split_whitespace().nth(1).unwrap();
    let filename = filename.strip_prefix('/').unwrap_or(filename);

    let file = File::open(filename);
    if file.is_err() {
        stream
            .write_all("HTTP/1.1 404 Not found\r\n".as_bytes())
            .unwrap();
        let paths = fs::read_dir("./").unwrap();
        let mut names = String::new();
        names.push_str("Available files\n");
        for path in paths {
            names += path.unwrap().path().to_str().unwrap();
            names.push_str("\r\n");
        }
        stream
            .write_all(format!("Content-Length: {}\r\n\r\n", names.len()).as_bytes())
            .unwrap();
        stream.write_all(names.as_bytes()).unwrap();
        eprintln!("File not found: {}", filename);
        return;
    }
    let mut file = file.unwrap();
    stream.write_all("HTTP/1.1 200 OK\r\n".as_bytes()).unwrap();
    stream
        .write_all(format!("Content-Length: {}\r\n\r\n", file.metadata().unwrap().len()).as_bytes())
        .unwrap();
    match std::io::copy(&mut file, &mut stream) {
        Ok(_) => {
            println!("Successfully transferred the file");
        }
        Err(e) => {
            eprintln!("Failed to transfer the file: {}", e);
        }
    }
}
