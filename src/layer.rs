use crate::ValidateGitHubWebhook;
use tower::Layer;

#[derive(Clone)]
pub struct ValidateGitHubWebhookLayer<Secret> {
    webhook_secret: Secret,
}

impl<Secret> ValidateGitHubWebhookLayer<Secret> {
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
