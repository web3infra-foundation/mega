use async_trait::async_trait;
use common::{config::MailConfig, errors::MegaError};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{MultiPart, SinglePart, header::ContentType},
    transport::smtp::authentication::Credentials,
};

#[async_trait]
pub trait Mailer: Send + Sync {
    async fn send_html(
        &self,
        to: &str,
        subject: &str,
        html: &str,
        text: Option<&str>,
    ) -> Result<(), MegaError>;
}

pub struct NoopMailer;

#[async_trait]
impl Mailer for NoopMailer {
    async fn send_html(
        &self,
        _to: &str,
        _subject: &str,
        _html: &str,
        _text: Option<&str>,
    ) -> Result<(), MegaError> {
        Ok(())
    }
}

pub struct SmtpMailer {
    enabled: bool,
    from: String,
    transport: Option<AsyncSmtpTransport<Tokio1Executor>>,
}

impl SmtpMailer {
    pub fn new(cfg: &MailConfig) -> Result<Self, MegaError> {
        if !cfg.enabled {
            return Ok(Self {
                enabled: false,
                from: cfg.from.clone(),
                transport: None,
            });
        }

        let mut builder = if cfg.starttls {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.smtp_host)
                .map_err(|e| MegaError::Other(format!("smtp starttls relay error: {e}")))?
                .port(cfg.smtp_port)
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.smtp_host)
                .port(cfg.smtp_port)
        };

        if let (Some(user), Some(pass)) = (&cfg.username, &cfg.password) {
            builder = builder.credentials(Credentials::new(user.clone(), pass.clone()));
        }

        Ok(Self {
            enabled: true,
            from: cfg.from.clone(),
            transport: Some(builder.build()),
        })
    }

    fn build_message(
        &self,
        to: &str,
        subject: &str,
        html: &str,
        text: Option<&str>,
    ) -> Result<Message, MegaError> {
        let from = self
            .from
            .parse()
            .map_err(|e| MegaError::Other(format!("invalid from email: {e}")))?;
        let to = to
            .parse()
            .map_err(|e| MegaError::Other(format!("invalid to email: {e}")))?;

        let html_part = SinglePart::builder()
            .header(ContentType::TEXT_HTML)
            .body(html.to_string());

        let multipart = if let Some(text) = text {
            let text_part = SinglePart::builder()
                .header(ContentType::TEXT_PLAIN)
                .body(text.to_string());
            MultiPart::alternative()
                .singlepart(text_part)
                .singlepart(html_part)
        } else {
            MultiPart::alternative().singlepart(html_part)
        };

        Message::builder()
            .from(from)
            .to(to)
            .subject(subject)
            .multipart(multipart)
            .map_err(|e| MegaError::Other(format!("build email message error: {e}")))
    }
}

#[async_trait]
impl Mailer for SmtpMailer {
    async fn send_html(
        &self,
        to: &str,
        subject: &str,
        html: &str,
        text: Option<&str>,
    ) -> Result<(), MegaError> {
        if !self.enabled {
            return Ok(());
        }
        let transport = self
            .transport
            .as_ref()
            .ok_or_else(|| MegaError::Other("smtp transport missing while enabled".to_string()))?;

        let msg = self.build_message(to, subject, html, text)?;
        transport
            .send(msg)
            .await
            .map(|_| ())
            .map_err(|e| MegaError::Other(format!("smtp send error: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use common::config::MailConfig;

    use super::*;

    #[test]
    fn test_smtp_mailer_disabled_is_noop() {
        let cfg = MailConfig {
            enabled: false,
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: 587,
            username: None,
            password: None,
            from: "no-reply@example.com".to_string(),
            starttls: true,
        };

        let mailer = SmtpMailer::new(&cfg).expect("create mailer");
        assert!(!mailer.enabled);
    }

    #[test]
    fn test_build_message_validates_addresses() {
        let cfg = MailConfig {
            enabled: false,
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: 587,
            username: None,
            password: None,
            from: "no-reply@example.com".to_string(),
            starttls: true,
        };
        let mailer = SmtpMailer::new(&cfg).unwrap();

        let msg = mailer
            .build_message("user@example.com", "Subj", "<p>Hi</p>", Some("Hi"))
            .expect("message should build");

        let raw = msg.formatted();
        assert!(!raw.is_empty());
    }

    #[test]
    fn test_build_message_rejects_bad_to() {
        let cfg = MailConfig {
            enabled: false,
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: 587,
            username: None,
            password: None,
            from: "no-reply@example.com".to_string(),
            starttls: true,
        };
        let mailer = SmtpMailer::new(&cfg).unwrap();
        let err = mailer
            .build_message("not-an-email", "Subj", "<p>Hi</p>", None)
            .expect_err("should fail");
        let _ = format!("{err:?}");
    }
}
