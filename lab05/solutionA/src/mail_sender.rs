use crate::email::Email;
use anyhow::Result;
#[async_trait::async_trait]
pub trait MailSender: Send + Sync {
    async fn send(&self, email: Email) -> Result<()>;
}
