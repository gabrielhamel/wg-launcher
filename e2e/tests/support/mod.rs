use crate::{
    fixture::{Fixture, Release},
    server::WgUpdateService,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

pub struct Sandbox {
    pub path: PathBuf,
}

impl Sandbox {
    pub fn new() -> Self {
        let base = std::env::temp_dir();
        loop {
            let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = base.join(format!("wg-launcher-e2e-{}-{sequence}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Self { path },
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("cannot create test directory: {error}"),
            }
        }
    }

    pub fn configuration(&self, server: &WgUpdateService) -> PathBuf {
        let path = self.path.join("custom product.json");
        fs::write(&path, Fixture::products(&server.url())).unwrap();
        path
    }

    pub fn game(&self) -> PathBuf {
        self.path.join("game with spaces")
    }

    pub fn fixture(&self, release: Release) -> Fixture {
        Fixture::build(
            release,
            &self.path.join(format!("archives-{}", release.version())),
        )
    }

    pub fn run(&self, cwd: &Path, args: &[&str]) -> Output {
        // Files avoid pipe-buffer deadlocks; polling bounds every child execution.
        let stdout = self.path.join("stdout.log");
        let stderr = self.path.join("stderr.log");
        let mut child = Command::new(crate::support::binary("wg-launcher"))
            .args(args)
            .current_dir(cwd)
            // Unexpected live discovery fails locally instead of using the internet.
            .env("HTTPS_PROXY", "http://127.0.0.1:0")
            .env("https_proxy", "http://127.0.0.1:0")
            .env("HTTP_PROXY", "http://127.0.0.1:0")
            .env("http_proxy", "http://127.0.0.1:0")
            .env("ALL_PROXY", "http://127.0.0.1:0")
            .env("all_proxy", "http://127.0.0.1:0")
            .env("NO_PROXY", "127.0.0.1,localhost")
            .env("no_proxy", "127.0.0.1,localhost")
            .stdin(Stdio::null())
            .stdout(fs::File::create(&stdout).unwrap())
            .stderr(fs::File::create(&stderr).unwrap())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "CLI timed out for {args:?}: {}",
                    fs::read_to_string(stderr).unwrap()
                );
            }
            thread::sleep(Duration::from_millis(10));
        };
        Output {
            status,
            stdout: fs::read(stdout).unwrap(),
            stderr: fs::read(stderr).unwrap(),
        }
    }
}

pub fn binary(name: &str) -> PathBuf {
    static BINARIES: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();
    let (launcher, service) = BINARIES.get_or_init(build_binaries);
    match name {
        "wg-launcher" => launcher.clone(),
        "wg-update-service" => service.clone(),
        _ => panic!("unknown workspace binary {name}"),
    }
}

fn build_binaries() -> (PathBuf, PathBuf) {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.join("target"));
    let status = Command::new("cargo")
        .args([
            "build",
            "-p",
            "wg-launcher-cli",
            "--bin",
            "wg-launcher",
            "-p",
            "wg-update-service",
            "--bin",
            "wg-update-service",
            "--locked",
        ])
        .current_dir(workspace)
        .status()
        .unwrap();
    assert!(status.success(), "failed to build workspace binaries");
    let suffix = std::env::consts::EXE_SUFFIX;
    (
        target.join("debug").join(format!("wg-launcher{suffix}")),
        target
            .join("debug")
            .join(format!("wg-update-service{suffix}")),
    )
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.path) {
            eprintln!("test directory cleanup failed: {error}");
        }
    }
}

pub fn success(output: Output) -> String {
    assert!(output.status.success(), "CLI failed: {output:?}");
    String::from_utf8(output.stdout).unwrap()
}

pub fn failure(output: Output, message: &str) {
    assert!(
        !output.status.success(),
        "CLI unexpectedly succeeded: {output:?}"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(message),
        "{output:?}"
    );
}

pub fn assert_installed(game: &Path, release: Release) {
    assert_installed_in_region(game, release, "local");
}

pub fn assert_installed_in_region(game: &Path, release: Release, region: &str) {
    assert_eq!(
        fs::read(game.join("payload.pkg")).unwrap(),
        release.payload()
    );
    assert_eq!(fs::read(game.join("locale.txt")).unwrap(), release.locale());
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(game.join(".wg-launcher/state.json")).unwrap()).unwrap();
    assert_eq!(state["version"], release.version());
    assert_eq!(state["region"], region);
    assert_eq!(state["game_id"], "LOCAL.WOT");
    assert_eq!(state["channel"], "testing");
    assert_eq!(state["product"]["region"], region);
    assert_eq!(state["product"]["game_id"], "LOCAL.WOT");
    assert_eq!(state["product"]["channel"], "testing");
    assert_eq!(state["client_type"], "sd");
    assert_eq!(state["language"], "EN");
    assert_eq!(state["platform"], "windows");
    assert_eq!(state["architecture"], "x64");
    assert!(!game.join(".wg-launcher/download").exists());
    assert!(!game.join(".wg-launcher/installing").exists());
    assert!(game.join(".wg-launcher/wg-launcher.lock").is_file());
    let mut entries = fs::read_dir(game)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    entries.sort();
    assert_eq!(entries, [".wg-launcher", "locale.txt", "payload.pkg"]);
}
