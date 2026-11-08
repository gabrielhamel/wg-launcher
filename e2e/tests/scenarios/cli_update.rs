use crate::{
    fixture::{Release, cli_update},
    server::WgUpdateService,
    support::{Sandbox, binary, failure, success},
};
use std::{fs, path::Path};

fn server(sandbox: &Sandbox, version: &str, corrupt: bool, bad_version: bool) -> WgUpdateService {
    let fixture = sandbox.fixture(Release::V1);
    WgUpdateService::start(cli_update::resources(
        fixture.service_resources("updates"),
        version,
        corrupt,
        bad_version,
    ))
}

fn copy_launcher(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::copy(binary("wg-launcher"), path).unwrap();
}

fn executable_name() -> &'static str {
    if cfg!(windows) {
        "wg-launcher.exe"
    } else {
        "wg-launcher"
    }
}

#[test]
fn commands_show_update_notice_before_output_and_version_shows_installed_and_latest() {
    let sandbox = Sandbox::new();
    let launcher = sandbox.path.join("standalone").join(executable_name());
    copy_launcher(&launcher);
    let server = server(&sandbox, cli_update::FUTURE, false, false);
    let manifest = format!("{}/dist-manifest.json", server.url());
    let env = [("WG_LAUNCHER_UPDATE_MANIFEST", manifest.as_str())];
    let output = sandbox.run_cli(&launcher, &sandbox.path, &["--version"], &env);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains(&format!(
            "Installed:    {}\nLatest:       {}",
            env!("CARGO_PKG_VERSION"),
            cli_update::FUTURE
        )),
        "{stdout}"
    );
    assert!(
        stdout
            .contains("Changelog:    https://github.com/gabrielhamel/wg-launcher/releases/latest")
    );
    assert!(stdout.contains("Update available."));
    assert!(!stdout.contains('`'));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("CLI update available!"));
    let short = success(sandbox.run_cli(&launcher, &sandbox.path, &["-V"], &env));
    assert_eq!(short, stdout);
    let color_env = [
        ("WG_LAUNCHER_UPDATE_MANIFEST", manifest.as_str()),
        ("CLICOLOR_FORCE", "1"),
    ];
    let colored_version = success(sandbox.run_cli(&launcher, &sandbox.path, &["-V"], &color_env));
    let highlighted_command = "\u{1b}[36m\u{1b}[1mwg-launcher self-update\u{1b}[0m";
    assert!(colored_version.contains(highlighted_command));
    let colored_help = sandbox.run_cli(&launcher, &sandbox.path, &["--help"], &color_env);
    assert!(String::from_utf8_lossy(&colored_help.stderr).contains(highlighted_command));
    let output = sandbox.run_cli(&launcher, &sandbox.path, &["status"], &env);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains(&format!(
            "CLI update available! {} → 99.0.0",
            env!("CARGO_PKG_VERSION")
        )),
        "{stderr}"
    );
    assert!(stderr.contains("wg-launcher self-update"));
    assert!(
        stderr.contains("Changelog: https://github.com/gabrielhamel/wg-launcher/releases/latest")
    );
    assert!(!stderr.contains("warning:"));
    assert!(!stderr.contains('`'));
    assert!(stderr.find("CLI update available!").unwrap() < stderr.find("error:").unwrap());
    assert!(stderr.contains("error:"));
    let output = sandbox.run_cli(&launcher, &sandbox.path, &["--help"], &env);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("CLI update available!"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("self-update"));
}

#[test]
fn self_update_replaces_only_the_copied_cli_and_cleans_staging() {
    let sandbox = Sandbox::new();
    let server = server(&sandbox, cli_update::FUTURE, false, false);
    let manifest = format!("{}/dist-manifest.json", server.url());
    let launcher = sandbox.path.join("standalone").join(executable_name());
    copy_launcher(&launcher);
    let shared_launcher = binary("wg-launcher");
    let shared_before = fs::read(&shared_launcher).unwrap();
    let sibling = sandbox
        .path
        .join("another-installation")
        .join(executable_name());
    copy_launcher(&sibling);
    let sentinel = launcher.parent().unwrap().join("game-owned-file");
    fs::write(&sentinel, b"keep me").unwrap();
    let output = sandbox.run_cli(
        &launcher,
        &sandbox.path,
        &["self-update"],
        &[("WG_LAUNCHER_UPDATE_MANIFEST", &manifest)],
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains("CLI update available!"));
    assert_eq!(
        success(output).trim(),
        "✓ wg-launcher has been updated to 99.0.0.\nChangelog: https://github.com/gabrielhamel/wg-launcher/releases/latest"
    );
    assert_eq!(
        fs::read(&launcher).unwrap(),
        fs::read(env!("CARGO_BIN_EXE_cli-update-fixture")).unwrap()
    );
    assert_eq!(fs::read(sentinel).unwrap(), b"keep me");
    let output = sandbox.run_cli(&launcher, &sandbox.path, &["--version"], &[]);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "wg-launcher 99.0.0"
    );
    assert!(
        !fs::read_dir(launcher.parent().unwrap())
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".wg-launcher-self-update-"))
    );
    assert!(
        server
            .requests()
            .iter()
            .any(|request| request.path.ends_with(".sha256"))
    );
    assert_eq!(fs::read(&shared_launcher).unwrap(), shared_before);
    assert_eq!(fs::read(&sibling).unwrap(), shared_before);
    for untouched in [&shared_launcher, &sibling] {
        let output = success(sandbox.run_cli(
            untouched,
            &sandbox.path,
            &["--version"],
            &[("WG_LAUNCHER_NO_UPDATE_CHECK", "1")],
        ));
        assert_eq!(
            output.trim(),
            format!(
                "wg-launcher\nInstalled:    {}\nLatest:       unavailable\nChangelog:    https://github.com/gabrielhamel/wg-launcher/releases/latest",
                env!("CARGO_PKG_VERSION")
            )
        );
    }
    assert!(!sandbox.game().exists());
}

