use crate::support::{Sandbox, assert_installed, failure, success};
use crate::{fixture::Release, server::WgUpdateService};
use std::fs;

#[test]
fn retry_resumes_partial_volume_and_reuses_complete_volume() {
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
        ],
    ));
    let cache = game.join(".wg-launcher/download");
    let client = fs::read(cache.join("client.wgpkg")).unwrap();
    let prefix = client.len() / 2;
    // Retain the CLI-produced manifest and model a durable partial transfer.
    fs::remove_file(cache.join("client.wgpkg")).unwrap();
    fs::write(cache.join("client.wgpkg.part"), &client[..prefix]).unwrap();
    let before_retry = server.requests().len();
    // A newer catalog must not replace the pending manifest during retry.
    let updated = sandbox.fixture(Release::V2);
    let mut resources = updated.service_resources("updates");
    resources.extend(fixture.packages.clone());
    server.replace_resources(resources);

    success(sandbox.run(
        &sandbox.path,
        &[
            "install",
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

    assert_installed(&game, Release::V1);
    let requests = server.requests();
    assert_eq!(requests.len(), before_retry + 1);
    assert_eq!(requests[before_retry].path, "/client.wgpkg");
    assert_eq!(
        requests[before_retry].range,
        Some(format!("bytes={prefix}-"))
    );
}

#[test]
fn corrupt_archive_keeps_cache_and_existing_payload_and_can_be_retried() {
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
        ],
    ));
    let before_extract = server.requests();
    let cache = game.join(".wg-launcher/download");
    let client = fs::read(cache.join("client.wgpkg")).unwrap();
    fs::write(cache.join("client.wgpkg"), vec![0; client.len()]).unwrap();
    fs::write(game.join("payload.pkg"), b"existing game data").unwrap();

    failure(sandbox.run(&game, &["extract"]), "archive");
    assert_eq!(
        fs::read(game.join("payload.pkg")).unwrap(),
        b"existing game data"
    );
    assert!(!game.join(".wg-launcher/state.json").exists());
    for name in fixture.packages.keys() {
        assert!(
            cache.join(name).exists(),
            "failed patch must keep its packages"
        );
    }
    assert_eq!(server.requests(), before_extract);
    fs::write(cache.join("client.wgpkg"), client).unwrap();

    success(sandbox.run(&game, &["extract"]));
    assert_installed(&game, Release::V1);
    assert_eq!(server.requests(), before_extract);
}
