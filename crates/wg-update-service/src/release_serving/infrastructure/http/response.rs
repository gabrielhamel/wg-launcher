use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    net::TcpStream,
    path::Path,
};

pub(super) fn write_bytes(
    stream: &mut TcpStream,
    head: bool,
    body: &[u8],
    content_type: &str,
    range: Option<&str>,
) -> io::Result<()> {
    let Some(range) = ResponseRange::resolve(range, body.len() as u64) else {
        return write_range_not_satisfiable(stream, body.len() as u64);
    };
    write_headers(stream, content_type, &range)?;
    if !head {
        stream.write_all(&body[range.start as usize..])?;
    }
    Ok(())
}

pub(super) fn write_file(
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
    let Some(range) = ResponseRange::resolve(range, total) else {
        return write_range_not_satisfiable(stream, total);
    };
    write_headers(stream, content_type, &range)?;
    if !head {
        file.seek(SeekFrom::Start(range.start))?;
        io::copy(&mut file.take(range.length()), stream)?;
    }
    Ok(())
}

/// Validated response extent for a full body or an open-ended resumed download.
struct ResponseRange {
    total: u64,
    start: u64,
    partial: bool,
}

impl ResponseRange {
    fn resolve(header: Option<&str>, total: u64) -> Option<Self> {
        let start = match header {
            None => 0,
            Some(value) => value
                .strip_prefix("bytes=")?
                .strip_suffix('-')?
                .parse::<u64>()
                .ok()
                .filter(|start| *start < total)?,
        };
        Some(Self {
            total,
            start,
            partial: header.is_some(),
        })
    }

    fn length(&self) -> u64 {
        self.total - self.start
    }
}

fn write_headers(
    stream: &mut TcpStream,
    content_type: &str,
    range: &ResponseRange,
) -> io::Result<()> {
    let status = if range.partial {
        "206 Partial Content"
    } else {
        "200 OK"
    };
    let content_range = if range.partial {
        format!(
            "Content-Range: bytes {}-{}/{}\r\n",
            range.start,
            range.total - 1,
            range.total
        )
    } else {
        String::new()
    };
    let length = range.length();
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

pub(super) fn write_empty(
    stream: &mut TcpStream,
    status: &str,
    headers: &[&str],
) -> io::Result<()> {
    write!(stream, "HTTP/1.1 {status}\r\n")?;
    for header in headers {
        write!(stream, "{header}\r\n")?;
    }
    stream.write_all(b"Content-Length: 0\r\nConnection: close\r\n\r\n")
}
