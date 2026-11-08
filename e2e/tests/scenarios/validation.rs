use crate::support::{Sandbox, assert_installed, failure, success};
use crate::{
    fixture::{Fixture, Release},
    server::WgUpdateService,
};
use std::fs;

#[test]
fn another_process_cannot_extract_while_installation_lock_is_held() {
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
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(game.join(".wg-launcher/wg-launcher.lock"))
        .unwrap();
    fs2::FileExt::try_lock_exclusive(&lock).unwrap();

    failure(
        sandbox.run(&game, &["extract"]),
        "another installation is already running",
    );
    assert!(!game.join("payload.pkg").exists());
    assert!(!game.join(".wg-launcher/state.json").exists());
    assert!(game.join(".wg-launcher/download/client.wgpkg").exists());
    drop(lock);

    success(sandbox.run(&game, &["extract"]));
    assert_installed(&game, Release::V1);
    assert_eq!(server.requests(), before_extract);
}

#[test]
fn invalid_catalogs_and_missing_regions_fail_before_requests() {
    let sandbox = Sandbox::new();
    let fixture = sandbox.fixture(Release::V1);
    let server = WgUpdateService::start(fixture.service_resources("updates"));
    let config = sandbox.configuration(&server);
    failure(sandbox.run(&sandbox.path, &["install"]), "--game <GAME>");
    for command in ["list", "inspect", "download", "install"] {
        failure(
            sandbox.run(
                &sandbox.path,
                &[
                    command,
                    "--game",
                    "wot",
                    "--product",
                    config.to_str().unwrap(),
                    "--region",
                    "missing",
                ],
            ),
            "no product configured for region 'missing'",
        );
    }
    // Even a single-entry custom catalog uses the ordinary EU selection default.
    failure(
        sandbox.run(
            &sandbox.path,
            &[
                "inspect",
                "--game",
                "wot",
                "--product",
                config.to_str().unwrap(),
            ],
        ),
        "no product configured for region 'eu'",
    );
    for (json, message) in [
        ("[]".to_owned(), "must not be empty"),
        (Fixture::product(&server.url()), "expected a sequence"),
        (Fixture::products("file:///tmp"), "invalid product"),
        (
            format!(
                "[{},{}]",
                Fixture::product(&server.url()),
                Fixture::product(&server.url())
            ),
            "duplicate region",
        ),
    ] {
        fs::write(&config, json).unwrap();
        failure(
            sandbox.run(
                &sandbox.path,
                &["list", "--product", config.to_str().unwrap()],
            ),
            message,
        );
    }
    assert!(server.requests().is_empty());
}

#[test]
fn inconsistent_saved_identity_fails_before_live_or_pinned_update_discovery() {
    let sandbox = Sandbox::new();
    let fixture = sandbox.fixture(Release::V1);
    let server = WgUpdateService::start(fixture.service_resources("updates"));
    let config = sandbox.configuration(&server);
    let game = sandbox.game();
    let selection = [
        "--game",
        "wot",
        "--product",
        config.to_str().unwrap(),
        "--region",
        "local",
        "--output",
        game.to_str().unwrap(),
    ];
    let mut install = vec!["install"];
    install.extend(selection);
    success(sandbox.run(&sandbox.path, &install));
    let state_path = game.join(".wg-launcher/state.json");
    let original = fs::read(&state_path).unwrap();
    // Identity refactoring must not turn required persisted fields into defaults.
    for field in [
        "game_id",
        "region",
        "channel",
        "version",
        "client_type",
        "language",
        "platform",
        "metadata_version",
    ] {
        let mut state: serde_json::Value = serde_json::from_slice(&original).unwrap();
        state.as_object_mut().unwrap().remove(field);
        let saved = serde_json::to_vec_pretty(&state).unwrap();
        fs::write(&state_path, &saved).unwrap();
        let before = server.requests();
        for command in ["status", "update"] {
            failure(
                sandbox.run(&game, &[command]),
                &format!("missing field `{field}`"),
            );
            assert_eq!(server.requests(), before);
            assert_eq!(fs::read(&state_path).unwrap(), saved);
        }
    }
    fs::write(&state_path, &original).unwrap();
    // Validate both live resolution and the path where an existing pin takes precedence.
    for pinned in [false, true] {
        if pinned {
            let mut download = vec!["download"];
            download.extend(selection);
            success(sandbox.run(&sandbox.path, &download));
        }
        for invalid_product in [false, true] {
            let mut state: serde_json::Value = serde_json::from_slice(&original).unwrap();
            let message = if invalid_product {
                state["product"]["game"] = "".into();
                "the saved installation product is invalid"
            } else {
                state["region"] = "another-region".into();
                "the saved installation region 'another-region' differs from its product region 'local'"
            };
            let saved = serde_json::to_vec_pretty(&state).unwrap();
            fs::write(&state_path, &saved).unwrap();
            let before = server.requests();
            for command in ["status", "update"] {
                failure(sandbox.run(&game, &[command]), message);
                assert_eq!(server.requests(), before);
                assert_eq!(fs::read(&state_path).unwrap(), saved);
                assert_eq!(
                    fs::read(game.join("payload.pkg")).unwrap(),
                    Release::V1.payload()
                );
            }
        }
        fs::write(&state_path, &original).unwrap();
    }
    let manifest_path = game.join(".wg-launcher/download/manifest.json");
    let pinned = fs::read(&manifest_path).unwrap();
    success(sandbox.run(&game, &["status"]));
    success(sandbox.run(&game, &["update"]));
    assert_eq!(fs::read(&state_path).unwrap(), original);
    assert_eq!(fs::read(&manifest_path).unwrap(), pinned);
    assert_eq!(
        fs::read(game.join("payload.pkg")).unwrap(),
        Release::V1.payload()
    );
    assert_eq!(
        fs::read(game.join("locale.txt")).unwrap(),
        Release::V1.locale()
    );
}
