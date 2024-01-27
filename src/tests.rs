use crate::test_helpers::Body;
use crate::ValidateGitHubWebhookLayer;
use hmac::{Hmac, Mac};
use http::{Request, Response, StatusCode};
use sha2::Sha256;
use tower::{service_fn, util::ServiceExt, BoxError, Layer};

async fn echo(req: Request<Body>) -> Result<Response<Body>, BoxError> {
    Ok(Response::new(req.into_body()))
}

#[tokio::test]
async fn gives_unauthorized_error_when_no_header() {
    let svc_fun = service_fn(echo);
    let svc = ValidateGitHubWebhookLayer::new("123").layer(svc_fun);
    let res = svc.oneshot(Request::new(Body::empty())).await.unwrap();
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
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED)
}

#[tokio::test]
async fn gives_ok_when_correct_signature() {
    let svc_fun = service_fn(echo);
    let svc = ValidateGitHubWebhookLayer::new("123").layer(svc_fun);
    let hmac =
        Hmac::<Sha256>::new_from_slice("123".as_bytes()).expect("Failed to parse webhook secret");
    let signature = format!("sha256={}", hex::encode(hmac.finalize().into_bytes()));

    let res = svc
        .oneshot(
            Request::builder()
                .header("x-hub-signature-256", signature)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK)
}
