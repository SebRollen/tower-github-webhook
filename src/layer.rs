use crate::ValidateGitHubWebhook;
use tower::Layer;

/// Layer that applies the [ValidateGitHubWebhook] middleware which authorizes all requests using
/// the `X-Hub-Signature-256` header.
#[derive(Clone)]
pub struct ValidateGitHubWebhookLayer<Secret> {
    webhook_secret: Secret,
}

impl<Secret> ValidateGitHubWebhookLayer<Secret> {
    /// Authorize requests using the `X-Hub-Signature-256` header. If the signature specified in
    /// that header is not signed using the `webhook_secret` secret, the request will fail.
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
