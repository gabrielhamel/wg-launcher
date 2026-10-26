use crate::support::binary;
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Component, Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub path: String,
    pub query: Option<String>,
    pub range: Option<String>,
}

pub struct WgUpdateService {
    address: SocketAddr,
    directory: PathBuf,
    release: PathBuf,
    access_log: PathBuf,
    prefix: String,
    child: Mutex<Child>,
}

impl WgUpdateService {
    pub fn start(resources: BTreeMap<String, Vec<u8>>) -> Self {
        let directory = temporary_directory();
        let release = directory.join("release");
        let prefix = write_release(&release, resources);
        let access_log = directory.join("access.log");
        let address = available_address();
        let child = spawn_service(&directory, &release, &access_log, &prefix, address);
        Self {
            address,
            directory,
            release,
            access_log,
            prefix,
            child: Mutex::new(child),
        }
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.address)
    }

    pub fn requests(&self) -> Vec<Request> {
        let contents = fs::read_to_string(&self.access_log).unwrap_or_default();
        contents
            .lines()
            .map(|line| {
                let mut fields = line.splitn(3, '\t');
                let path = fields.next().unwrap_or_default().to_owned();
                let query = nonempty(fields.next());
                let range = nonempty(fields.next());
                Request { path, query, range }
            })
            .collect()
    }

    pub fn replace_resources(&self, resources: BTreeMap<String, Vec<u8>>) {
        let mut child = self.child.lock().unwrap();
        child.kill().unwrap();
        child.wait().unwrap();
        let prefix = write_release(&self.release, resources);
        assert_eq!(prefix, self.prefix, "replacement changed the API prefix");
        *child = spawn_service(
            &self.directory,
            &self.release,
            &self.access_log,
            &self.prefix,
            self.address,
        );
    }
}

impl Drop for WgUpdateService {
    fn drop(&mut self) {
        let child = self.child.get_mut().unwrap();
        let _ = child.kill();
        let _ = child.wait();
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn spawn_service(
    directory: &Path,
    release: &Path,
    access_log: &Path,
    prefix: &str,
    address: SocketAddr,
) -> Child {
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join("stdout.log"))
        .unwrap();
    let stderr_path = directory.join("stderr.log");
    let stderr = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&stderr_path)
        .unwrap();
    let mut child = Command::new(binary("wg-update-service"))
        .args([
            "--input",
            release.to_str().unwrap(),
            "--listen",
            &address.to_string(),
            "--prefix",
            prefix,
            "--access-log",
            access_log.to_str().unwrap(),
        ])
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if TcpStream::connect(address).is_ok() {
            return child;
        }
        assert!(
            child.try_wait().unwrap().is_none(),
            "service exited early: {}",
            fs::read_to_string(&stderr_path).unwrap()
        );
        assert!(Instant::now() < deadline, "service did not become ready");
        thread::sleep(Duration::from_millis(10));
    }
}

fn write_release(release: &Path, resources: BTreeMap<String, Vec<u8>>) -> String {
    if release.exists() {
        fs::remove_dir_all(release).unwrap();
    }
    let packages = release.join("packages");
    fs::create_dir_all(&packages).unwrap();
    let metadata_key = resources
        .keys()
        .find(|key| key.ends_with("api/v1/metadata/"))
        .cloned()
        .expect("fixture has no metadata route");
    let patch_chain_key = resources
        .keys()
        .find(|key| key.ends_with("api/v1/patches_chain/"))
        .cloned()
        .expect("fixture has no patch-chain route");
    let prefix = metadata_key
        .strip_suffix("api/v1/metadata/")
        .unwrap()
        .trim_matches('/')
        .to_owned();
    fs::write(release.join("metadata.xml"), &resources[&metadata_key]).unwrap();
    fs::write(
        release.join("patches_chain.xml"),
        &resources[&patch_chain_key],
    )
    .unwrap();
    for (name, body) in resources {
        if name == metadata_key || name == patch_chain_key {
            continue;
        }
        let relative = Path::new(&name);
        assert!(
            relative
                .components()
                .all(|part| matches!(part, Component::Normal(_))),
            "unsafe fixture path {name}"
        );
        let target = packages.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(target, body).unwrap();
    }
    prefix
}

fn temporary_directory() -> PathBuf {
    loop {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "wg-update-service-e2e-{}-{sequence}",
            std::process::id()
        ));
        match fs::create_dir(&path) {
            Ok(()) => return path,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("cannot create test directory: {error}"),
        }
    }
}

fn available_address() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap()
}

fn nonempty(value: Option<&str>) -> Option<String> {
    value.filter(|value| !value.is_empty()).map(str::to_owned)
}
