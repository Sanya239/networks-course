use crate::message::{get_content_length, MessageParser};
use crate::stream::read_message;
use httparse::{Error, Response, Status, EMPTY_HEADER};
use std::collections::HashMap;
use tokio::net::TcpStream;

pub struct ResponseParser {
    pub(crate) content_length: usize,
}

impl MessageParser for ResponseParser {
    fn parse(&mut self, buf: &[u8]) -> Result<Option<usize>, Error> {
        let mut headers = [EMPTY_HEADER; 32];
        let mut resp = Response::new(&mut headers);

        match resp.parse(buf)? {
            Status::Complete(n) => {
                self.content_length = get_content_length(&resp.headers);

                Ok(Some(n))
            }
            Status::Partial => Ok(None),
        }
    }

    fn content_length(&self) -> usize {
        self.content_length
    }
}

pub struct HttpResponse {
    version: Option<String>,
    code: Option<String>,
    reason: Option<String>,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl HttpResponse {
    pub fn from_bytes(header_len: usize, buf: Vec<u8>) -> anyhow::Result<Self> {
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut resp = httparse::Response::new(&mut headers);
        resp.parse(&buf)?;
        let version = resp.version.map(|x| x.to_string());

        let code = resp.code.map(|x| x.to_string());

        let reason = resp.reason.map(|x| x.to_string());

        let mut owned_headers: HashMap<String, String> = HashMap::new();

        for h in resp.headers.iter() {
            let name = h.name.to_string().to_lowercase();

            let value = std::str::from_utf8(h.value)?.to_string();

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
            version,
            code,
            reason,
            headers: owned_headers,
            body,
        })
    }

    pub fn from_body_ok(body: Vec<u8>) -> Self {
        let mut headers = HashMap::new();

        headers.insert("Content-Type".into(), "text/html; charset=utf-8".into());

        headers.insert("Content-Length".into(), body.len().to_string());

        headers.insert("Connection".into(), "close".into());

        HttpResponse {
            version: Some("HTTP/1.1".into()),
            code: Some("200".into()),
            reason: None,
            headers,
            body,
        }
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        self.version.as_ref().map(|_| {
            buf.extend_from_slice(format!("HTTP/{} ", 1.1).as_bytes());
        });

        self.code.as_ref().map(|x| {
            buf.extend_from_slice(format!("{} ", x).as_bytes());
        });

        self.reason.as_ref().map(|x| {
            buf.extend_from_slice(format!("{}", x).as_bytes());
        });
        buf.extend_from_slice(b"\r\n");

        for (k, v) in &self.headers {
            buf.extend_from_slice(format!("{}: {}\r\n", k, v).as_bytes());
        }
        buf.extend_from_slice(b"\r\n");
        buf.extend_from_slice(&self.body);
        buf
    }

    pub async fn parse(mut client_stream: &mut TcpStream) -> anyhow::Result<Self> {
        let (header_len, data) = read_message(
            &mut client_stream,
            &mut ResponseParser {
                content_length: 0usize,
            },
        )
        .await?;

        let response = HttpResponse::from_bytes(header_len, data)?;
        Ok(response)
    }

    pub fn get_body(&self) -> &Vec<u8> {
        &self.body
    }
}
