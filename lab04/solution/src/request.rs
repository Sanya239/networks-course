use std::collections::HashMap;

pub struct RequestParser {
    pub(crate) content_length: usize,
}

impl MessageParser for RequestParser {
    fn parse(&mut self, buf: &[u8]) -> Result<Option<usize>, httparse::Error> {
        let mut headers = [httparse::EMPTY_HEADER; 32];
        let mut req = httparse::Request::new(&mut headers);

        match req.parse(buf)? {
            httparse::Status::Complete(n) => {
                self.content_length = get_content_length(&req.headers);
                Ok(Some(n))
            }
            httparse::Status::Partial => Ok(None),
        }
    }

    fn content_length(&self) -> usize {
        self.content_length
    }
}

pub struct HttpRequest {
    method: String,
    pub(crate) path: String,
    // version: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

use crate::message::{get_content_length, MessageParser};
use crate::response::HttpResponse;
use crate::stream::read_message;
use std::io;
use anyhow::Context;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

impl HttpRequest {
    pub fn from_bytes(header_len: usize, buf: Vec<u8>) -> anyhow::Result<Self> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        req.parse(&buf)?;

        let method = req
            .method
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no method"))?
            .to_string();

        let path = req
            .path
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no path"))?
            .to_string();

        let _version = req
            .version
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no path"))?
            .to_string();

        let mut owned_headers: HashMap<String, String> = HashMap::new();

        for h in req.headers.iter() {
            let name = h.name.to_string().to_lowercase();

            let value = std::str::from_utf8(h.value)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid header"))?
                .to_string();

            owned_headers.insert(name, value);
        }

        let content_length = match owned_headers.get("content-length") {
            None => 0usize,
            Some(s) => s.parse::<usize>()?,
        };

        let body = buf
            .get(header_len..header_len + content_length)
            .unwrap_or(&[])
            .to_vec();

        Ok(Self {
            method,
            path,
            // version,
            headers: owned_headers,
            body,
        })
    }

    pub fn truncate_adress(&mut self) -> anyhow::Result<String> {
        let tmp = self.path.clone();
        let splited = tmp.split_once('/');
        if splited.is_none() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("failed to split path {}", tmp)).into());
        }
        self.path = splited.clone().unwrap().1.to_owned();
        self.headers.insert("host".parse()?, splited.clone().unwrap().0.to_owned());
        println!("path after redirection: {}", self.path);
        Ok(splited.unwrap().0.to_owned())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = vec![];
        buf.extend_from_slice(format!("{} /{} HTTP/1.1\r\n", self.method, self.path).as_bytes());
        for (k, v) in &self.headers {
            buf.extend_from_slice(format!("{}: {}\r\n", k, v).as_bytes());
        }

        buf.extend_from_slice("\r\n".as_bytes());

        buf.extend(self.body.as_slice());
        buf
    }

    pub async fn parse(mut client_stream: &mut TcpStream) -> anyhow::Result<Self> {
        let (header_len, data) = read_message(
            &mut client_stream,
            &mut RequestParser {
                content_length: 0usize,
            },
        )
        .await?;

        let mut request = HttpRequest::from_bytes(header_len, data)?;
        let _ = request.truncate_adress();
        Ok(request)
    }

    pub async fn send(&mut self) -> anyhow::Result<HttpResponse> {
        let adress = self.truncate_adress()?;
        println!("adress: {}", adress);
        let stream = TcpStream::connect(adress.clone()).await;
        println!("adress {}", adress);
        if stream.is_err() {
            return Err(stream.err().unwrap().into());
        }
        let mut stream = stream.unwrap();
        println!("\naaaa\n{}", String::from_utf8(self.to_bytes()).unwrap());
        stream.write_all(self.to_bytes().as_slice()).await?;
        stream.flush().await?;

        HttpResponse::parse(&mut stream).await
    }
}
