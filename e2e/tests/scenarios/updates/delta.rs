use crate::{
    fixture::{DeltaFixture, Fixture, Release},
    server::WgUpdateService,
    support::{Sandbox, failure, success},
};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn installed(sandbox: &Sandbox) -> (PathBuf, Fixture, WgUpdateService) {
    let initial = sandbox.fixture(Release::V1);
    let server = WgUpdateService::start(initial.service_resources("updates"));
    let config = sandbox.configuration(&server);
    let game = sandbox.game();
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
    let state = state(&game);
    assert_eq!(
        state["component_versions"],
        serde_json::json!({"client": "1.0", "locale": "1.0"})
    );
    (game, initial, server)
}

fn state(game: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(game.join(".wg-launcher/state.json")).unwrap()).unwrap()
}

#[test]
fn update_downloads_only_delta_and_preserves_unchanged_component() {
    let sandbox = Sandbox::new();
    let (game, initial, server) = installed(&sandbox);
    let delta = DeltaFixture::build(&initial, &sandbox.path.join("delta"), false);
    server.replace_resources(delta.resources.clone());
    let before = server.requests().len();
    success(sandbox.run(&game, &["update"]));
    assert_eq!(fs::read(game.join("payload.pkg")).unwrap(), delta.payload);
    assert_eq!(
        fs::read(game.join("locale.txt")).unwrap(),
        Release::V1.locale()
    );
    assert_eq!(state(&game)["version"], "2.0");
    assert_eq!(
        state(&game)["component_versions"],
        serde_json::json!({"client": "2.0", "locale": "1.0"})
    );
    let requests = server.requests();
    let requests = &requests[before..];
    let chain = requests
        .iter()
        .find(|r| r.path.ends_with("patches_chain/"))
        .unwrap();
    let query = chain.query.as_deref().unwrap();
    assert!(query.contains("client_current_version=1.0"), "{query}");
    assert!(query.contains("locale_current_version=1.0"), "{query}");
    let packages = requests
        .iter()
        .filter(|r| r.path.ends_with(".wgpkg"))
        .map(|r| r.path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(packages, ["/delta.wgpkg"]);
    assert!(!game.join(".wg-launcher/download").exists());
    assert!(!game.join(".wg-launcher/installing").exists());

    // Live WGUS omits both the chain and seeds when no packages are needed.
    let mut current = delta.resources;
    current.insert(
        "updates/api/v1/patches_chain/".into(),
        b"<protocol><version_name>2.0</version_name></protocol>".to_vec(),
    );
    server.replace_resources(current);
    let output = success(sandbox.run(&game, &["update"]));
    assert!(output.contains("Already up to date (2.0)"));
    assert_eq!(fs::read(game.join("payload.pkg")).unwrap(), delta.payload);
}

#[test]
fn retry_after_partial_delta_application_uses_original_sources_and_pinned_release() {
    let sandbox = Sandbox::new();
    let (game, initial, server) = installed(&sandbox);
    let delta = DeltaFixture::build(&initial, &sandbox.path.join("delta"), true);
    server.replace_resources(delta.resources.clone());
    failure(
        sandbox.run(&game, &["update"]),
        "delta target does not exist",
    );
    assert_eq!(fs::read(game.join("payload.pkg")).unwrap(), delta.payload);
    assert_eq!(state(&game)["version"], "1.0");
    let cache = game.join(".wg-launcher/download");
    assert!(cache.join("delta.wgpkg").exists());
    let pending: serde_json::Value =
        serde_json::from_slice(&fs::read(cache.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(pending["base_versions"]["client"], "1.0");

    // Repair the missing source, then republish an older catalog. The same delta
    // archive is retried without discovery or another transfer.
    fs::write(game.join("z.pkg"), Release::V1.payload()).unwrap();
    server.replace_resources(initial.service_resources("updates"));
    let before = server.requests();
    success(sandbox.run(&game, &["update"]));
    assert_eq!(server.requests(), before);
    assert_eq!(fs::read(game.join("payload.pkg")).unwrap(), delta.payload);
    assert_eq!(fs::read(game.join("z.pkg")).unwrap(), delta.payload);
    assert_eq!(state(&game)["component_versions"]["client"], "2.0");
    assert!(!cache.exists());
    assert!(!game.join(".wg-launcher/installing").exists());
}

#[test]
fn completed_patch_is_checkpointed_before_packages_are_released() {
    let sandbox = Sandbox::new();
    let (game, initial, server) = installed(&sandbox);
    let mut delta = DeltaFixture::build(&initial, &sandbox.path.join("delta"), false);
    let updated = sandbox.fixture(Release::V2);
    let locale = updated.packages["locale.wgpkg"].clone();
    let chain =
        String::from_utf8(delta.resources["updates/api/v1/patches_chain/"].clone()).unwrap();
    let patch = format!(
        "<patch><part>locale</part><version_from>1.0</version_from><version_to>2.0</version_to><files><file><name>locale-delta.wgpkg</name><size>{}</size></file></files></patch>",
        locale.len()
    );
    delta.resources.insert(
        "updates/api/v1/patches_chain/".into(),
        chain
            .replace("</patches_chain>", &format!("{patch}</patches_chain>"))
            .into_bytes(),
    );
    // Correct transfer size, invalid archive: client completes before locale fails.
    delta
        .resources
        .insert("locale-delta.wgpkg".into(), vec![0; locale.len()]);
    server.replace_resources(delta.resources);
    failure(
        sandbox.run(&game, &["update"]),
        "package archive extraction failed",
    );
    let cache = game.join(".wg-launcher/download");
    let pending: serde_json::Value =
        serde_json::from_slice(&fs::read(cache.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(pending["base_versions"]["client"], "2.0");
    assert_eq!(pending["base_versions"]["locale"], "1.0");
    assert!(!cache.join("delta.wgpkg").exists());
    assert_eq!(state(&game)["version"], "1.0");
    fs::write(cache.join("locale-delta.wgpkg"), locale).unwrap();
    server.replace_resources(initial.service_resources("updates"));
    let before = server.requests();
    success(sandbox.run(&game, &["update"]));
    assert_eq!(server.requests(), before);
    assert_eq!(fs::read(game.join("payload.pkg")).unwrap(), delta.payload);
    assert_eq!(
        fs::read(game.join("locale.txt")).unwrap(),
        Release::V2.locale()
    );
    assert_eq!(
        state(&game)["component_versions"],
        serde_json::json!({"client": "2.0", "locale": "2.0"})
    );
    assert!(!cache.exists());
}

#[test]
fn state_without_component_versions_is_rejected_without_discovery() {
    let sandbox = Sandbox::new();
    let (game, _, server) = installed(&sandbox);
    let mut old = state(&game);
    old.as_object_mut().unwrap().remove("component_versions");
    fs::write(
        game.join(".wg-launcher/state.json"),
        serde_json::to_vec(&old).unwrap(),
    )
    .unwrap();
    let before = server.requests();
    failure(
        sandbox.run(&game, &["update"]),
        "missing field `component_versions`",
    );
    assert_eq!(server.requests(), before);
}

#[test]
fn delta_only_response_infers_installed_source_and_keeps_unpublished_component() {
    let sandbox = Sandbox::new();
    let (game, initial, server) = installed(&sandbox);
    let mut delta = DeltaFixture::build(&initial, &sandbox.path.join("delta"), false);
    delta.use_delta_only_chain();
    server.replace_resources(delta.resources);
    success(sandbox.run(&game, &["update"]));
    assert_eq!(fs::read(game.join("payload.pkg")).unwrap(), delta.payload);
    assert_eq!(
        state(&game)["component_versions"],
        serde_json::json!({"client":"2.0", "locale":"1.0"})
    );
}

#[test]
fn missing_delta_path_fails_before_downloading_packages() {
    let sandbox = Sandbox::new();
    let (game, initial, server) = installed(&sandbox);
    let mut delta = DeltaFixture::build(&initial, &sandbox.path.join("delta"), false);
    delta.use_delta_only_chain();
    let xml = String::from_utf8(delta.resources["updates/api/v1/patches_chain/"].clone()).unwrap();
    delta.resources.insert(
        "updates/api/v1/patches_chain/".into(),
        xml.replace(
            "<version_to>",
            "<version_from>unavailable</version_from><version_to>",
        )
        .into_bytes(),
    );
    server.replace_resources(delta.resources);
    let before = server.requests().len();
    failure(
        sandbox.run(&game, &["update"]),
        "cannot update client from 1.0 to 2.0",
    );
    assert!(
        server.requests()[before..]
            .iter()
            .all(|request| !request.path.ends_with(".wgpkg"))
    );
    assert_eq!(
        fs::read(game.join("payload.pkg")).unwrap(),
        Release::V1.payload()
    );
    assert_eq!(state(&game)["version"], "1.0");
}

#[test]
fn changed_component_updates_even_when_the_build_label_is_unchanged() {
    let sandbox = Sandbox::new();
    let (game, initial, server) = installed(&sandbox);
    let mut delta = DeltaFixture::build(&initial, &sandbox.path.join("delta"), false);
    let xml = String::from_utf8(delta.resources["updates/api/v1/patches_chain/"].clone()).unwrap();
    delta.resources.insert(
        "updates/api/v1/patches_chain/".into(),
        xml.replace(
            "<version_name>2.0</version_name>",
            "<version_name>1.0</version_name>",
        )
        .into_bytes(),
    );
    server.replace_resources(delta.resources);
    let output = success(sandbox.run(&game, &["status"]));
    assert!(output.contains("Update available."));
    success(sandbox.run(&game, &["update"]));
    assert_eq!(fs::read(game.join("payload.pkg")).unwrap(), delta.payload);
    assert_eq!(state(&game)["version"], "1.0");
    assert_eq!(state(&game)["component_versions"]["client"], "2.0");
}

#[test]
fn build_identity_can_advance_without_downloading_unchanged_components() {
    let sandbox = Sandbox::new();
    let (game, initial, server) = installed(&sandbox);
    let mut resources = initial.service_resources("updates");
    resources.insert(
        "updates/api/v1/patches_chain/".into(),
        b"<protocol><version_name>2.0</version_name></protocol>".to_vec(),
    );
    server.replace_resources(resources);
    let before = server.requests().len();
    success(sandbox.run(&game, &["update"]));
    assert_eq!(state(&game)["version"], "2.0");
    assert_eq!(
        state(&game)["component_versions"],
        serde_json::json!({"client":"1.0", "locale":"1.0"})
    );
    assert!(
        server.requests()[before..]
            .iter()
            .all(|request| !request.path.ends_with(".wgpkg"))
    );
    assert_eq!(
        fs::read(game.join("payload.pkg")).unwrap(),
        Release::V1.payload()
    );
    assert!(!game.join(".wg-launcher/download").exists());
    assert!(!game.join(".wg-launcher/installing").exists());
}
