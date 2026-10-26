use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Component, Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

const METADATA_FILE: &str = "metadata.xml";
const PATCH_CHAIN_FILE: &str = "patches_chain.xml";
const PACKAGES_DIRECTORY: &str = "packages";

struct Request {
    path: String,
    query: Option<String>,
    range: Option<String>,
}

/// Configuration for a file-backed update service.
#[derive(Clone, Debug)]
pub struct FileServiceOptions {
    /// Release root containing `metadata.xml`, `patches_chain.xml`, and `packages/`.
    pub input: PathBuf,
    /// Address to bind. Port zero selects an ephemeral port.
    pub listen: SocketAddr,
    /// Path prefix used for the WGUS API routes.
    pub prefix: String,
    /// Public HTTP(S) origin substituted for `__BASE_URL__` in XML files.
    pub public_url: Option<String>,
    /// Optional append-only tab-separated HTTP access log.
    pub access_log: Option<PathBuf>,
}

/// Running file-backed WGUS-compatible update service.
///
/// Metadata is read and rendered at startup. Package bodies are opened and streamed
/// for each request, so large release volumes are never retained in memory.
pub struct FileService {
    handle: ServerHandle,
    prefix: String,
    public_url: String,
}

impl FileService {
    /// Validates the release directory, binds the listener, and starts serving.
    pub fn start(options: FileServiceOptions) -> io::Result<Self> {
        let input = options.input.canonicalize().map_err(|error| {
            invalid_input(format!(
                "cannot open input directory '{}': {error}",
                options.input.display()
            ))
        })?;
        if !input.is_dir() {
            return Err(invalid_input(format!(
                "input '{}' is not a directory",
                input.display()
            )));
        }
        let packages = input.join(PACKAGES_DIRECTORY);
        if !packages.is_dir() {
            return Err(invalid_input(format!(
                "missing package directory '{}'",
                packages.display()
            )));
        }
        let metadata = read_required(&input.join(METADATA_FILE))?;
        let patch_chain = read_required(&input.join(PATCH_CHAIN_FILE))?;
        let prefix = normalize_prefix(&options.prefix)?;
        let listener = TcpListener::bind(options.listen)?;
        let address = listener.local_addr()?;
        let public_url = match options.public_url {
            Some(url) => validate_public_url(&url)?,
            None => format!("http://{address}"),
        };
        let metadata = render_text(metadata, &public_url, METADATA_FILE)?;
        let patch_chain = render_text(patch_chain, &public_url, PATCH_CHAIN_FILE)?;
        let metadata_route = api_route(&prefix, "metadata");
        let patch_chain_route = api_route(&prefix, "patches_chain");
        let access_log = options
            .access_log
            .map(|path| {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .map(Mutex::new)
                    .map(Arc::new)
                    .map_err(|error| {
                        invalid_input(format!(
                            "cannot open access log '{}': {error}",
                            path.display()
                        ))
                    })
            })
            .transpose()?;
        let handler = Arc::new(move |request: &Request| {
            if let Some(access_log) = &access_log {
                let mut access_log = access_log.lock().unwrap();
                let _ = writeln!(
                    access_log,
                    "{}\t{}\t{}",
                    request.path,
                    request.query.as_deref().unwrap_or_default(),
                    request.range.as_deref().unwrap_or_default()
                );
            }
            if request.path == metadata_route {
                return Resource::Bytes(metadata.clone(), "application/xml");
            }
            if request.path == patch_chain_route {
                return Resource::Bytes(patch_chain.clone(), "application/xml");
            }
            package_resource(&packages, &request.path)
        });
        let handle = ServerHandle::start(listener, handler);
        Ok(Self {
            handle,
            prefix,
            public_url,
        })
    }

    /// Public origin used when rendering the release metadata.
    pub fn url(&self) -> &str {
        &self.public_url
    }

    /// URL to store in a launcher's custom product configuration.
    pub fn product_host(&self) -> String {
        if self.prefix.is_empty() {
            self.public_url.clone()
        } else {
            format!("{}/{}", self.public_url, self.prefix)
        }
    }

    /// Bound socket address, including the selected port when port zero was used.
    pub fn local_addr(&self) -> SocketAddr {
        self.handle.address
    }
}

type Handler = dyn Fn(&Request) -> Resource + Send + Sync + 'static;

struct ServerHandle {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl ServerHandle {
    fn start(listener: TcpListener, handler: Arc<Handler>) -> Self {
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

enum Resource {
    Bytes(Vec<u8>, &'static str),
    File(PathBuf, &'static str),
    NotFound,
}

fn serve(stream: &mut TcpStream, handler: &Handler) -> io::Result<()> {
    let (method, request) = read_request(stream)?;
    if method != "GET" && method != "HEAD" {
        return write_empty(stream, "405 Method Not Allowed", &["Allow: GET, HEAD"]);
    }
    match handler(&request) {
        Resource::Bytes(body, content_type) => write_bytes(
            stream,
            method == "HEAD",
            &body,
            content_type,
            request.range.as_deref(),
        ),
        Resource::File(path, content_type) => write_file(
            stream,
            method == "HEAD",
            &path,
            content_type,
            request.range.as_deref(),
        ),
        Resource::NotFound => write_empty(stream, "404 Not Found", &[]),
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

fn package_resource(packages: &Path, request_path: &str) -> Resource {
    let relative = request_path.trim_start_matches('/');
    let relative = relative.strip_prefix("packages/").unwrap_or(relative);
    let path = Path::new(relative);
    if relative.is_empty()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Resource::NotFound;
    }
    let Ok(file) = packages.join(path).canonicalize() else {
        return Resource::NotFound;
    };
    if !file.starts_with(packages) || !file.is_file() {
        return Resource::NotFound;
    }
    Resource::File(file, content_type(relative))
}

fn normalize_prefix(prefix: &str) -> io::Result<String> {
    let prefix = prefix.trim_matches('/');
    if prefix
        .split('/')
        .any(|part| part == "." || part == ".." || part.contains(['?', '#']))
    {
        return Err(invalid_input(format!("invalid URL path prefix '{prefix}'")));
    }
    Ok(prefix.to_owned())
}

fn api_route(prefix: &str, route: &str) -> String {
    if prefix.is_empty() {
        format!("/api/v1/{route}/")
    } else {
        format!("/{prefix}/api/v1/{route}/")
    }
}

fn read_required(path: &Path) -> io::Result<Vec<u8>> {
    fs::read(path).map_err(|error| {
        invalid_input(format!(
            "cannot read required file '{}': {error}",
            path.display()
        ))
    })
}

fn render_text(body: Vec<u8>, public_url: &str, name: &str) -> io::Result<Vec<u8>> {
    String::from_utf8(body)
        .map(|body| body.replace("__BASE_URL__", public_url).into_bytes())
        .map_err(|_| invalid_input(format!("'{name}' must contain UTF-8 text")))
}

fn validate_public_url(url: &str) -> io::Result<String> {
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

fn content_type(path: &str) -> &'static str {
    if path.ends_with(".xml") {
        "application/xml"
    } else if path.ends_with(".json") {
        "application/json"
    } else {
        "application/octet-stream"
    }
}

fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}
