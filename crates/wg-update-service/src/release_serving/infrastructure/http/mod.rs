//! HTTP adapter: connection lifetime, request parsing, and response streaming.

mod request;
mod response;
mod server;

pub(crate) use request::Request;
pub(crate) use server::{ServerHandle, bind};

use super::super::application::{ContentKind, ReleaseContent};
use response::{write_bytes, write_empty, write_file};
use std::{io, net::TcpStream};

pub(crate) type Handler = dyn Fn(&Request) -> ReleaseContent + Send + Sync + 'static;

fn serve(stream: &mut TcpStream, handler: &Handler) -> io::Result<()> {
    let (method, request) = request::read_request(stream)?;
    if method != "GET" && method != "HEAD" {
        return write_empty(stream, "405 Method Not Allowed", &["Allow: GET, HEAD"]);
    }
    match handler(&request) {
        ReleaseContent::Metadata(body) | ReleaseContent::PatchChain(body) => write_bytes(
            stream,
            method == "HEAD",
            &body,
            "application/xml",
            request.range.as_deref(),
        ),
        ReleaseContent::File { path, kind } => write_file(
            stream,
            method == "HEAD",
            &path,
            match kind {
                ContentKind::Xml => "application/xml",
                ContentKind::Json => "application/json",
                ContentKind::Binary => "application/octet-stream",
            },
            request.range.as_deref(),
        ),
        ReleaseContent::Missing => write_empty(stream, "404 Not Found", &[]),
    }
}
