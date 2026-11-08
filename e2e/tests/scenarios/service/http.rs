use crate::{
    fixture::Release,
    server::WgUpdateService,
    support::{Sandbox, success},
};
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

fn request(
    service: &WgUpdateService,
    method: &str,
    path: &str,
    range: Option<&str>,
) -> (String, Vec<u8>) {
    let address = service
        .url()
        .strip_prefix("http://")
        .unwrap()
        .parse()
        .unwrap();
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(5)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let range = range
        .map(|value| format!("Range: {value}\r\n"))
        .unwrap_or_default();
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\n{range}Connection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    let split = response
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .unwrap();
    (
        String::from_utf8(response[..split].to_vec()).unwrap(),
        response[split + 4..].to_vec(),
    )
}

#[test]
fn discovered_release_serves_full_head_resumed_and_unsatisfiable_responses() {
    let sandbox = Sandbox::new();
    let fixture = sandbox.fixture(Release::V1);
    let mut resources = fixture.service_resources("updates");
    resources.insert("empty.bin".into(), Vec::new());
    let service = WgUpdateService::start(resources);
    let config = sandbox.configuration(&service);
    let listed = success(sandbox.run(
        &sandbox.path,
        &["list", "--product", config.to_str().unwrap()],
    ));
    assert!(listed.contains("1.0"));

    // Exercise the same range handling with memory-backed XML and disk-backed packages.
    for path in ["/updates/api/v1/metadata/", "/client.wgpkg"] {
        let (headers, body) = request(&service, "GET", path, None);
        assert!(headers.starts_with("HTTP/1.1 200 OK"), "{headers}");
        assert!(headers.contains(&format!("Content-Length: {}", body.len())));
        assert!(!body.is_empty());
        if path.ends_with(".wgpkg") {
            assert_eq!(body, fixture.packages["client.wgpkg"]);
        }
        let (head_headers, head_body) = request(&service, "HEAD", path, None);
        assert_eq!(head_headers, headers);
        assert!(head_body.is_empty());
        for offset in [0, 1, body.len() - 1] {
            let range = format!("bytes={offset}-");
            let (headers, resumed) = request(&service, "GET", path, Some(&range));
            assert!(
                headers.starts_with("HTTP/1.1 206 Partial Content"),
                "{headers}"
            );
            assert!(headers.contains(&format!(
                "Content-Range: bytes {offset}-{}/{}",
                body.len() - 1,
                body.len()
            )));
            assert!(headers.contains(&format!("Content-Length: {}", body.len() - offset)));
            assert_eq!(resumed, body[offset..]);
            let (head_headers, head_body) = request(&service, "HEAD", path, Some(&range));
            assert_eq!(head_headers, headers);
            assert!(head_body.is_empty());
        }
        for range in [
            format!("bytes={}-", body.len()),
            "bytes=invalid-".into(),
            "bytes=0-1".into(),
        ] {
            let (headers, body_response) = request(&service, "GET", path, Some(&range));
            assert!(
                headers.starts_with("HTTP/1.1 416 Range Not Satisfiable"),
                "{headers}"
            );
            assert!(headers.contains(&format!("Content-Range: bytes */{}", body.len())));
            assert!(body_response.is_empty());
        }
    }
    let (headers, body) = request(&service, "GET", "/empty.bin", None);
    assert!(headers.starts_with("HTTP/1.1 200 OK"));
    assert!(headers.contains("Content-Length: 0"));
    assert!(body.is_empty());
    let (headers, body) = request(&service, "GET", "/empty.bin", Some("bytes=0-"));
    assert!(headers.starts_with("HTTP/1.1 416 Range Not Satisfiable"));
    assert!(headers.contains("Content-Range: bytes */0"));
    assert!(body.is_empty());
    let (headers, _) = request(&service, "GET", "/missing.bin", None);
    assert!(headers.starts_with("HTTP/1.1 404 Not Found"));
    let (headers, _) = request(&service, "POST", "/client.wgpkg", None);
    assert!(headers.starts_with("HTTP/1.1 405 Method Not Allowed"));
    assert!(headers.contains("Allow: GET, HEAD"));
}
