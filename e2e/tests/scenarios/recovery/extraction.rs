use super::Recovery;
use crate::{
    fixture::Release,
    support::{assert_installed, failure, success},
};
use std::fs;

#[test]
fn blocked_destination_keeps_packages_and_recovers_offline() {
    let recovery = Recovery::downloaded();
    let blocked = recovery.game.join("payload.pkg");
    fs::create_dir(&blocked).unwrap();
    fs::write(blocked.join("user-file"), b"preserve me").unwrap();
    let before = recovery.server.requests();
    failure(
        recovery.sandbox.run(&recovery.game, &["extract"]),
        "cannot replace",
    );
    assert_eq!(fs::read(blocked.join("user-file")).unwrap(), b"preserve me");
    assert!(recovery.cache().join("client.wgpkg").is_file());
    assert!(!recovery.game.join(".wg-launcher/state.json").exists());
    fs::remove_dir_all(blocked).unwrap();
    success(recovery.sandbox.run(&recovery.game, &["extract"]));
    assert_eq!(recovery.server.requests(), before);
    assert_installed(&recovery.game, Release::V1);
}

#[test]
fn later_corrupt_archive_preserves_checkpoint_and_retries_offline() {
    let recovery = Recovery::downloaded();
    let cache = recovery.cache();
    let locale = &recovery.fixture.packages["locale.wgpkg"];
    fs::write(cache.join("locale.wgpkg"), vec![0; locale.len()]).unwrap();
    failure(
        recovery.sandbox.run(&recovery.game, &["extract"]),
        "archive",
    );
    let pinned: serde_json::Value =
        serde_json::from_slice(&fs::read(cache.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(pinned["base_versions"]["client"], "1.0");
    assert!(!cache.join("client.wgpkg").exists());
    assert_eq!(
        fs::read(recovery.game.join("payload.pkg")).unwrap(),
        Release::V1.payload()
    );
    assert!(!recovery.game.join(".wg-launcher/state.json").exists());
    fs::write(cache.join("locale.wgpkg"), locale).unwrap();
    let before = recovery.server.requests();
    let output = success(recovery.sandbox.run(&recovery.game, &["extract"]));
    assert!(
        output.lines().next().unwrap().contains(": 1 assets,"),
        "{output}"
    );
    assert_eq!(recovery.server.requests(), before);
    assert_installed(&recovery.game, Release::V1);
}

#[test]
fn blocked_state_write_retries_without_reapplying_completed_patches() {
    let recovery = Recovery::downloaded();
    let marker_part = recovery.game.join(".wg-launcher/state.json.part");
    fs::create_dir(&marker_part).unwrap();
    failure(recovery.sandbox.run(&recovery.game, &["extract"]), "");
    assert!(!recovery.game.join(".wg-launcher/state.json").exists());
    let cache = recovery.cache();
    let pinned: serde_json::Value =
        serde_json::from_slice(&fs::read(cache.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(pinned["base_versions"], pinned["component_versions"]);
    assert!(!cache.join("client.wgpkg").exists());
    assert!(!cache.join("locale.wgpkg").exists());
    fs::remove_dir(marker_part).unwrap();
    let before = recovery.server.requests();
    let output = success(recovery.run("install"));
    assert!(
        output
            .lines()
            .next()
            .unwrap()
            .contains(": 0 assets, 0 B ->"),
        "{output}"
    );
    assert_eq!(recovery.server.requests(), before);
    assert_installed(&recovery.game, Release::V1);
}

#[test]
fn state_destination_directory_is_preserved_on_failure() {
    let recovery = Recovery::downloaded();
    let marker = recovery.game.join(".wg-launcher/state.json");
    fs::create_dir(&marker).unwrap();
    fs::write(marker.join("user-file"), b"preserve me").unwrap();
    failure(recovery.sandbox.run(&recovery.game, &["extract"]), "");
    assert_eq!(fs::read(marker.join("user-file")).unwrap(), b"preserve me");
    fs::remove_dir_all(marker).unwrap();
    success(recovery.sandbox.run(&recovery.game, &["extract"]));
    assert_installed(&recovery.game, Release::V1);
}

#[test]
fn interrupted_installation_can_move_and_finish_offline_without_configuration() {
    let mut recovery = Recovery::downloaded();
    let locale = &recovery.fixture.packages["locale.wgpkg"];
    fs::write(recovery.cache().join("locale.wgpkg"), vec![0; locale.len()]).unwrap();
    failure(
        recovery.sandbox.run(&recovery.game, &["extract"]),
        "archive",
    );
    fs::write(recovery.cache().join("locale.wgpkg"), locale).unwrap();
    let moved = recovery.sandbox.path.join("moved pending installation");
    fs::rename(&recovery.game, &moved).unwrap();
    recovery.game = moved;
    fs::remove_file(&recovery.config).unwrap();
    drop(recovery.server);
    success(recovery.sandbox.run(&recovery.game, &["extract"]));
    assert_installed(&recovery.game, Release::V1);
}

#[test]
fn download_refuses_an_installation_owned_by_another_process() {
    let recovery = Recovery::downloaded();
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(recovery.game.join(".wg-launcher/wg-launcher.lock"))
        .unwrap();
    fs2::FileExt::try_lock_exclusive(&lock).unwrap();
    let pinned = fs::read(recovery.cache().join("manifest.json")).unwrap();
    let before = recovery.server.requests();
    failure(
        recovery.run("download"),
        "another installation is already running",
    );
    assert_eq!(
        fs::read(recovery.cache().join("manifest.json")).unwrap(),
        pinned
    );
    assert_eq!(recovery.server.requests(), before);
    drop(lock);
    success(recovery.run("install"));
    assert_installed(&recovery.game, Release::V1);
}

#[cfg(unix)]
#[test]
fn cleanup_permission_failure_retains_committed_identity_and_can_be_retried() {
    use std::os::unix::fs::PermissionsExt;
    struct Restore(std::path::PathBuf);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o700));
        }
    }
    let recovery = Recovery::downloaded();
    let blocked = recovery.game.join(".wg-launcher/installing/leftovers");
    fs::create_dir_all(&blocked).unwrap();
    fs::write(blocked.join("temporary"), b"staging data").unwrap();
    let restore = Restore(blocked.clone());
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o500)).unwrap();
    let output = recovery.sandbox.run(&recovery.game, &["extract"]);
    // Restore permissions before assertions and Sandbox cleanup, including on panic.
    drop(restore);
    failure(output, "Permission denied");
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(recovery.game.join(".wg-launcher/state.json")).unwrap())
            .unwrap();
    assert_eq!(state["version"], "1.0");
    assert!(recovery.cache().join("manifest.json").is_file());
    let before = recovery.server.requests();
    success(recovery.sandbox.run(&recovery.game, &["extract"]));
    assert_eq!(recovery.server.requests(), before);
    assert_installed(&recovery.game, Release::V1);
}

#[test]
fn damaged_archive_stream_does_not_truncate_an_existing_game_file() {
    let recovery = Recovery::downloaded();
    let path = recovery.cache().join("client.wgpkg");
    let valid = fs::read(&path).unwrap();
    let mut damaged = valid.clone();
    // Keep the 7z signature and headers intact; damage the packed data stream.
    damaged[40] ^= 0xff;
    fs::write(&path, damaged).unwrap();
    fs::write(recovery.game.join("payload.pkg"), b"existing game bytes").unwrap();
    failure(
        recovery.sandbox.run(&recovery.game, &["extract"]),
        "archive",
    );
    assert_eq!(
        fs::read(recovery.game.join("payload.pkg")).unwrap(),
        b"existing game bytes"
    );
    assert!(path.is_file());
    assert!(!recovery.game.join(".wg-launcher/state.json").exists());
    fs::write(path, valid).unwrap();
    success(recovery.sandbox.run(&recovery.game, &["extract"]));
    assert_installed(&recovery.game, Release::V1);
}
