use super::Recovery;
use crate::{
    fixture::{
        DeltaFixture, Release,
        recovery::{FILE_COUNT, archive, filename, transfer_payload},
    },
    server::WgUpdateService,
    support::{Sandbox, success},
};
use std::{fs, path::Path};

fn size(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

#[test]
fn killed_concurrent_downloads_resume_and_release_the_installation_lock() {
    interrupted_transfer(false);
}

#[test]
fn service_loss_during_transfer_recovers_on_the_same_address() {
    interrupted_transfer(true);
}

fn interrupted_transfer(stop_service: bool) {
    let sandbox = Sandbox::new();
    let mut fixture = sandbox.fixture(Release::V1);
    for (part, name) in [("client", "payload.pkg"), ("locale", "locale.txt")] {
        fixture.packages.insert(
            format!("{part}.wgpkg"),
            archive(
                &sandbox.path.join(part),
                [(name.into(), transfer_payload())],
            ),
        );
    }
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
    let cache = game.join(".wg-launcher/download");
    let partial = cache.join("client.wgpkg.part");
    let mut child = sandbox.spawn(&sandbox.path, &args);
    child.until(|| {
        size(&partial) > 0 && size(&partial) < fixture.packages["client.wgpkg"].len() as u64
    });
    if stop_service {
        server.stop();
    } else {
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(game.join(".wg-launcher/wg-launcher.lock"))
            .unwrap();
        assert!(
            fs2::FileExt::try_lock_exclusive(&lock).is_err(),
            "download must own the installation lock"
        );
    }
    // Stop the launcher as well: this leaves a stable durable prefix for assertions.
    let output = child.kill();
    assert!(!output.status.success());
    let prefix = size(&partial);
    assert!(
        prefix > 0 && prefix < fixture.packages["client.wgpkg"].len() as u64,
        "missed in-flight transfer: {prefix}"
    );
    assert!(!game.join(".wg-launcher/state.json").exists());
    if stop_service {
        server.replace_resources(fixture.service_resources("updates"));
    }
    let before = server.requests().len();
    success(sandbox.run(&sandbox.path, &args));
    let requests = server.requests();
    assert!(
        requests[before..]
            .iter()
            .any(|r| r.path == "/client.wgpkg" && r.range == Some(format!("bytes={prefix}-")))
    );
    assert!(
        requests[before..]
            .iter()
            .all(|r| r.path.ends_with(".wgpkg")),
        "retry performed discovery"
    );
    for (name, bytes) in &fixture.packages {
        assert_eq!(fs::read(cache.join(name)).unwrap(), *bytes);
    }
    server.stop();
    success(sandbox.run(&game, &["extract"]));
    for name in ["payload.pkg", "locale.txt"] {
        assert_eq!(fs::read(game.join(name)).unwrap(), transfer_payload());
    }
    assert!(!cache.exists());
    assert!(!game.join(".wg-launcher/installing").exists());
}

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
fn killed_delta_patch_preserves_original_sources_and_retries_pinned_update() {
    let recovery = Recovery::downloaded();
    success(recovery.run("install"));
    for i in 0..FILE_COUNT {
        let target = recovery.game.join(filename(i));
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, Release::V1.payload()).unwrap();
    }
    let mut delta = DeltaFixture::build(
        &recovery.fixture,
        &recovery.sandbox.path.join("delta"),
        false,
    );
    let bytes = vec![
        0x72,
        0x73,
        0x02,
        0x36,
        0x45,
        1,
        (Release::V1.payload().len() - 1) as u8,
        1,
        b'!',
        0,
    ];
    let transport = archive(
        &recovery.sandbox.path.join("many-deltas"),
        (0..FILE_COUNT).map(|i| (format!("{}.1.deadbeef.rdiff", filename(i)), bytes.clone())),
    );
    let old_size = delta.resources["delta.wgpkg"].len();
    let chain = String::from_utf8(delta.resources["updates/api/v1/patches_chain/"].clone())
        .unwrap()
        .replace(
            &format!("<size>{old_size}</size>"),
            &format!("<size>{}</size>", transport.len()),
        );
    delta
        .resources
        .insert("updates/api/v1/patches_chain/".into(), chain.into_bytes());
    delta.resources.insert("delta.wgpkg".into(), transport);
    recovery.server.replace_resources(delta.resources);
    let mut child = recovery.sandbox.spawn(&recovery.game, &["update"]);
    child.until(|| {
        fs::read(recovery.game.join(filename(0))).is_ok_and(|bytes| bytes == delta.payload)
    });
    assert!(!child.kill().status.success());
    assert_eq!(
        fs::read(recovery.game.join(filename(FILE_COUNT - 1))).unwrap(),
        Release::V1.payload(),
        "missed delta phase"
    );
    assert_eq!(
        fs::read(
            recovery
                .game
                .join(".wg-launcher/installing/active/sources")
                .join(filename(0))
        )
        .unwrap(),
        Release::V1.payload()
    );
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(recovery.game.join(".wg-launcher/state.json")).unwrap())
            .unwrap();
    assert_eq!(state["version"], "1.0");
    recovery
        .server
        .replace_resources(recovery.fixture.service_resources("updates"));
    let before = recovery.server.requests();
    success(recovery.sandbox.run(&recovery.game, &["update"]));
    assert_eq!(recovery.server.requests(), before);
    for i in 0..FILE_COUNT {
        assert_eq!(
            fs::read(recovery.game.join(filename(i))).unwrap(),
            delta.payload
        );
    }
    assert!(!recovery.cache().exists());
    assert!(!recovery.game.join(".wg-launcher/installing").exists());
}
