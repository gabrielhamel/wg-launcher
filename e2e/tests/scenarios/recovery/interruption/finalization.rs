use super::*;

fn fill_cleanup_directory(path: &Path) {
    fs::create_dir_all(path).unwrap();
    for i in 0..FILE_COUNT {
        fs::write(path.join(format!("{i:04}")), b"staging remainder").unwrap();
    }
}

#[test]
fn killed_after_checkpoint_before_package_release_retries_without_reapplying() {
    let recovery = Recovery::downloaded();
    let active = recovery.game.join(".wg-launcher/installing/active");
    // Obtain genuine active-patch identity through a failed CLI extraction.
    let blocker = recovery.game.join("payload.pkg");
    fs::create_dir(&blocker).unwrap();
    crate::support::failure(
        recovery.sandbox.run(&recovery.game, &["extract"]),
        "cannot replace",
    );
    fs::remove_dir(blocker).unwrap();
    // Slow only ordinary filesystem cleanup, without production fault hooks.
    fill_cleanup_directory(&active);
    let manifest_path = recovery.cache().join("manifest.json");
    let mut child = recovery.sandbox.spawn(&recovery.game, &["extract"]);
    child.until(|| {
        fs::read(&manifest_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .is_some_and(|m| m["base_versions"]["client"] == "1.0")
    });
    assert!(!child.kill().status.success());
    assert!(
        recovery.cache().join("client.wgpkg").is_file(),
        "missed checkpoint-before-release window"
    );
    assert!(!recovery.game.join(".wg-launcher/state.json").exists());
    let before = recovery.server.requests();
    success(recovery.sandbox.run(&recovery.game, &["extract"]));
    assert_eq!(recovery.server.requests(), before);
    crate::support::assert_installed(&recovery.game, Release::V1);
}

#[test]
fn killed_after_state_commit_finishes_cleanup_offline() {
    let recovery = Recovery::downloaded();
    fill_cleanup_directory(&recovery.game.join(".wg-launcher/installing/leftovers"));
    let marker = recovery.game.join(".wg-launcher/state.json");
    let mut child = recovery.sandbox.spawn(&recovery.game, &["extract"]);
    child.until(|| marker.is_file());
    assert!(!child.kill().status.success());
    assert!(
        recovery.cache().join("manifest.json").is_file(),
        "missed staging cleanup window"
    );
    recovery.server.stop();
    success(recovery.sandbox.run(&recovery.game, &["extract"]));
    crate::support::assert_installed(&recovery.game, Release::V1);
}
