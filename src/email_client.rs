use std::time::Duration;

use secrecy::{ExposeSecret, SecretString};

use crate::domain::SubscriberEmail;

pub struct EmailCient {
    client: reqwest::Client,
    base_url: reqwest::Url,
    sender: SubscriberEmail,
    authorization_token: SecretString,
}

impl EmailCient {
    fn new(
        base_url: &str,
        sender: SubscriberEmail,
        timeout: Duration,
        authorization_token: SecretString,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("failed to build email client.");
        let base_url = reqwest::Url::parse(base_url).expect("failed to parse base url.");

        Self {
            client,
            base_url,
            sender,
            authorization_token,
        }
    }

    pub fn from_config(config: &crate::config::Config) -> Self {
        let email_client_config = &config.email_client;
        let sender = SubscriberEmail::parse(&email_client_config.sender).unwrap();
        let timeout = Duration::from_millis(email_client_config.timeout_milliseconds);
        let authorization_token = email_client_config.authorization_token.clone();

        Self::new(
            &email_client_config.base_url,
            sender,
            timeout,
            authorization_token,
        )
    }

    #[tracing::instrument(name = "sending email", skip_all)]
    pub async fn send(
        &self,
        receiver: &SubscriberEmail,
        subject: &str,
        text_body: &str,
        html_body: &str,
    ) -> reqwest::Result<()> {
        let url = self.base_url.join("/email").unwrap();
        let body = EmailRequestBody {
            from: self.sender.as_ref(),
            to: receiver.as_ref(),
            subject,
            text_body,
            html_body,
        };

        self.client
            .post(url)
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct EmailRequestBody<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    text_body: &'a str,
    html_body: &'a str,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use claim::{assert_err, assert_ok};
    use fake::{
        faker::{
            internet::en::SafeEmail,
            lorem::zh_cn::{Paragraph, Sentence},
        },
        Fake,
    };
    use secrecy::SecretString;
    use wiremock::{
        matchers::{header, header_exists, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use crate::domain::SubscriberEmail;

    use super::EmailCient;
    struct EmailRequestBodyMatcher;

    impl wiremock::Match for EmailRequestBodyMatcher {
        fn matches(&self, request: &wiremock::Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                return body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("TextBody").is_some()
                    && body.get("HtmlBody").is_some();
            }
            false
        }
    }

    async fn mock_send_helper(mock_response: ResponseTemplate) -> Result<(), reqwest::Error> {
        let mock = MockServer::start().await;
        let sender: String = SafeEmail().fake();
        let email_client = EmailCient::new(
            mock.uri().as_str(),
            SubscriberEmail::parse(&sender).unwrap(),
            Duration::from_millis(200),
            SecretString::new("my-secret-token".into()),
        );

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(EmailRequestBodyMatcher)
            .respond_with(mock_response)
            .expect(1)
            .mount(&mock)
            .await;

        let receiver: String = SafeEmail().fake();
        let receiver = SubscriberEmail::parse(&receiver).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        email_client
            .send(&receiver, &subject, &content, &content)
            .await
    }

    #[tokio::test]
    async fn mock_send_ok() {
        let mock_response = ResponseTemplate::new(200);
        let result = mock_send_helper(mock_response).await;
        assert_ok!(result);
    }

    #[tokio::test]
    async fn mock_send_400() {
        let mock_response = ResponseTemplate::new(400);
        let result = mock_send_helper(mock_response).await;
        assert_err!(result);
    }

    #[tokio::test]
    async fn mock_send_500() {
        let mock_response = ResponseTemplate::new(500);
        let result = mock_send_helper(mock_response).await;
        assert_err!(result);
    }

    #[tokio::test]
    async fn mock_send_timeout() {
        let mock_response = ResponseTemplate::new(200).set_delay(Duration::from_secs(70));
        let result = mock_send_helper(mock_response).await;
        assert_err!(result);
    }
}
