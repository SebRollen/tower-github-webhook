use crate::future::ValidateGitHubWebhookFuture;
use hmac::{Hmac, Mac};
use http::{Request, Response};
use http_body::Body;
use sha2::Sha256;
use std::task::{Context, Poll};
use tower::Service;

/// Middleware that authorizes all requests using the X-Hub-Signature-256 header.
#[derive(Clone)]
pub struct ValidateGitHubWebhook<S> {
    inner: S,
    hmac: Hmac<Sha256>,
}

impl<S> ValidateGitHubWebhook<S> {
    pub fn new(webhook_secret: impl AsRef<[u8]>, inner: S) -> Self {
        let hmac = Hmac::<Sha256>::new_from_slice(webhook_secret.as_ref())
            .expect("Failed to parse webhook_secret");
        Self { inner, hmac }
    }
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for ValidateGitHubWebhook<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone,
    ReqBody: Body + Unpin,
    ResBody: Body + Default,
{
    type Response = Response<ResBody>;
    type Error = S::Error;
    type Future = ValidateGitHubWebhookFuture<S, ReqBody, ResBody>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let inner = self.inner.clone();
        let hmac = self.hmac.clone();
        ValidateGitHubWebhookFuture::new(req, hmac, inner)
    }
}
