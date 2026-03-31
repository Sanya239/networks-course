use crate::message::MessageParser;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

async fn read_headers(
    stream: &mut TcpStream,
    parser: &mut dyn MessageParser,
) -> std::io::Result<(usize, Vec<u8>)> {
    let mut buf = Vec::new();
    loop {
        let mut temp = [0u8; 1024];
        let res = stream.read(&mut temp).await;
        if res.is_err() {
            return Err(res.err().unwrap());
        }
        let n = res.unwrap();
        if n == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }

        buf.extend_from_slice(&temp[..n]);
        println!("{}", String::from_utf8_lossy(buf.to_vec().as_slice()));
        match parser.parse(&buf) {
            Err(err) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                err.to_string(),
            ))?,
            Ok(None) => {
                continue;
            }
            Ok(Some(header_len)) => {
                return Ok((header_len, buf));
            }
        }
    }
}

pub async fn read_message(
    stream: &mut TcpStream,
    parser: &mut dyn MessageParser,
) -> anyhow::Result<(usize, Vec<u8>)> {
    let (header_len, mut buf) = read_headers(stream, parser).await?;

    parser.parse(&buf)?; // TODO

    while buf.len() < parser.content_length() + header_len {
        let mut temp = [0u8; 1024];
        let n = stream.read(&mut temp).await?;
        buf.extend_from_slice(&temp[..n]);
    }
    Ok((header_len, buf))
}
