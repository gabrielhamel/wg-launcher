use std::{
    io::{self, Read},
    net::TcpStream,
};

pub(crate) struct Request {
    pub(crate) path: String,
    pub(crate) query: Option<String>,
    pub(crate) range: Option<String>,
}

pub(super) fn read_request(stream: &mut TcpStream) -> io::Result<(String, Request)> {
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
