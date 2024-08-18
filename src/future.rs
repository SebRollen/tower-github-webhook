use bytes::{Buf, Bytes, BytesMut};
use hmac::{Hmac, Mac};
use http::{request::Parts, Request, Response, StatusCode};
use http_body::Body;
use http_body_util::{Either, Empty, Full};
use pin_project_lite::pin_project;
use sha2::Sha256;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower_service::Service;

type FutureResponse<ResBody, Error> = Result<Response<Either<ResBody, Empty<Bytes>>>, Error>;

pin_project! {
  pub struct Future<S: Service<Request<Full<Bytes>>, Response = Response<ResBody>>, ReqBody, ResBody> {
      // We use Option<X> here and for `hmac` to make it easy to move these fields out of the future
      // later.
      parts: Option<Parts>,
      buffer: BytesMut,
      inner: S,
      hmac: Option<Hmac<Sha256>>,
      #[pin]
      body: ReqBody,
      #[pin]
      state: State<S::Future>,
    }
}

impl<S, ReqBody, ResBody> Future<S, ReqBody, ResBody>
where
    S: Service<Request<Full<Bytes>>, Response = Response<ResBody>>,
    ReqBody: Body,
{
    pub fn new(req: Request<ReqBody>, hmac: Hmac<Sha256>, inner: S) -> Self {
        let (parts, body) = req.into_parts();
        let body_size = body.size_hint().lower().try_into().unwrap_or(0);
        let buffer = BytesMut::with_capacity(body_size);
        Self {
            parts: Some(parts),
            body,
            buffer,
            inner,
            hmac: Some(hmac),
            state: State::new(),
        }
    }
}

pin_project! {
    #[project = StateProj]
    enum State<F> {
        ExtractSignature,
        ExtractBody {
            signature: Vec<u8>,
        },
        Inner {
            #[pin]
            fut: F,
        },
    }
}

impl<F> State<F> {
    pub fn new() -> Self {
        Self::ExtractSignature
    }
}

impl<S, F, ReqBody, ResBody> std::future::Future for Future<S, ReqBody, ResBody>
where
    S: Service<Request<Full<Bytes>>, Response = Response<ResBody>, Future = F>,
    F: std::future::Future<Output = Result<Response<ResBody>, S::Error>>,
    ReqBody: Body,
{
    type Output = FutureResponse<ResBody, S::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();
        let mut curr_state = this.state;
        match curr_state.as_mut().project() {
            StateProj::ExtractSignature => {
                let parts = this
                    .parts
                    .take()
                    .expect("Parts is either reset at the end of this method, or we bail");
                let Some(signature) = parts.headers.get("x-hub-signature-256") else {
                    return bail("Missing X-HUB-SIGNATURE-256 header");
                };
                let Some(signature) = signature.as_bytes().splitn(2, |x| x == &b'=').nth(1) else {
                    return bail("Invalid header format");
                };
                let Ok(signature) = hex::decode(signature) else {
                    return bail("Invalid header format");
                };
                *this.parts = Some(parts);
                curr_state.set(State::ExtractBody { signature });
                rewake(cx)
            }
            StateProj::ExtractBody { signature } => {
                if this.body.is_end_stream() {
                    // We're done updating the HMAC, so we can now move it out
                    let hmac = this
                        .hmac
                        .take()
                        .expect("HMAC is only moved out of the option once, here");
                    if hmac.verify_slice(signature).is_ok() {
                        let parts = this.parts.take().unwrap();
                        let body = Full::new(this.buffer.split().freeze());
                        let req = Request::from_parts(parts, body);
                        let fut = this.inner.call(req);
                        curr_state.set(State::Inner { fut });
                        rewake(cx)
                    } else {
                        bail("Invalid signature")
                    }
                } else {
                    let Poll::Ready(maybe_frame) = this.body.poll_frame(cx) else {
                        return Poll::Pending;
                    };
                    if let Some(Ok(frame)) = maybe_frame {
                        if let Ok(data) = frame.into_data() {
                            let bytes = data.chunk();
                            this.buffer.extend(bytes);
                            let Some(h) = this.hmac.as_mut() else {
                                unreachable!()
                            };
                            h.update(bytes);
                        }
                    }
                    rewake(cx)
                }
            }
            StateProj::Inner { fut } => {
                let Poll::Ready(response) = fut.poll(cx) else {
                    return Poll::Pending;
                };
                let response = response?;
                Poll::Ready(Ok(response.map(|b| Either::Left(b))))
            }
        }
    }
}

fn bail<ResBody, Error>(debug_message: &str) -> Poll<FutureResponse<ResBody, Error>> {
    tracing::debug!("[tower-github-webhook] {debug_message}");
    tracing::warn!("[tower-github-webhook] Request not authorized");
    let mut res = Response::new(Either::Right(Empty::new()));
    *res.status_mut() = StatusCode::UNAUTHORIZED;
    Poll::Ready(Ok(res))
}

fn rewake<T>(cx: &mut Context<'_>) -> Poll<T> {
    cx.waker().wake_by_ref();
    Poll::Pending
}
