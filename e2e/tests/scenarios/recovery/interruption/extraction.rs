use super::*;

#[test]
fn killed_full_extraction_replays_the_uncheckpointed_patch_offline() {
    let sandbox = Sandbox::new();
    let mut fixture = sandbox.fixture(Release::V1);
    fixture.packages.insert(
        "client.wgpkg".into(),
        archive(
            &sandbox.path.join("many"),
            (0..FILE_COUNT).map(|i| (filename(i), vec![i as u8; 1024])),
        ),
    );
    let server = WgUpdateService::start(fixture.service_resources("updates"));
    let config = sandbox.configuration(&server);
    let game = sandbox.game();
    success(sandbox.run(
        &sandbox.path,
        &[
            "download",
            "--game",
            "wot",
            "--product",
            config.to_str().unwrap(),
            "--region",
            "local",
            "--output",
            game.to_str().unwrap(),
        ],
    ));
    let mut child = sandbox.spawn(&game, &["extract"]);
    child.until(|| game.join(filename(0)).is_file());
    assert!(!child.kill().status.success());
    assert!(
        !game.join(filename(FILE_COUNT - 1)).exists(),
        "missed extraction phase"
    );
    assert!(!game.join(".wg-launcher/state.json").exists());
    assert!(game.join(".wg-launcher/download/client.wgpkg").is_file());
    server.stop();
    success(sandbox.run(&game, &["extract"]));
    for i in 0..FILE_COUNT {
        assert_eq!(
            fs::read(game.join(filename(i))).unwrap(),
            vec![i as u8; 1024]
        );
    }
    assert!(!game.join(".wg-launcher/download").exists());
    assert!(!game.join(".wg-launcher/installing").exists());
}

#[test]
fn failed_checkpoint_after_extraction_keeps_packages_for_replay() {
    let sandbox = Sandbox::new();
    let mut fixture = sandbox.fixture(Release::V1);
    fixture.packages.insert(
        "client.wgpkg".into(),
        archive(
            &sandbox.path.join("many"),
            (0..FILE_COUNT).map(|i| (filename(i), vec![i as u8; 1024])),
        ),
    );
    let server = WgUpdateService::start(fixture.service_resources("updates"));
    let config = sandbox.configuration(&server);
    let game = sandbox.game();
    success(sandbox.run(
        &sandbox.path,
        &[
            "download",
            "--game",
            "wot",
            "--product",
            config.to_str().unwrap(),
            "--region",
            "local",
            "--output",
            game.to_str().unwrap(),
        ],
    ));
    let cache = game.join(".wg-launcher/download");
    let mut child = sandbox.spawn(&game, &["extract"]);
    child.until(|| game.join(filename(0)).is_file());
    // Initial manifest persistence has completed; block the next checkpoint write.
    let blocker = cache.join("manifest.json.part");
    fs::create_dir(&blocker).unwrap();
    crate::support::failure(child.wait(), "");
    assert!(game.join(filename(FILE_COUNT - 1)).is_file());
    let pinned: serde_json::Value =
        serde_json::from_slice(&fs::read(cache.join("manifest.json")).unwrap()).unwrap();
    assert_ne!(pinned["base_versions"]["client"], "1.0");
    assert!(cache.join("client.wgpkg").is_file());
    assert!(!game.join(".wg-launcher/state.json").exists());
    fs::remove_dir(blocker).unwrap();
    server.stop();
    success(sandbox.run(&game, &["extract"]));
    for i in 0..FILE_COUNT {
        assert_eq!(
            fs::read(game.join(filename(i))).unwrap(),
            vec![i as u8; 1024]
        );
    }
    assert!(!cache.exists());
}

#[test]
fn killed_while_decoding_a_full_file_preserves_the_existing_destination() {
    let sandbox = Sandbox::new();
    let mut fixture = sandbox.fixture(Release::V1);
    let expected = transfer_payload();
    fixture.packages.insert(
        "client.wgpkg".into(),
        archive(
            &sandbox.path.join("large"),
            [("payload.pkg".into(), expected.clone())],
        ),
    );
    let server = WgUpdateService::start(fixture.service_resources("updates"));
    let config = sandbox.configuration(&server);
    let game = sandbox.game();
    success(sandbox.run(
        &sandbox.path,
        &[
            "download",
            "--game",
            "wot",
            "--product",
            config.to_str().unwrap(),
            "--region",
            "local",
            "--output",
            game.to_str().unwrap(),
        ],
    ));
    fs::write(game.join("payload.pkg"), b"existing game bytes").unwrap();
    let staged = game.join(".wg-launcher/installing/active/archive/payload.pkg");
    let mut child = sandbox.spawn(&game, &["extract"]);
    child.until(|| size(&staged) > 0 && size(&staged) < expected.len() as u64);
    assert!(!child.kill().status.success());
    assert_eq!(
        fs::read(game.join("payload.pkg")).unwrap(),
        b"existing game bytes"
    );
    assert!(!game.join(".wg-launcher/state.json").exists());
    assert!(game.join(".wg-launcher/download/client.wgpkg").is_file());
    server.stop();
    success(sandbox.run(&game, &["extract"]));
    assert_eq!(fs::read(game.join("payload.pkg")).unwrap(), expected);
    assert_state(&game, "1.0", "1.0");
    assert!(!game.join(".wg-launcher/download").exists());
    assert!(!game.join(".wg-launcher/installing").exists());
}
