use crate::email::Email;
use crate::mail_sender::MailSender;
use anyhow::Result;
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};

pub struct LettreSender {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
}

impl LettreSender {
    pub fn new() -> Self {
        Self {
            mailer: AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("localhost")
                .port(1025)
                .build(),
        }
    }
}

#[async_trait::async_trait]
impl MailSender for LettreSender {
    async fn send(&self, email: Email) -> Result<()> {
        self.mailer.send(email.build_message()?).await?;
        Ok(())
    }
}
