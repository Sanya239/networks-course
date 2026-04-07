mod message;
mod request;
mod response;
mod stream;

use crate::request::HttpRequest;
use std::io::Write;
use std::net::TcpStream;
use std::str::from_utf8;
use std::{env, io};

fn prompt(message: &str) -> anyhow::Result<String> {
    print!("{message}: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        return Err(anyhow::anyhow!("Usage: client <port>"));
    }

    let port = &args[1];
    let address = format!("localhost:{}", port);
    println!("Connecting to {}", address);
    loop {
        println!("Please enter command to run on server ");
        println!("\t(empty string to close program)");
        let cmd = prompt(">")?;
        if cmd.len() == 0 {
            break;
        }
        let mut stream = TcpStream::connect(&address)?;
        let mut addr = address.clone();
        addr.push('/');
        let mut request = HttpRequest::from_body_get(addr, cmd.into_bytes());

        let response = request.send().await?;

        println!("Response:");
        println!("{}", from_utf8(response.get_body())?);
    }
    Ok(())
}
