use lettre::AsyncTransport;
use lettre::{message::Mailbox, Message};
use tiberius_models::User;

#[derive(Debug, thiserror::Error)]
pub enum TiberiusMailError {
    #[error("Mail Error: {0}")]
    Lettre(#[from] lettre::error::Error),
    #[error("SMTP Error: {0}")]
    SMTPError(#[from] lettre::transport::smtp::Error),
    #[error("Address Error: {0}")]
    AddrError(#[from] lettre::address::AddressError),
}

pub type Result<T> = std::result::Result<T, TiberiusMailError>;

#[async_trait::async_trait]
pub trait EmailService {
    async fn send_signup_welcome(&self, user: &User, register_url: &str) -> Result<()>;
    async fn send_email_reset(&self, user: &User, reset_url: &str) -> Result<()>;
    async fn send_unlock_account(&self, user: &User, unlock_url: &str) -> Result<()>;
    async fn send_update_email(&self, user: &User, update_url: &str) -> Result<()>;
}

pub struct SMTP {
    transport: lettre::AsyncSmtpTransport<lettre::Tokio1Executor>,
    from: lettre::message::Mailbox,
    reply_to: lettre::message::Mailbox,
}

impl SMTP {
    pub fn new(
        transport: lettre::AsyncSmtpTransport<lettre::Tokio1Executor>,
        from: lettre::message::Mailbox,
        reply_to: lettre::message::Mailbox,
    ) -> Self {
        Self {
            transport,
            from,
            reply_to,
        }
    }
    /// Builds a new SMTP Connection against a default MailHog instance
    #[cfg(test)]
    pub fn new_mailhog() -> Self {
        use lettre::Tokio1Executor;

        let transport =
            lettre::AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("localhost")
                .port(1025)
                .build();
        let from = lettre::message::Mailbox::new(
            Some("MailHog SMTP".to_string()),
            "noreply@example.com".parse().unwrap(),
        );
        let reply_to = lettre::message::Mailbox::new(
            Some("MailHog SMTP".to_string()),
            "noreply@example.com".parse().unwrap(),
        );
        Self::new(transport, from, reply_to)
    }
}

#[async_trait::async_trait]
impl EmailService for SMTP {
    async fn send_signup_welcome(&self, user: &User, register_url: &str) -> Result<()> {
        let body: String =
            tiberius_common_html::email::confirm_account::build(&user.name, register_url)
                .into_string();
        let body_txt: String =
            tiberius_common_html::email::confirm_account::build_txt(&user.name, register_url);
        let body = lettre::message::MultiPart::alternative_plain_html(body_txt, body);
        let email = Message::builder()
            .from(self.from.clone())
            .reply_to(self.reply_to.clone())
            .to(Mailbox::new(
                Some(user.name.clone()),
                user.email.to_string().parse()?,
            ))
            .date_now()
            .subject(tiberius_common_html::email::confirm_account::subject(
                &user.name,
            ))
            .multipart(body)?;
        self.transport.send(email).await?;
        Ok(())
    }

    async fn send_email_reset(&self, user: &User, reset_url: &str) -> Result<()> {
        let body: String =
            tiberius_common_html::email::password_reset::build(&user.name, reset_url).into_string();
        let body_txt: String =
            tiberius_common_html::email::password_reset::build_txt(&user.name, reset_url);
        let body = lettre::message::MultiPart::alternative_plain_html(body_txt, body);
        let email = Message::builder()
            .from(self.from.clone())
            .reply_to(self.reply_to.clone())
            .to(Mailbox::new(
                Some(user.name.clone()),
                user.email.to_string().parse()?,
            ))
            .date_now()
            .subject(tiberius_common_html::email::password_reset::subject(
                &user.name,
            ))
            .multipart(body)?;
        self.transport.send(email).await?;
        Ok(())
    }
    async fn send_unlock_account(&self, user: &User, unlock_url: &str) -> Result<()> {
        let body: String =
            tiberius_common_html::email::unlock_account::build(&user.name, unlock_url)
                .into_string();
        let body_txt: String =
            tiberius_common_html::email::unlock_account::build_txt(&user.name, unlock_url);
        let body = lettre::message::MultiPart::alternative_plain_html(body_txt, body);
        let email = Message::builder()
            .from(self.from.clone())
            .reply_to(self.reply_to.clone())
            .to(Mailbox::new(
                Some(user.name.clone()),
                user.email.to_string().parse()?,
            ))
            .date_now()
            .subject(tiberius_common_html::email::unlock_account::subject(
                &user.name,
            ))
            .multipart(body)?;
        self.transport.send(email).await?;
        Ok(())
    }
    async fn send_update_email(&self, user: &User, update_url: &str) -> Result<()> {
        let body: String =
            tiberius_common_html::email::update_email::build(&user.name, update_url).into_string();
        let body_txt: String =
            tiberius_common_html::email::update_email::build_txt(&user.name, update_url);
        let body = lettre::message::MultiPart::alternative_plain_html(body_txt, body);
        let email = Message::builder()
            .from(self.from.clone())
            .reply_to(self.reply_to.clone())
            .to(Mailbox::new(
                Some(user.name.clone()),
                user.email.to_string().parse()?,
            ))
            .date_now()
            .subject(tiberius_common_html::email::update_email::subject(
                &user.name,
            ))
            .multipart(body)?;
        self.transport.send(email).await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::{EmailService, Result, SMTP};

    #[tokio::test]
    pub async fn test_email_reset_delivery() -> Result<()> {
        let mail: Box<dyn EmailService> = Box::new(SMTP::new_mailhog());
        mail.send_email_reset(
            &tiberius_models::User {
                name: "Example User".to_string(),
                email: "test@example.com".to_string().into(),
                ..Default::default()
            },
            "https://example.com/reset-pw/012456869",
        )
        .await?;
        Ok(())
    }

    #[tokio::test]
    pub async fn test_email_update_email() -> Result<()> {
        let mail: Box<dyn EmailService> = Box::new(SMTP::new_mailhog());
        mail.send_update_email(
            &tiberius_models::User {
                name: "Example User".to_string(),
                email: "test@example.com".to_string().into(),
                ..Default::default()
            },
            "https://example.com/update-email/012456869",
        )
        .await?;
        Ok(())
    }

    #[tokio::test]
    pub async fn test_email_welcome() -> Result<()> {
        let mail: Box<dyn EmailService> = Box::new(SMTP::new_mailhog());
        mail.send_signup_welcome(
            &tiberius_models::User {
                name: "Example User".to_string(),
                email: "test@example.com".to_string().into(),
                ..Default::default()
            },
            "https://example.com/welcome/012456869",
        )
        .await?;
        Ok(())
    }

    #[tokio::test]
    pub async fn test_email_unlock() -> Result<()> {
        let mail: Box<dyn EmailService> = Box::new(SMTP::new_mailhog());
        mail.send_unlock_account(
            &tiberius_models::User {
                name: "Example User".to_string(),
                email: "test@example.com".to_string().into(),
                ..Default::default()
            },
            "https://example.com/unlock/012456869",
        )
        .await?;
        Ok(())
    }
}
