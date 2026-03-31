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
}

pub fn blocked_response(url: &str) -> HttpResponse {
    let body = format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>Access Blocked</title>
<style>
body {{
    font-family: sans-serif;
    background: #f6f7f9;
    display: flex;
    height: 100vh;
    align-items: center;
    justify-content: center;
}}
.card {{
    background: white;
    padding: 40px;
    border-radius: 12px;
    box-shadow: 0 10px 25px rgba(0,0,0,0.1);
    max-width: 600px;
}}
h1 {{ color: #d32f2f; }}
code {{
    background: #eee;
    padding: 3px 6px;
    border-radius: 4px;
}}
</style>
</head>
<body>
<div class="card">
<h1> Request Blocked</h1>
<p>This request was blocked by the proxy server.</p>
<p>URL:</p>
<p><code>{}</code></p>
</div>
</body>
</html>"#,
        url
    );

    let mut headers = HashMap::new();

    headers.insert("Content-Type".into(), "text/html; charset=utf-8".into());

    headers.insert("Content-Length".into(), body.as_bytes().len().to_string());

    headers.insert("Connection".into(), "close".into());

    headers.insert("Server".into(), "SanyaProxy/1.0".into());

    HttpResponse {
        version: Some("HTTP/1.1".into()),
        code: Some("403".into()),
        reason: Some("Forbidden".into()),
        headers,
        body: body.into_bytes(),
    }
}
