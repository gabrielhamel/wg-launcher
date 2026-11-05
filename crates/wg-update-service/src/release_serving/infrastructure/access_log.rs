use super::http::Request;
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};
type AccessLog = Arc<Mutex<File>>;

pub(crate) fn open(path: Option<PathBuf>) -> io::Result<Option<AccessLog>> {
    path.map(|path| {
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
    .transpose()
}

pub(crate) fn record(access_log: &Option<AccessLog>, request: &Request) {
    if let Some(access_log) = access_log {
        let mut access_log = access_log.lock().unwrap();
        let _ = writeln!(
            access_log,
            "{}\t{}\t{}",
            request.path,
            request.query.as_deref().unwrap_or_default(),
            request.range.as_deref().unwrap_or_default()
        );
    }
}

fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}
