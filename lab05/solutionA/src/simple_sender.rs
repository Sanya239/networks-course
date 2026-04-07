use crate::email::Email;
use crate::mail_sender::MailSender;
use anyhow::{bail, Result};
use base64::{engine::general_purpose, Engine};
use log::{debug, info};
use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

pub struct SimpleSender {
    pub host: String,
    pub port: u16,
}

impl SimpleSender {
    pub fn new() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 1025,
        }
    }
}

#[async_trait::async_trait]
impl MailSender for SimpleSender {
    async fn send(&self, email: Email) -> Result<()> {
        let stream = TcpStream::connect((&*self.host, self.port)).await?;

        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);

        async fn read_response(
            reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
        ) -> Result<()> {
            let mut line = String::new();
            reader.read_line(&mut line).await?;

            if !line.starts_with('2') && !line.starts_with('3') {
                bail!("SMTP error: {}", line);
            }

            Ok(())
        }

        read_response(&mut reader).await?;

        write_half.write_all(b"HELO localhost\r\n").await?;
        read_response(&mut reader).await?;

        debug!("Handshake completed");
        write_half
            .write_all(format!("MAIL FROM:<{}>\r\n", email.from).as_bytes())
            .await?;
        read_response(&mut reader).await?;

        write_half
            .write_all(format!("RCPT TO:<{}>\r\n", email.to).as_bytes())
            .await?;
        read_response(&mut reader).await?;

        write_half.write_all(b"DATA\r\n").await?;
        read_response(&mut reader).await?;

        let boundary = "TotallyUnsuspiciousPattern239";

        let mut message = format!(
            concat!(
                "From: {from}\r\n",
                "To: {to}\r\n",
                "Subject: {subject}\r\n",
                "MIME-Version: 1.0\r\n",
                "Content-Type: multipart/mixed; boundary=\"{boundary}\"\r\n",
                "\r\n",
                "--{boundary}\r\n",
                "Content-Type: text/{type}; charset=utf-8\r\n",
                "\r\n",
                "{body}\r\n"
            ),
            from = email.from,
            to = email.to,
            subject = email.subject,
            boundary = boundary,
            type = if email.useHTML{ "html" }else {"text"},
            body = email.body
        );

        for path in email.attachments {
            let bytes = fs::read(&path).await?;
            let encoded = general_purpose::STANDARD.encode(bytes);

            let filename = std::path::Path::new(&path)
                .file_name()
                .unwrap()
                .to_string_lossy();

            message.push_str(&format!(
                concat!(
                    "\r\n--{}\r\n",
                    "Content-Type: application/octet-stream\r\n",
                    "Content-Disposition: attachment; filename=\"{}\"\r\n",
                    "Content-Transfer-Encoding: base64\r\n",
                    "\r\n",
                    "{}\r\n"
                ),
                boundary, filename, encoded
            ));
        }

        message.push_str(&format!("--{}--\r\n.\r\n", boundary));

        write_half.write_all(message.as_bytes()).await?;
        read_response(&mut reader).await?;

        write_half.write_all(b"QUIT\r\n").await?;

        Ok(())
    }
}
