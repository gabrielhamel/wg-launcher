use crate::{
    fixture::{DeltaFixture, Fixture, Release},
    server::WgUpdateService,
    support::{Sandbox, assert_installed, success},
};
use std::fs;

#[test]
fn install_crosses_both_public_clis_and_the_http_boundary() {
    let sandbox = Sandbox::new();
    let fixture = sandbox.fixture(Release::V1);
    let service = WgUpdateService::start(fixture.service_resources("updates"));
    let config = sandbox.path.join("products.json");
    fs::write(&config, Fixture::products(&service.url())).unwrap();
    let game = sandbox.game();

    let listed = success(sandbox.run(
        &sandbox.path,
        &["list", "--product", config.to_str().unwrap()],
    ));
    assert!(listed.contains("1.0"));
    let inspected = success(sandbox.run(
        &sandbox.path,
        &[
            "inspect",
            "--game",
            "wot",
            "--product",
            config.to_str().unwrap(),
            "--region",
            "local",
        ],
    ));
    assert!(inspected.contains("client"));
    assert!(inspected.contains("locale"));
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
    let status = success(sandbox.run(&game, &["status"]));
    assert!(status.contains("Up to date."));
}

#[test]
fn full_patch_preserves_empty_directories_in_streaming_and_staged_installation() {
    for staged in [false, true] {
        let sandbox = Sandbox::new();
        let fixture = sandbox
            .fixture(Release::V1)
            .with_directories(&sandbox.path.join("directories"));
        let service = WgUpdateService::start(fixture.service_resources("updates"));
        let config = sandbox.configuration(&service);
        let game = sandbox.game();
        success(sandbox.run(
            &sandbox.path,
            &[
                if staged { "download" } else { "install" },
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
        if staged {
            drop(service);
            success(sandbox.run(&game, &["extract"]));
        }
        assert!(game.join("empty").is_dir());
        assert_eq!(
            fs::read(game.join("nested/payload.pkg")).unwrap(),
            Release::V1.payload()
        );
        assert_eq!(
            fs::read(game.join("locale.txt")).unwrap(),
            Release::V1.locale()
        );
        assert!(game.join(".wg-launcher/state.json").is_file());
        assert!(!game.join(".wg-launcher/download").exists());
        assert!(!game.join(".wg-launcher/installing").exists());
    }
}

#[test]
fn omitted_sources_keep_component_chains_and_packages_together() {
    for staged in [false, true] {
        let sandbox = Sandbox::new();
        let initial = sandbox.fixture(Release::V1);
        let mut delta = DeltaFixture::build(&initial, &sandbox.path.join("delta"), false);
        delta.omit_patch_sources();
        let service = WgUpdateService::start(delta.resources);
        let config = sandbox.configuration(&service);
        let game = sandbox.game();
        success(sandbox.run(
            &sandbox.path,
            &[
                if staged { "download" } else { "install" },
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
        if staged {
            let manifest: serde_json::Value = serde_json::from_slice(
                &fs::read(game.join(".wg-launcher/download/manifest.json")).unwrap(),
            )
            .unwrap();
            for collection in ["patches", "assets"] {
                let sources = manifest[collection]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|patch| {
                        (
                            patch["part"].as_str().unwrap(),
                            patch["version_from"].as_str().unwrap(),
                        )
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    sources,
                    [("client", "0"), ("client", "1.0"), ("locale", "0")]
                );
            }
            drop(service);
            success(sandbox.run(&game, &["extract"]));
        }
        assert_eq!(fs::read(game.join("payload.pkg")).unwrap(), delta.payload);
        assert_eq!(
            fs::read(game.join("locale.txt")).unwrap(),
            Release::V1.locale()
        );
        let state: serde_json::Value =
            serde_json::from_slice(&fs::read(game.join(".wg-launcher/state.json")).unwrap())
                .unwrap();
        assert_eq!(
            state["component_versions"],
            serde_json::json!({"client": "2.0", "locale": "1.0"})
        );
        assert!(!game.join(".wg-launcher/download").exists());
        assert!(!game.join(".wg-launcher/installing").exists());
    }
}
