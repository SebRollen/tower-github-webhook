# tower-github-webhook

## WORK IN PROGRESS

This crate does not currently work as intended—the middleware empties the request body completely rather than passing the body on to the inner service.

I would not recommend using this crate for anything meaningful until I have had time to fix the issue

`tower-github-webhook` is a crate that simplifies validating webhooks received from GitHub.

[![Crates.io](https://img.shields.io/crates/v/tower-github-webhook)](https://crates.io/crates/tower-github-webhook)
[![Documentation](https://docs.rs/tower-github-webhook/badge.svg)](https://docs.rs/tower-github-webhook)
[![Crates.io](https://img.shields.io/crates/l/tower-github-webhook)](tower-github-webhook/LICENSE)
