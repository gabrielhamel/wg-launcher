use super::super::application::{PackageKind, ReleaseContent};
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

pub(crate) struct Request {
    pub(crate) path: String,
    pub(crate) query: Option<String>,
    pub(crate) range: Option<String>,
}

pub(crate) type Handler = dyn Fn(&Request) -> ReleaseContent + Send + Sync + 'static;

pub(crate) struct ServerHandle {
    pub(crate) address: SocketAddr,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl ServerHandle {
    pub(crate) fn start(listener: TcpListener, handler: Arc<Handler>) -> Self {
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let worker = thread::spawn(move || {
            for connection in listener.incoming() {
                if stopping.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(mut stream) = connection else {
                    continue;
                };
                let handler = Arc::clone(&handler);
                thread::spawn(move || {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
                    let _ = serve(&mut stream, handler.as_ref());
                });
            }
        });
        Self {
            address,
            stop,
            worker: Some(worker),
        }
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect_timeout(&self.address, Duration::from_secs(2));
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn serve(stream: &mut TcpStream, handler: &Handler) -> io::Result<()> {
    let (method, request) = read_request(stream)?;
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
        ReleaseContent::Package { path, kind } => write_file(
            stream,
            method == "HEAD",
            &path,
            match kind {
                PackageKind::Xml => "application/xml",
                PackageKind::Json => "application/json",
                PackageKind::Binary => "application/octet-stream",
            },
            request.range.as_deref(),
        ),
        ReleaseContent::Missing => write_empty(stream, "404 Not Found", &[]),
    }
}

fn read_request(stream: &mut TcpStream) -> io::Result<(String, Request)> {
    let mut header = Vec::new();
    let mut byte = [0];
    while !header.ends_with(b"\r\n\r\n") {
        if header.len() >= 32 * 1024 || stream.read(&mut byte)? == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "incomplete or oversized HTTP request header",
            ));
        }
        header.push(byte[0]);
    }
    let header = std::str::from_utf8(&header)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "non-UTF-8 HTTP header"))?;
    let mut lines = header.lines();
    let mut first = lines.next().unwrap_or_default().split_whitespace();
    let method = first.next().unwrap_or_default();
    let target = first.next().unwrap_or_default();
    if !target.starts_with('/') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "HTTP request target must be an absolute path",
        ));
    }
    let (path, query) = target
        .split_once('?')
        .map_or((target, None), |(path, query)| {
            (path, Some(query.to_owned()))
        });
    let range = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("range"))
        .map(|(_, value)| value.trim().to_owned());
    Ok((
        method.to_owned(),
        Request {
            path: path.to_owned(),
            query,
            range,
        },
    ))
}

fn write_bytes(
    stream: &mut TcpStream,
    head: bool,
    body: &[u8],
    content_type: &str,
    range: Option<&str>,
) -> io::Result<()> {
    let Some(start) = range_start(range, body.len() as u64) else {
        return write_range_not_satisfiable(stream, body.len() as u64);
    };
    let length = body.len() as u64 - start;
    write_headers(
        stream,
        content_type,
        body.len() as u64,
        start,
        length,
        range.is_some(),
    )?;
    if !head {
        stream.write_all(&body[start as usize..])?;
    }
    Ok(())
}

fn write_file(
    stream: &mut TcpStream,
    head: bool,
    path: &Path,
    content_type: &str,
    range: Option<&str>,
) -> io::Result<()> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return write_empty(stream, "404 Not Found", &[]);
        }
        Err(error) => return Err(error),
    };
    let total = file.metadata()?.len();
    let Some(start) = range_start(range, total) else {
        return write_range_not_satisfiable(stream, total);
    };
    let length = total - start;
    write_headers(stream, content_type, total, start, length, range.is_some())?;
    if !head {
        file.seek(SeekFrom::Start(start))?;
        io::copy(&mut file.take(length), stream)?;
    }
    Ok(())
}

fn range_start(range: Option<&str>, total: u64) -> Option<u64> {
    let Some(range) = range else {
        return Some(0);
    };
    let start = range
        .strip_prefix("bytes=")
        .and_then(|value| value.strip_suffix('-'))
        .and_then(|value| value.parse::<u64>().ok());
    start.filter(|start| *start < total)
}

fn write_headers(
    stream: &mut TcpStream,
    content_type: &str,
    total: u64,
    start: u64,
    length: u64,
    partial: bool,
) -> io::Result<()> {
    let status = if partial {
        "206 Partial Content"
    } else {
        "200 OK"
    };
    let content_range = if partial {
        format!("Content-Range: bytes {start}-{}/{total}\r\n", total - 1)
    } else {
        String::new()
    };
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Length: {length}\r\nContent-Type: {content_type}\r\n{content_range}Accept-Ranges: bytes\r\nConnection: close\r\n\r\n"
    )
}

fn write_range_not_satisfiable(stream: &mut TcpStream, total: u64) -> io::Result<()> {
    write_empty(
        stream,
        "416 Range Not Satisfiable",
        &[&format!("Content-Range: bytes */{total}")],
    )
}

fn write_empty(stream: &mut TcpStream, status: &str, headers: &[&str]) -> io::Result<()> {
    write!(stream, "HTTP/1.1 {status}\r\n")?;
    for header in headers {
        write!(stream, "{header}\r\n")?;
    }
    stream.write_all(b"Content-Length: 0\r\nConnection: close\r\n\r\n")
}

pub(crate) fn bind(address: SocketAddr) -> io::Result<TcpListener> {
    TcpListener::bind(address)
}