#[test]
fn checksum_and_embedded_version_failures_preserve_the_installed_cli() {
    for (corrupt, bad_version, diagnostic) in [
        (true, false, "checksum mismatch"),
        (false, true, "failed its version check"),
    ] {
        let sandbox = Sandbox::new();
        let server = server(&sandbox, cli_update::FUTURE, corrupt, bad_version);
        let manifest = format!("{}/dist-manifest.json", server.url());
        let launcher = sandbox.path.join("standalone").join(executable_name());
        copy_launcher(&launcher);
        let before = fs::read(&launcher).unwrap();
        failure(
            sandbox.run_cli(
                &launcher,
                &sandbox.path,
                &["self-update"],
                &[("WG_LAUNCHER_UPDATE_MANIFEST", &manifest)],
            ),
            diagnostic,
        );
        assert_eq!(fs::read(launcher).unwrap(), before);
    }
}

#[test]
fn package_manager_installations_refuse_replacement_even_when_newer_is_available() {
    let sandbox = Sandbox::new();
    let server = server(&sandbox, cli_update::FUTURE, false, false);
    let manifest = format!("{}/dist-manifest.json", server.url());
    for (directory, manager) in [
        ("Cellar/wg-launcher/current/bin", "Homebrew"),
        ("Microsoft/WinGet/Packages/example", "WinGet"),
        ("scoop/apps/wg-launcher/current", "Scoop"),
        ("nix/store/wg-launcher/bin", "Nix"),
    ] {
        let launcher = sandbox.path.join(directory).join(executable_name());
        copy_launcher(&launcher);
        let before = fs::read(&launcher).unwrap();
        let output = sandbox.run_cli(
            &launcher,
            &sandbox.path,
            &["--version"],
            &[("WG_LAUNCHER_UPDATE_MANIFEST", &manifest)],
        );
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Self-update not supported"));
        failure(
            sandbox.run_cli(
                &launcher,
                &sandbox.path,
                &["self-update"],
                &[("WG_LAUNCHER_UPDATE_MANIFEST", &manifest)],
            ),
            manager,
        );
        assert_eq!(fs::read(launcher).unwrap(), before);
    }
    let prefix = sandbox.path.join("custom-homebrew-prefix");
    let launcher = prefix.join("bin").join(executable_name());
    copy_launcher(&launcher);
    fs::write(
        prefix.join("INSTALL_RECEIPT.json"),
        br#"{"homebrew_version":"test"}"#,
    )
    .unwrap();
    failure(
        sandbox.run_cli(
            &launcher,
            &sandbox.path,
            &["self-update"],
            &[("WG_LAUNCHER_UPDATE_MANIFEST", &manifest)],
        ),
        "Homebrew",
    );
    assert!(
        !server
            .requests()
            .iter()
            .any(|request| request.path.ends_with(".sha256"))
    );
}

#[test]
fn equal_or_older_releases_do_not_download_and_offline_checks_do_not_break_commands() {
    let sandbox = Sandbox::new();
    let launcher = sandbox.path.join("standalone").join(executable_name());
    copy_launcher(&launcher);
    let server = server(&sandbox, env!("CARGO_PKG_VERSION"), false, false);
    let manifest = format!("{}/dist-manifest.json", server.url());
    let output = sandbox.run_cli(
        &launcher,
        &sandbox.path,
        &["self-update"],
        &[("WG_LAUNCHER_UPDATE_MANIFEST", &manifest)],
    );
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("already up to date"));
    assert!(
        !server
            .requests()
            .iter()
            .any(|request| request.path.ends_with(".sha256"))
    );
    let old = cli_update::resources(
        sandbox.fixture(Release::V1).service_resources("updates"),
        "0.0.1",
        false,
        false,
    );
    server.replace_resources(old);
    success(sandbox.run_cli(
        &launcher,
        &sandbox.path,
        &["self-update"],
        &[("WG_LAUNCHER_UPDATE_MANIFEST", &manifest)],
    ));
    let output = sandbox.run_cli(
        &launcher,
        &sandbox.path,
        &["--version"],
        &[(
            "WG_LAUNCHER_UPDATE_MANIFEST",
            "http://127.0.0.1:0/dist-manifest.json",
        )],
    );
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Latest:       unavailable"));
    failure(
        sandbox.run_cli(
            &launcher,
            &sandbox.path,
            &["status"],
            &[(
                "WG_LAUNCHER_UPDATE_MANIFEST",
                "http://127.0.0.1:0/dist-manifest.json",
            )],
        ),
        "error:",
    );
    let before = server.requests();
    success(sandbox.run_cli(
        &launcher,
        &sandbox.path,
        &["--version"],
        &[
            ("WG_LAUNCHER_UPDATE_MANIFEST", &manifest),
            ("WG_LAUNCHER_NO_UPDATE_CHECK", "1"),
        ],
    ));
    assert_eq!(server.requests(), before);
}
