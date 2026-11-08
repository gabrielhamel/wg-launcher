//! HTTP adapter: connection lifetime, request parsing, and response streaming.

mod representation;
mod request;
mod response;
mod server;

pub(crate) use representation::{ContentType, HttpResponse};
pub(crate) use request::Request;
pub(crate) use server::{ServerHandle, bind};

use response::{write_bytes, write_empty, write_file};
use std::{io, net::TcpStream};

pub(crate) type Handler = dyn Fn(&Request) -> HttpResponse + Send + Sync + 'static;

fn serve(stream: &mut TcpStream, handler: &Handler) -> io::Result<()> {
    let (method, request) = request::read_request(stream)?;
    if method != "GET" && method != "HEAD" {
        return write_empty(stream, "405 Method Not Allowed", &["Allow: GET, HEAD"]);
    }
    match handler(&request) {
        HttpResponse::Bytes { body, content_type } => write_bytes(
            stream,
            method == "HEAD",
            &body,
            content_type.as_str(),
            request.range.as_deref(),
        ),
        HttpResponse::File { path, content_type } => write_file(
            stream,
            method == "HEAD",
            &path,
            content_type.as_str(),
            request.range.as_deref(),
        ),
        HttpResponse::NotFound => write_empty(stream, "404 Not Found", &[]),
    }
}
