use crate::support::{Sandbox, assert_installed, failure, success};
use crate::{fixture::Release, server::WgUpdateService};
use std::fs;

#[test]
fn staged_download_reuses_cache_and_extracts_offline_from_current_directory() {
    let sandbox = Sandbox::new();
    let fixture = sandbox.fixture(Release::V1);
    let server = WgUpdateService::start(fixture.service_resources("updates"));
    let config = sandbox.configuration(&server);
    let game = sandbox.game();
    let args = [
        "download",
        "--game",
        "wot",
        "--product",
        config.to_str().unwrap(),
        "--region",
        "local",
        "--output",
        game.to_str().unwrap(),
    ];

    success(sandbox.run(&sandbox.path, &args));
    for (name, bytes) in &fixture.packages {
        assert_eq!(
            fs::read(game.join(".wg-launcher/download").join(name)).unwrap(),
            *bytes
        );
    }
    assert!(!game.join("payload.pkg").exists());
    assert!(!game.join(".wg-launcher/state.json").exists());
    let before_retry = server.requests();
    assert_eq!(before_retry.len(), 5);
    success(sandbox.run(&sandbox.path, &args));
    assert_eq!(
        server.requests(),
        before_retry,
        "pinned discovery and complete volumes must be reused"
    );
    fs::remove_file(config).unwrap();
    drop(server);

    success(sandbox.run(&game, &["extract"]));
    assert_installed(&game, Release::V1);
}

#[test]
fn filtered_download_preserves_manifest_and_extraction_requires_remaining_packages() {
    let sandbox = Sandbox::new();
    let fixture = sandbox.fixture(Release::V1);
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
            "--part",
            "client",
        ],
    ));
    let requests = server.requests();
    assert_eq!(requests.len(), 4);
    assert_eq!(requests[3].path, "/client.wgpkg");
    let manifest_path = game.join(".wg-launcher/download/manifest.json");
    let pinned = fs::read(&manifest_path).unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&pinned).unwrap();
    assert_eq!(manifest["parts"], serde_json::json!(["client", "locale"]));
    assert_eq!(manifest["assets"].as_array().unwrap().len(), 2);
    assert_eq!(manifest["product"]["game_id"], "LOCAL.WOT");

    failure(sandbox.run(&game, &["extract"]), "locale.wgpkg");
    assert!(!game.join("payload.pkg").exists());
    assert!(!game.join(".wg-launcher/state.json").exists());
    assert!(game.join(".wg-launcher/download/client.wgpkg").exists());
    assert_eq!(fs::read(&manifest_path).unwrap(), pinned);
    assert_eq!(server.requests(), requests);

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
    let requests = server.requests();
    assert_eq!(requests.len(), 5);
    assert_eq!(requests[4].path, "/locale.wgpkg");
    assert_eq!(fs::read(&manifest_path).unwrap(), pinned);
    success(sandbox.run(&game, &["extract"]));
    assert_installed(&game, Release::V1);
}
