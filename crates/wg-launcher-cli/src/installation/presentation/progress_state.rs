//! Shared initialization and synchronized access to one workflow's progress display.

use std::sync::{Arc, Mutex};

pub(super) struct ProgressState<T> {
    inner: Arc<Mutex<Option<T>>>,
}

impl<T> Default for ProgressState<T> {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(None)),
        }
    }
}

impl<T> Clone for ProgressState<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> ProgressState<T> {
    pub(super) fn initialize<E: std::fmt::Display>(&self, progress: Result<T, E>) {
        match progress {
            Ok(progress) => *self.inner.lock().expect("progress lock") = Some(progress),
            Err(error) => eprintln!("warning: terminal progress unavailable: {error}"),
        }
    }

    pub(super) fn with(&self, render: impl FnOnce(&T)) {
        if let Some(progress) = self.inner.lock().expect("progress lock").as_ref() {
            render(progress);
        }
    }
}
