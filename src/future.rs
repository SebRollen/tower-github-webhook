use bytes::Buf;
use hmac::{Hmac, Mac};
use http::{Request, Response, StatusCode};
use http_body::Body;
use pin_project::pin_project;
use sha2::Sha256;
use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use tower::Service;

#[pin_project]
pub struct ValidateGitHubWebhookFuture<
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
    ReqBody,
    ResBody,
> {
    req: Option<Request<ReqBody>>,
    signature: Option<Vec<u8>>,
    inner: S,
    hmac: Option<Hmac<Sha256>>,
    #[pin]
    state: ValidateGitHubWebhookFutureState<ReqBody, ResBody, S>,
}

impl<S, ReqBody, ResBody> ValidateGitHubWebhookFuture<S, ReqBody, ResBody>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
{
    pub fn new(req: Request<ReqBody>, hmac: Hmac<Sha256>, inner: S) -> Self {
        Self {
            req: Some(req),
            signature: None,
            inner,
            hmac: Some(hmac),
            state: ValidateGitHubWebhookFutureState::ExtractSignature,
        }
    }
}

impl<S, F, ReqBody, ResBody> Future for ValidateGitHubWebhookFuture<S, ReqBody, ResBody>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>, Future = F>,
    F: Future<Output = Result<Response<ResBody>, S::Error>>,
    ReqBody: Body + Unpin,
    ResBody: Body + Default,
{
    type Output = Result<Response<ResBody>, S::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();
        let mut curr_state = this.state;
        match curr_state.as_mut().project() {
            ValidateGitHubProj::ExtractSignature => {
                tracing::trace!(
                    "[tower-github-webhook] ValidateGitHubWebhookFutureState::ExtractSignature"
                );
                let req = this.req.take().unwrap();
                let signature = match req.headers().get("x-hub-signature-256") {
                    Some(sig) => {
                        let Some(sig) = sig.as_bytes().splitn(2, |x| x == &b'=').nth(1) else {
                            tracing::debug!("[tower-github-webhook] Invalid header format");
                            curr_state.set(ValidateGitHubWebhookFutureState::Unauthorized);
                            cx.waker().wake_by_ref();
                            return Poll::Pending;
                        };
                        match hex::decode(sig) {
                            Ok(sig) => sig,
                            Err(_) => {
                                tracing::debug!("[tower-github-webhook] Invalid header format");
                                curr_state.set(ValidateGitHubWebhookFutureState::Unauthorized);
                                cx.waker().wake_by_ref();
                                return Poll::Pending;
                            }
                        }
                    }
                    None => {
                        tracing::debug!(
                            "[tower-github-webhook] Missing X-HUB-SIGNATURE-256 header"
                        );
                        curr_state.set(ValidateGitHubWebhookFutureState::Unauthorized);
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                };
                curr_state.set(ValidateGitHubWebhookFutureState::ExtractBody);
                *this.signature = Some(signature);
                *this.req = Some(req);
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            ValidateGitHubProj::ExtractBody => {
                tracing::trace!(
                    "[tower-github-webhook] ValidateGitHubWebhookFutureState::ExtractBody"
                );
                let mut req = this.req.take().unwrap();
                let body = Pin::new(req.body_mut());
                if body.is_end_stream() {
                    curr_state.set(ValidateGitHubWebhookFutureState::ValidateSignature);
                } else {
                    let frame = ready!(Pin::new(req.body_mut()).poll_frame(cx));
                    if let Some(Ok(frame)) = frame {
                        if let Ok(data) = frame.into_data() {
                            let mut hmac = this.hmac.take().unwrap();
                            hmac.update(data.chunk());
                            *this.hmac = Some(hmac);
                        }
                    }
                }
                *this.req = Some(req);
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            ValidateGitHubProj::ValidateSignature => {
                tracing::trace!(
                    "[tower-github-webhook] ValidateGitHubWebhookFutureState::ValidateSignature"
                );
                let signature = this.signature.take().unwrap();
                let hmac = this.hmac.take().unwrap();
                if hmac.verify_slice(&signature).is_ok() {
                    tracing::debug!("[tower-github-webhook] Valid signature");
                    curr_state.set(ValidateGitHubWebhookFutureState::InnerBefore);
                } else {
                    tracing::debug!("[tower-github-webhook] Invalid signature");
                    curr_state.set(ValidateGitHubWebhookFutureState::Unauthorized);
                }
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            ValidateGitHubProj::InnerBefore => {
                tracing::trace!(
                    "[tower-github-webhook] ValidateGitHubWebhookFutureState::InnerBefore"
                );
                let req = this.req.take().unwrap();
                let fut = this.inner.call(req);
                curr_state.set(ValidateGitHubWebhookFutureState::Inner { fut });
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            ValidateGitHubProj::Inner { fut } => {
                tracing::trace!("[tower-github-webhook] ValidateGitHubWebhookFutureState::Inner");
                fut.poll(cx)
            }
            ValidateGitHubProj::Unauthorized => {
                tracing::trace!(
                    "[tower-github-webhook] ValidateGitHubWebhookFutureState::Unauthorized"
                );
                tracing::warn!("[tower-github-webhook] Request not authorized");
                let mut res = Response::new(ResBody::default());
                *res.status_mut() = StatusCode::UNAUTHORIZED;
                Poll::Ready(Ok(res))
            }
        }
    }
}

#[pin_project(project = ValidateGitHubProj)]
pub(crate) enum ValidateGitHubWebhookFutureState<
    ReqBody,
    ResBody,
    S: Service<http::Request<ReqBody>, Response = http::Response<ResBody>>,
> {
    ExtractSignature,
    ExtractBody,
    ValidateSignature,
    InnerBefore,
    Inner {
        #[pin]
        fut: S::Future,
    },
    Unauthorized,
}
