use std::fs;
use std::io::{self, Write};
use anyhow::Result;
use lettre::Message;
use lettre::message::{Attachment, MultiPart, SinglePart};
use mime::TEXT_PLAIN;

#[derive(Debug)]
pub struct Email {
    pub from: String,
    pub to: String,
    pub subject: String,
    pub useHTML: bool,
    pub body: String,
    pub attachments: Vec<String>,
}

fn prompt(message: &str) -> Result<String> {
    print!("{message}: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}

fn read_body() -> Result<String> {
    println!("Please enter mail body (blank line to stop)");

    let mut body = String::new();
    
    loop {
        let line = prompt("line>")?;
        let line = line.trim();
        if line.is_empty() {
            break;
        }
        body.push_str(&line);
        body.push('\n');
    }
    Ok(body)
}

fn read_attachments() -> Result<Vec<String>> {
    println!("Please enter mail attachments (blank line to stop):");

    let mut attachments = Vec::new();

    loop {
        print!("attachment> "); // TODO use "prompt"
        io::stdout().flush()?;

        let mut line = String::new();
        io::stdin().read_line(&mut line)?;

        let line = line.trim();

        if line.is_empty() {
            break;
        }

        attachments.push(line.to_string());
    }

    Ok(attachments)
}

pub fn read_email() -> Result<Email> {
    let from = prompt("Please enter the senders email")?;
    let to = prompt("Please enter the recipients email")?;
    let subject = prompt("Please enter the mail subject")?;
    let useHTML = prompt("Use HTML format [y/N]")?=="y";
    
    let body = read_body()?;

    let attachments = read_attachments()?;

    Ok(Email {
        from,
        to,
        subject,
        useHTML,
        body,
        attachments,
    })
}

impl Email {
    pub fn build_message(&self) -> Result<Message> {
        let mut body_part = SinglePart::plain(self.body.clone());
        if self.useHTML{
            body_part = SinglePart::html(self.body.clone());
        }

        let mut multipart = MultiPart::mixed().singlepart(body_part);

        for path in self.attachments.clone() {
            let bytes = fs::read(&path)?;

            let filename = std::path::Path::new(&path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();

            let attachment = Attachment::new(filename)
                .body(bytes, lettre::message::header::ContentType::TEXT_PLAIN);

            multipart = multipart.singlepart(attachment);
        }

        let message = Message::builder()
            .from(self.from.parse()?)
            .to(self.to.parse()?)
            .subject(self.subject.clone())
            .multipart(multipart)?;

        Ok(message)
    }
}