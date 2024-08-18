use crate::ValidateGitHubWebhookLayer;
use bytes::Bytes;
use hmac::{Hmac, Mac};
use http::{Request, Response, StatusCode};
use http_body_util::Full;
use sha2::Sha256;
use tower::{service_fn, util::ServiceExt, BoxError, Layer};

async fn echo<B: http_body::Body>(req: Request<B>) -> Result<Response<B>, BoxError> {
    Ok(Response::new(req.into_body()))
}

type EmptyBody = http_body_util::Empty<Bytes>;

#[tokio::test]
async fn gives_unauthorized_error_when_no_header() {
    let svc_fun = service_fn(echo);
    let svc = ValidateGitHubWebhookLayer::new("123").layer(svc_fun);
    let res = svc.oneshot(Request::new(EmptyBody::new())).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED)
}

#[tokio::test]
async fn gives_unauthorized_error_when_wrong_signature() {
    let svc_fun = service_fn(echo);
    let svc = ValidateGitHubWebhookLayer::new("123").layer(svc_fun);
    let res = svc
        .oneshot(
            Request::builder()
                .header("x-hub-signature-256", "sha256=fake")
                .body(EmptyBody::new())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED)
}

#[tokio::test]
async fn gives_ok_when_correct_signature() {
    use http_body_util::BodyExt;

    let svc_fun = service_fn(echo);
    let svc = ValidateGitHubWebhookLayer::new("123").layer(svc_fun);
    let mut hmac =
        Hmac::<Sha256>::new_from_slice("123".as_bytes()).expect("Failed to parse webhook secret");
    hmac.update(b"hello world");
    let signature = format!("sha256={}", hex::encode(hmac.finalize().into_bytes()));

    let res = svc
        .oneshot(
            Request::builder()
                .header("x-hub-signature-256", signature)
                .body(Full::new(Bytes::from_static(b"hello world")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.into_body().collect().await.unwrap().to_bytes(),
        Bytes::from("hello world")
    );
}
