//! Bounded delta output writes and throttled byte notifications.

use std::io::{self, BufWriter, Write};
use std::time::{Duration, Instant};

pub(super) struct DeltaWriter<W: Write, F> {
    inner: BufWriter<W>,
    progress: F,
    written: u64,
    last_notification: Instant,
}

impl<W: Write, F: FnMut(u64)> DeltaWriter<W, F> {
    pub(super) fn new(inner: W, mut progress: F) -> Self {
        progress(0);
        Self {
            inner: BufWriter::new(inner),
            progress,
            written: 0,
            last_notification: Instant::now(),
        }
    }

    pub(super) fn finish(&mut self) -> io::Result<()> {
        self.inner.flush()?;
        (self.progress)(self.written);
        Ok(())
    }
}

impl<W: Write, F: FnMut(u64)> Write for DeltaWriter<W, F> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        // Decoders may submit an entire large source range in one call. Bound
        // each write so progress can be reported while that range is copied.
        let length = bytes.len().min(1024 * 1024);
        let count = self.inner.write(&bytes[..length])?;
        self.written += count as u64;
        if self.last_notification.elapsed() >= Duration::from_millis(100) {
            (self.progress)(self.written);
            self.last_notification = Instant::now();
        }
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
