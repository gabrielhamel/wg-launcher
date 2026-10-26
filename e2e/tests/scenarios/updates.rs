use crate::support::{Sandbox, assert_installed, success};
use crate::{fixture::Release, server::WgUpdateService};
use std::fs;

#[test]
fn pending_update_preserves_directory_and_selection_and_changes_installed_version() {
    let sandbox = Sandbox::new();
    let game = sandbox.game();
    let initial = sandbox.fixture(Release::V1);
    let server = WgUpdateService::start(initial.service_resources("updates"));
    let config = sandbox.configuration(&server);
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

    let updated = sandbox.fixture(Release::V2);
    server.replace_resources(updated.service_resources("updates"));
    let download = [
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
    ];
    success(sandbox.run(&sandbox.path, &download));
    let state_path = game.join(".wg-launcher/state.json");
    let state: serde_json::Value = serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    assert_eq!(
        state["version"], "1.0",
        "staging an update must not mark it installed"
    );
    // Retry must use the pinned v2 even if live discovery now advertises v1.
    let mut resources = initial.service_resources("updates");
    resources.extend(updated.packages.clone());
    server.replace_resources(resources);
    let before_update = server.requests().len();
    success(sandbox.run(&game, &["update"]));
    assert_installed(&game, Release::V2);
    let requests = server.requests();
    assert_eq!(requests.len(), before_update + 1);
    assert_eq!(requests[before_update].path, "/locale.wgpkg");

    // A CLI-produced pending manifest matching the installed version does no work.
    server.replace_resources(updated.service_resources("updates"));
    success(sandbox.run(&sandbox.path, &download));
    let before_current = server.requests();
    let state = fs::read(&state_path).unwrap();
    let output = success(sandbox.run(&sandbox.path, &["update", "--path", game.to_str().unwrap()]));
    assert!(output.contains("Already up to date (2.0)"));
    assert_eq!(fs::read(&state_path).unwrap(), state);
    assert_eq!(server.requests(), before_current);
}

#[test]
fn fresh_custom_installation_can_move_and_update_without_original_configuration() {
    let sandbox = Sandbox::new();
    let fixture = sandbox.fixture(Release::V1);
    let server = WgUpdateService::start(fixture.service_resources("updates"));
    let config = sandbox.configuration(&server);
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
        ],
    ));
    let initial = sandbox.path.join("wot-local-testing-1.0-windows-x64-sd-en");
    assert_installed(&initial, Release::V1);
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(initial.join(".wg-launcher/state.json")).unwrap())
            .unwrap();
    assert_eq!(state["game_id"], "LOCAL.WOT");
    assert_eq!(state["channel"], "testing");
    assert_eq!(
        state["product"]["host"],
        format!("{}/updates", server.url())
    );
    fs::remove_file(config).unwrap();
    let moved = sandbox.game();
    fs::rename(initial, &moved).unwrap();

    let output = success(sandbox.run(&moved, &["status"]));
    assert!(output.contains("Up to date."));
    let updated = sandbox.fixture(Release::V2);
    server.replace_resources(updated.service_resources("updates"));
    let output = success(sandbox.run(&moved, &["status"]));
    assert!(output.contains("Update available."));
    let before_update = server.requests().len();
    success(sandbox.run(&moved, &["update"]));
    assert_installed(&moved, Release::V2);
    assert!(
        !sandbox
            .path
            .join("wot-local-testing-2.0-windows-x64-sd-en")
            .exists()
    );
    assert_eq!(
        server.requests()[before_update..]
            .iter()
            .filter(|request| request.path.ends_with(".wgpkg"))
            .count(),
        2
    );
    let before_current = server.requests().len();
    let output = success(sandbox.run(&moved, &["update"]));
    assert!(output.contains("Already up to date (2.0)"));
    assert!(
        server.requests()[before_current..]
            .iter()
            .all(|request| !request.path.ends_with(".wgpkg"))
    );
}
