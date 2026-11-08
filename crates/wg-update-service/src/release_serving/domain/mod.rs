mod address;
mod error;
pub(crate) use address::{PublicOrigin, RoutePrefix};
pub(crate) use error::ReleaseError;
use error::{Result, invalid_input};

pub(crate) fn render_text(body: Vec<u8>, public_url: &PublicOrigin, name: &str) -> Result<Vec<u8>> {
    String::from_utf8(body)
        .map(|body| {
            body.replace("__BASE_URL__", public_url.as_str())
                .into_bytes()
        })
        .map_err(|_| invalid_input(format!("'{name}' must contain UTF-8 text")))
}
