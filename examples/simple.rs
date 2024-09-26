//! # Example
//!
//! This is a simple example of how to implement a webhook handler to handle incoming requests.
//!
//! The server will reject any requests that don't have a valid signature, and print the body of
//! any verified request.
//!
//! One example of a curl request that will pass:`curl -X POST -H "X-Hub-Signature-256:
//! sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17" -d "Hello, World\!"
//! localhost:3000`
use axum::debug_handler;
use axum::{routing::post, Router};
use tower_github_webhook::ValidateGitHubWebhookLayer;

const WEBHOOK_SECRET: &'static str = "It's a Secret to Everybody";

#[tokio::main]
async fn main() {
    // Setup tracing
    tracing_subscriber::fmt::init();

    // Run our service
    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app().into_make_service())
        .await
        .unwrap();
}

fn app() -> Router {
    // Build route service
    Router::new().route(
        "/",
        post(print_body).layer(ValidateGitHubWebhookLayer::new(WEBHOOK_SECRET)),
    )
}

#[debug_handler]
async fn print_body(body: String) {
    println!("{:#?}", body);
}
