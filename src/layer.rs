use crate::ValidateGitHubWebhook;
use tower_layer::Layer;

/// Layer that applies the [ValidateGitHubWebhook] middleware which authorizes all requests using
/// the `X-Hub-Signature-256` header.
#[derive(Clone)]
pub struct ValidateGitHubWebhookLayer<Secret> {
    webhook_secret: Secret,
}

impl<Secret> ValidateGitHubWebhookLayer<Secret> {
    /// Authorize requests using the `X-Hub-Signature-256` header. If the signature specified in
    /// that header is not signed using the `webhook_secret` secret, the request will fail.
    ///
    /// The `webhook_secret` parameter can be any type that implements `AsRef<[u8]>` such as
    /// `String`. However, using `secrecy::SecretString` is recommended to prevent the secret from
    /// being printed in any logs.
    pub fn new(webhook_secret: Secret) -> Self {
        Self { webhook_secret }
    }
}

impl<S, Secret> Layer<S> for ValidateGitHubWebhookLayer<Secret>
where
    Secret: AsRef<[u8]> + Clone,
{
    type Service = ValidateGitHubWebhook<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ValidateGitHubWebhook::new(self.webhook_secret.clone(), inner)
    }
}
