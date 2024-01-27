use axum::async_trait;
use axum::body::Bytes;
use axum::debug_handler;
use axum::extract::{FromRequest, Request};
use axum::response::{IntoResponse, Response};
use axum::{extract::Json, routing::post, Router};
use octocrab::models::{
    webhook_events::{WebhookEvent, WebhookEventPayload, WebhookEventType},
    Author, Repository,
};
use serde::{Deserialize, Serialize};
use tower_github_webhook::ValidateGitHubWebhookLayer;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub kind: WebhookEventType,
    pub sender: Option<Author>,
    pub repository: Option<Repository>,
    pub payload: WebhookEventPayload,
}

impl From<WebhookEvent> for Event {
    fn from(e: WebhookEvent) -> Self {
        Self {
            kind: e.kind,
            sender: e.sender,
            repository: e.repository,
            payload: e.specific,
        }
    }
}

#[async_trait]
impl<S> FromRequest<S> for Event
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let headers = req.headers().clone();
        let header = headers
            .get("x-github-event")
            .map(|x| x.to_str())
            .unwrap()
            .map_err(|_| {
                "Failed to convert header to string"
                    .to_string()
                    .into_response()
            })?;
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(IntoResponse::into_response)?;
        let webhook_event = WebhookEvent::try_from_header_and_body(header, &bytes).unwrap();
        Ok(Self::from(webhook_event))
    }
}

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
        "/github/events",
        post(print_body).layer(ValidateGitHubWebhookLayer::new("123")),
    )
}

#[debug_handler]
async fn print_body(Json(event): Json<Event>) {
    println!("{:#?}", event);
}
