//! Opaque TCP forwarding with an explicit response-byte gate. HTTP is handled
//! exclusively by the real update service, including framing and range requests.
use std::{
    io::{self, Read, Write},
    net::{Shutdown, SocketAddr, TcpListener, TcpStream},
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

const PREFIX_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Paused,
    Open,
    Disconnected,
    Stopped,
}

struct State {
    mode: Mode,
    sockets: Vec<TcpStream>,
}

struct Shared {
    state: Mutex<State>,
    changed: Condvar,
}

pub struct TransferGate {
    address: SocketAddr,
    shared: Arc<Shared>,
    accept_thread: Option<JoinHandle<()>>,
}

impl TransferGate {
    pub fn new(upstream: SocketAddr) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                mode: Mode::Paused,
                sockets: Vec::new(),
            }),
            changed: Condvar::new(),
        });
        let worker_shared = Arc::clone(&shared);
        let worker = thread::spawn(move || {
            let mut workers = Vec::new();
            loop {
                if worker_shared.state.lock().unwrap().mode == Mode::Stopped {
                    break;
                }
                match listener.accept() {
                    Ok((client, _)) => {
                        // A stopped upstream must close connections promptly rather
                        // than leave the launcher waiting for a response.
                        let mut state = worker_shared.state.lock().unwrap();
                        if matches!(state.mode, Mode::Disconnected | Mode::Stopped) {
                            continue;
                        }
                        let Ok(server) =
                            TcpStream::connect_timeout(&upstream, Duration::from_secs(2))
                        else {
                            continue;
                        };
                        state.sockets.push(client.try_clone().unwrap());
                        state.sockets.push(server.try_clone().unwrap());
                        drop(state);
                        let shared = Arc::clone(&worker_shared);
                        workers.push(thread::spawn(move || forward(client, server, shared)));
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(1))
                    }
                    Err(error) => panic!("TCP gate accept failed: {error}"),
                }
            }
            for worker in workers {
                worker.join().unwrap();
            }
        });
        Self {
            address,
            shared,
            accept_thread: Some(worker),
        }
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.address)
    }

    /// Abort established streams and reject new ones while the real service is down.
    pub fn disconnect(&self) {
        self.set_mode(Mode::Disconnected);
    }

    /// Forward future and currently paused response bytes without a size limit.
    pub fn open(&self) {
        self.set_mode(Mode::Open);
    }

    fn set_mode(&self, mode: Mode) {
        let mut state = self.shared.state.lock().unwrap();
        state.mode = mode;
        if matches!(mode, Mode::Disconnected | Mode::Stopped) {
            for socket in state.sockets.drain(..) {
                let _ = socket.shutdown(Shutdown::Both);
            }
        }
        self.shared.changed.notify_all();
    }
}

impl Drop for TransferGate {
    fn drop(&mut self) {
        self.set_mode(Mode::Stopped);
        self.accept_thread.take().unwrap().join().unwrap();
    }
}

fn forward(mut client: TcpStream, mut server: TcpStream, shared: Arc<Shared>) {
    let mut request = client.try_clone().unwrap();
    let mut upstream = server.try_clone().unwrap();
    let upload = thread::spawn(move || {
        let _ = io::copy(&mut request, &mut upstream);
        let _ = upstream.shutdown(Shutdown::Write);
    });
    let mut forwarded = 0;
    let mut buffer = [0; 16 * 1024];
    'response: while let Ok(count) = server.read(&mut buffer) {
        if count == 0 {
            break;
        }
        let mut offset = 0;
        while offset < count {
            let mut state = shared.state.lock().unwrap();
            while state.mode == Mode::Paused && forwarded >= PREFIX_BYTES {
                state = shared.changed.wait(state).unwrap();
            }
            let limit = match state.mode {
                Mode::Paused => (PREFIX_BYTES - forwarded).min(count - offset),
                Mode::Open => count - offset,
                Mode::Disconnected | Mode::Stopped => break 'response,
            };
            drop(state);
            if client.write_all(&buffer[offset..offset + limit]).is_err() {
                break 'response;
            }
            forwarded += limit;
            offset += limit;
        }
    }
    let _ = client.shutdown(Shutdown::Both);
    let _ = server.shutdown(Shutdown::Both);
    upload.join().unwrap();
}
