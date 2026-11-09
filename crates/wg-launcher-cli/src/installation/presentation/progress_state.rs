//! One-time initialization shared by a workflow observer and its finish handler.

use std::sync::{Arc, OnceLock};

pub(super) struct ProgressState<T> {
    inner: Arc<OnceLock<T>>,
}

impl<T> Default for ProgressState<T> {
    fn default() -> Self {
        Self {
            inner: Arc::new(OnceLock::new()),
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
            Ok(progress) => {
                self.inner.get_or_init(|| progress);
            }
            Err(error) => eprintln!("warning: terminal progress unavailable: {error}"),
        }
    }

    pub(super) fn with(&self, render: impl FnOnce(&T)) {
        if let Some(progress) = self.inner.get() {
            render(progress);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{convert::Infallible, sync::mpsc, thread, time::Duration};

    #[test]
    fn shared_display_access_does_not_serialize_independent_renderers() {
        let state = ProgressState::default();
        state.initialize(Ok::<_, Infallible>(()));
        let other = state.clone();
        let (entered, rendered) = mpsc::channel();
        let mut worker = None;
        let mut rendered_concurrently = false;
        state.with(|_| {
            worker = Some(thread::spawn(move || {
                other.with(|_| entered.send(()).unwrap());
            }));
            rendered_concurrently = rendered.recv_timeout(Duration::from_secs(5)).is_ok();
        });
        // Join after leaving the first callback so a regression fails without deadlocking.
        worker.unwrap().join().unwrap();
        assert!(rendered_concurrently);
    }
}
