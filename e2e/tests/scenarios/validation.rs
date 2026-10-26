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
