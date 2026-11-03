use super::Recovery;
use crate::{
    fixture::{DeltaFixture, Release, recovery::archive},
    support::{failure, success},
};
use std::fs;

#[test]
fn invalid_delta_preserves_original_and_recovers_with_repaired_cached_volume() {
    let recovery = Recovery::downloaded();
    success(recovery.run("install"));
    let mut delta = DeltaFixture::build(
        &recovery.fixture,
        &recovery.sandbox.path.join("delta"),
        false,
    );
    let valid = delta.resources["delta.wgpkg"].clone();
    let mut invalid = archive(
        &recovery.sandbox.path.join("invalid"),
        [(
            "payload.pkg.1.deadbeef.rdiff".into(),
            b"invalid librsync delta".to_vec(),
        )],
    );
    let size = invalid.len().max(valid.len()) + 64;
    invalid.resize(size, 0);
    let chain = String::from_utf8(delta.resources["updates/api/v1/patches_chain/"].clone())
        .unwrap()
        .replace(
            &format!("<size>{}</size>", valid.len()),
            &format!("<size>{size}</size>"),
        );
    delta
        .resources
        .insert("updates/api/v1/patches_chain/".into(), chain.into_bytes());
    delta.resources.insert("delta.wgpkg".into(), invalid);
    recovery.server.replace_resources(delta.resources);
    failure(
        recovery.sandbox.run(&recovery.game, &["update"]),
        "librsync",
    );
    assert_eq!(
        fs::read(recovery.game.join("payload.pkg")).unwrap(),
        Release::V1.payload()
    );
    assert_eq!(
        fs::read(
            recovery
                .game
                .join(".wg-launcher/installing/active/sources/payload.pkg")
        )
        .unwrap(),
        Release::V1.payload()
    );
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(recovery.game.join(".wg-launcher/state.json")).unwrap())
            .unwrap();
    assert_eq!(state["version"], "1.0");
    assert!(recovery.cache().join("delta.wgpkg").is_file());
    let mut repaired = valid;
    repaired.resize(size, 0);
    fs::write(recovery.cache().join("delta.wgpkg"), repaired).unwrap();
    let before = recovery.server.requests();
    success(recovery.sandbox.run(&recovery.game, &["update"]));
    assert_eq!(recovery.server.requests(), before);
    assert_eq!(
        fs::read(recovery.game.join("payload.pkg")).unwrap(),
        delta.payload
    );
    assert!(!recovery.cache().exists());
    assert!(!recovery.game.join(".wg-launcher/installing").exists());
}
