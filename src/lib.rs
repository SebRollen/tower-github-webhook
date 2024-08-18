//! # Overview
//!
//! `tower-github-webhook` is a crate for verifying signed webhooks received from GitHub.
//!
//! The crate exports two structs: `ValidateGitHubWebhookLayer` and `ValidateGitHubWebhook`. These
//! structs implement `tower_layer::Layer` and `tower_service::Service`, respectively, and so can
//! be used as middleware for any servers that build on top of the Tower ecosystem.
mod future;
mod layer;
mod service;
#[cfg(test)]
mod tests;

pub use layer::ValidateGitHubWebhookLayer;
pub use service::ValidateGitHubWebhook;
