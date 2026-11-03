use super::*;

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
    let newer = recovery.sandbox.fixture(Release::V2);
    let mut newer_resources = newer.service_resources("updates");
    let chain = String::from_utf8(newer_resources["updates/api/v1/patches_chain/"].clone())
        .unwrap()
        .replace("2.0", "3.0");
    newer_resources.insert("updates/api/v1/patches_chain/".into(), chain.into_bytes());
    recovery.server.replace_resources(newer_resources);
    let before = recovery.server.requests();
    success(recovery.sandbox.run(&recovery.game, &["update"]));
    assert_eq!(recovery.server.requests(), before);
    for i in 0..FILE_COUNT {
        assert_eq!(
            fs::read(recovery.game.join(filename(i))).unwrap(),
            delta.payload
        );
    }
    assert_state(&recovery.game, "2.0", "1.0");
    assert!(!recovery.cache().exists());
    assert!(!recovery.game.join(".wg-launcher/installing").exists());
}
