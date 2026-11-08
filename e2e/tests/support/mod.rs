pub mod transfer_gate;

use crate::{
    fixture::{Fixture, Release},
    server::WgUpdateService,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
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
        self.run_cli(&binary("wg-launcher"), cwd, args, &[])
    }

    pub fn run_cli(
        &self,
        launcher: &Path,
        cwd: &Path,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> Output {
        self.spawn_cli(launcher, cwd, args, env).wait()
    }

    pub fn spawn(&self, cwd: &Path, args: &[&str]) -> RunningCli {
        self.spawn_cli(&binary("wg-launcher"), cwd, args, &[])
    }

    fn spawn_cli(
        &self,
        launcher: &Path,
        cwd: &Path,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> RunningCli {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let stdout = self.path.join(format!("stdout-{sequence}.log"));
        let stderr = self.path.join(format!("stderr-{sequence}.log"));
        let child = Command::new(launcher)
            .args(args)
            .current_dir(cwd)
            .env("HTTPS_PROXY", "http://127.0.0.1:0")
            .env("https_proxy", "http://127.0.0.1:0")
            .env("HTTP_PROXY", "http://127.0.0.1:0")
            .env("http_proxy", "http://127.0.0.1:0")
            .env("ALL_PROXY", "http://127.0.0.1:0")
            .env("all_proxy", "http://127.0.0.1:0")
            .env("NO_PROXY", "127.0.0.1,localhost")
            .env("no_proxy", "127.0.0.1,localhost")
            .env_remove("WG_LAUNCHER_UPDATE_MANIFEST")
            .env_remove("WG_LAUNCHER_NO_UPDATE_CHECK")
            .envs(env.iter().copied())
            .stdin(Stdio::null())
            .stdout(fs::File::create(&stdout).unwrap())
            .stderr(fs::File::create(&stderr).unwrap())
            .spawn()
            .unwrap();
        RunningCli {
            child,
            stdout,
            stderr,
            started: Instant::now(),
        }
    }
}

/// Owns and reaps a real launcher, including on assertion failure.
pub struct RunningCli {
    child: Child,
    stdout: PathBuf,
    stderr: PathBuf,
    started: Instant,
}

impl RunningCli {
    pub fn until(&mut self, mut reached: impl FnMut() -> bool) {
        loop {
            if reached() {
                return;
            }
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "CLI exited before reaching failure point: {}",
                self.diagnostics()
            );
            assert!(
                self.started.elapsed() < Duration::from_secs(20),
                "CLI did not reach failure point: {}",
                self.diagnostics()
            );
            thread::sleep(Duration::from_micros(100));
        }
    }

    pub fn kill(&mut self) -> Output {
        self.child.kill().unwrap();
        let status = self.child.wait().unwrap();
        Output {
            status,
            stdout: fs::read(&self.stdout).unwrap(),
            stderr: fs::read(&self.stderr).unwrap(),
        }
    }

    pub fn wait(self) -> Output {
        self.wait_with_timeout(Duration::from_secs(20))
    }

    pub fn wait_with_timeout(mut self, timeout: Duration) -> Output {
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return Output {
                    status,
                    stdout: fs::read(&self.stdout).unwrap(),
                    stderr: fs::read(&self.stderr).unwrap(),
                };
            }
            assert!(
                self.started.elapsed() < timeout,
                "CLI timed out: {}",
                self.diagnostics()
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn diagnostics(&self) -> String {
        format!(
            "stdout: {}\nstderr: {}",
            fs::read_to_string(&self.stdout).unwrap_or_default(),
            fs::read_to_string(&self.stderr).unwrap_or_default()
        )
    }
}

impl Drop for RunningCli {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
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
    let fields = state
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        fields,
        [
            "architecture",
            "channel",
            "client_type",
            "component_versions",
            "game_id",
            "language",
            "metadata_version",
            "platform",
            "product",
            "region",
            "version",
        ]
        .into_iter()
        .collect(),
        "installed records retain their flat storage contract"
    );
    assert_eq!(state["metadata_version"], "1");
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
