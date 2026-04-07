mod email;
mod lettre_sender;
mod mail_sender;
mod simple_sender;

use crate::email::read_email;
use crate::lettre_sender::LettreSender;
use crate::mail_sender::MailSender;
use crate::simple_sender::SimpleSender;
use lettre::AsyncTransport;
use log::{error, info};

fn create_sender(use_lettre: bool) -> Box<dyn MailSender> {
    if use_lettre {
        return Box::new(LettreSender::new());
    }
    Box::new(SimpleSender::new())
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    info!("program started");

    let args: Vec<String> = std::env::args().collect();
    let use_lettre =
        args.len() >= 2 && (args.get(1).expect("Usage: Cargo run [use_lettre]") == "use_lettre");
    if use_lettre {
        info!("starting with lettre sender");
    } else {
        info!("starting with simple sender");
    }
    
    let email = read_email()?;
    info!("successfully read email");
    
    let sender = create_sender(use_lettre);
    sender.send(email).await?;
    info!("successfully sent email");
    
    Ok(())
}
