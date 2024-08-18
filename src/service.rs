use crate::future::Future;
use bytes::Bytes;
use hmac::{Hmac, Mac};
use http::{Request, Response};
use http_body::Body;
use http_body_util::{Either, Empty, Full};
use sha2::Sha256;
use std::task::{Context, Poll};
use tower_service::Service;

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
    S: Service<Request<Full<Bytes>>, Response = Response<ResBody>> + Clone,
    ReqBody: Body,
{
    type Response = Response<Either<ResBody, Empty<Bytes>>>;
    type Error = S::Error;
    type Future = Future<S, ReqBody, ResBody>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let inner = self.inner.clone();
        let hmac = self.hmac.clone();
        Future::new(req, hmac, inner)
    }
}
