//! Shared HTTPS, redirect, timeout, and bounded metadata-transfer policy.

use crate::self_update::Result;
use reqwest::{Client, Url};
use std::time::Duration;

pub(super) fn validate_url(url: &Url) -> Result<()> {
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if url.scheme() == "https" || (url.scheme() == "http" && local) {
        Ok(())
    } else {
        Err("CLI updates require HTTPS (HTTP is allowed only for loopback mirrors).".into())
    }
}

pub(super) fn client(timeout: Duration) -> Result<Client> {
    Ok(Client::builder()
        .user_agent(concat!("wg-launcher/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(2))
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 10 {
                attempt.error("Too many CLI update redirects")
            } else if validate_url(attempt.url()).is_ok() {
                attempt.follow()
            } else {
                attempt.error("Unsafe CLI update redirect")
            }
        }))
        .build()?)
}

pub(super) async fn fetch(client: &Client, url: Url, limit: usize) -> Result<Vec<u8>> {
    let mut response = client.get(url).send().await?.error_for_status()?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err("CLI update metadata is too large.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
