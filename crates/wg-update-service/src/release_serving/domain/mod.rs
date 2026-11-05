mod error;
pub(crate) use error::ReleaseError;
use error::{Result, invalid_input};

pub(crate) fn normalize_prefix(prefix: &str) -> Result<String> {
    let prefix = prefix.trim_matches('/');
    if prefix
        .split('/')
        .any(|part| part == "." || part == ".." || part.contains(['?', '#']))
    {
        return Err(invalid_input(format!("invalid URL path prefix '{prefix}'")));
    }
    Ok(prefix.to_owned())
}

pub(crate) fn api_route(prefix: &str, route: &str) -> String {
    if prefix.is_empty() {
        format!("/api/v1/{route}/")
    } else {
        format!("/{prefix}/api/v1/{route}/")
    }
}

pub(crate) fn render_text(body: Vec<u8>, public_url: &str, name: &str) -> Result<Vec<u8>> {
    String::from_utf8(body)
        .map(|body| body.replace("__BASE_URL__", public_url).into_bytes())
        .map_err(|_| invalid_input(format!("'{name}' must contain UTF-8 text")))
}

pub(crate) fn validate_public_url(url: &str) -> Result<String> {
    let url = url.trim_end_matches('/');
    let parsed = url::Url::parse(url)
        .map_err(|_| invalid_input("--public-url must be an absolute HTTP(S) origin"))?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || parsed.path() != "/"
    {
        return Err(invalid_input(
            "--public-url must be an absolute HTTP(S) origin without credentials, path, query, or fragment",
        ));
    }
    Ok(url.to_owned())
}
