mod client_handler;
mod message;
mod request;
mod response;
mod stream;
mod cmd;
use crate::client_handler::handle_client;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let port = args
        .get(1)
        .expect("Usage: Cargo run <port>")
        .parse::<u16>()
        .expect("Port must be a number");

    let listener = TcpListener::bind(format!("localhost:{port}"))
        .await
        .expect(format!("Could not bind to port {port}").as_str());

    loop {
        let connection = listener.accept().await;
        if connection.is_err() {
            println!("couldn't get client: {:?}", connection.err().unwrap());
            continue;
        }
        let (stream, _) = connection?;
        tokio::spawn(async move { handle_client(stream).await });
    }
}
