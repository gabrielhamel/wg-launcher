use crate::{
    fixture::{DeltaFixture, Fixture, Release, advertise_preload},
    server::WgUpdateService,
    support::{Sandbox, assert_installed, assert_installed_in_region, failure, success},
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
fn eu_workflows_use_the_install_chain_when_preload_is_advertised() {
    let sandbox = Sandbox::new();
    let initial = sandbox.fixture(Release::V1);
    let mut resources = initial.service_resources("updates");
    advertise_preload(&mut resources, "1.0");
    let service = WgUpdateService::start(resources);
    let config = sandbox.path.join("products.json");
    fs::write(
        &config,
        Fixture::products(&service.url()).replace(r#""region":"local""#, r#""region":"eu""#),
    )
    .unwrap();
    let selection = [
        "--game",
        "wot",
        "--region",
        "eu",
        "--product",
        config.to_str().unwrap(),
    ];
    for command in ["list", "inspect"] {
        let mut args = vec![command];
        args.extend(selection);
        let output = success(sandbox.run(&sandbox.path, &args));
        assert!(output.contains("1.0"), "{output}");
        assert!(!output.contains("3.0"), "{output}");
    }

    let game = sandbox.game();
    let mut args = vec!["install", "--output", game.to_str().unwrap()];
    args.extend(selection);
    success(sandbox.run(&sandbox.path, &args));
    assert_installed_in_region(&game, Release::V1, "eu");

    let mut delta = DeltaFixture::build(&initial, &sandbox.path.join("delta"), false);
    advertise_preload(&mut delta.resources, "2.0");
    service.replace_resources(delta.resources.clone());
    success(sandbox.run(&game, &["update"]));
    assert_eq!(fs::read(game.join("payload.pkg")).unwrap(), delta.payload);
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(game.join(".wg-launcher/state.json")).unwrap()).unwrap();
    assert_eq!(state["version"], "2.0");
    assert_eq!(
        state["component_versions"],
        serde_json::json!({"client": "2.0", "locale": "1.0"})
    );

    delta.resources.insert(
        "updates/api/v1/patches_chain/".into(),
        b"<protocol><version_name>2.0</version_name></protocol>".to_vec(),
    );
    advertise_preload(&mut delta.resources, "2.0");
    service.replace_resources(delta.resources);
    let before = service.requests().len();
    let output = success(sandbox.run(&game, &["update"]));
    assert!(output.contains("Already up to date (2.0)"), "{output}");
    let requests = service.requests();
    assert!(
        requests[before..]
            .iter()
            .all(|request| !request.path.ends_with(".wgpkg"))
    );
    assert!(
        requests
            .iter()
            .all(|request| request.path != "/preload.wgpkg")
    );
}

#[test]
fn preload_flag_selects_the_advertised_delta_after_production() {
    let sandbox = Sandbox::new();
    let initial = sandbox.fixture(Release::V1);
    let delta = DeltaFixture::build(&initial, &sandbox.path.join("preload"), false);
    let chain_path = "updates/api/v1/patches_chain/";
    let mut resources = initial.service_resources("updates");
    resources.insert(
        "preload.wgpkg".into(),
        delta.resources["delta.wgpkg"].clone(),
    );
    advertise_preload(&mut resources, "1.0");
    let service = WgUpdateService::start(resources.clone());
    let config = sandbox.configuration(&service);
    let selection = [
        "--game",
        "wot",
        "--region",
        "local",
        "--product",
        config.to_str().unwrap(),
    ];

    let mut list = vec!["list", "--preload"];
    list.extend(selection);
    let listed = success(sandbox.run(&sandbox.path, &list));
    assert!(
        listed.contains("1.0") && listed.contains("--preload"),
        "{listed}"
    );
    let mut inspect = vec!["inspect", "--preload"];
    inspect.extend(selection);
    let inspected = success(sandbox.run(&sandbox.path, &inspect));
    assert!(inspected.contains("3.0"), "{inspected}");
    assert!(
        service
            .requests()
            .iter()
            .all(|request| !request.path.ends_with(".wgpkg"))
    );

    let integrated = sandbox.path.join("integrated");
    let staged = sandbox.path.join("staged");
    let production = sandbox.game();
    for (command, path, preload) in [
        ("install", &integrated, true),
        ("download", &staged, true),
        ("install", &production, false),
    ] {
        let mut args = vec![command, "--output", path.to_str().unwrap()];
        args.extend(selection);
        if preload {
            args.push("--preload");
        }
        success(sandbox.run(&sandbox.path, &args));
    }
    assert_installed(&production, Release::V1);
    let before_extract = service.requests().len();
    success(sandbox.run(&staged, &["extract"]));
    assert_eq!(service.requests().len(), before_extract);

    resources.insert(
        chain_path.into(),
        br#"<protocol><version_name>1.0</version_name><web_seeds><url>__BASE_URL__/</url></web_seeds></protocol>"#.to_vec(),
    );
    advertise_preload(&mut resources, "1.0");
    service.replace_resources(resources);
    let status = success(sandbox.run(&production, &["status", "--preload"]));
    assert!(status.contains("wg-launcher update --preload"), "{status}");
    let before_update = service.requests().len();
    success(sandbox.run(&production, &["update", "--preload"]));
    let requests = service.requests();
    let packages: Vec<_> = requests[before_update..]
        .iter()
        .filter(|request| request.path.ends_with(".wgpkg"))
        .map(|request| request.path.as_str())
        .collect();
    assert_eq!(packages, ["/preload.wgpkg"]);

    for path in [integrated, staged, production] {
        assert_eq!(fs::read(path.join("payload.pkg")).unwrap(), delta.payload);
        let state: serde_json::Value =
            serde_json::from_slice(&fs::read(path.join(".wg-launcher/state.json")).unwrap())
                .unwrap();
        assert_eq!(state["version"], "1.0");
        assert_eq!(
            state["component_versions"],
            serde_json::json!({"client": "3.0", "locale": "1.0"})
        );
    }
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

#[test]
fn split_volumes_keep_local_paths_sizes_and_archive_order_in_both_workflows() {
    for staged in [false, true] {
        let sandbox = Sandbox::new();
        let fixture = sandbox.fixture(Release::V1);
        let resources = fixture.split_volume_resources("updates");
        let service = WgUpdateService::start(resources.clone());
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
        let before = service.requests();
        for name in ["client/content.wgpkg.001", "client/content.wgpkg.002"] {
            assert_eq!(
                before
                    .iter()
                    .filter(|request| request.path == format!("/{name}"))
                    .count(),
                1
            );
        }
        if staged {
            service.stop();
            let cache = game.join(".wg-launcher/download");
            let missing = cache.join("client/content.wgpkg.002");
            fs::remove_file(&missing).unwrap();
            failure(sandbox.run(&game, &["extract"]), "incomplete");
            assert!(!game.join("payload.pkg").exists());
            assert!(!game.join(".wg-launcher/state.json").exists());
            let manifest: serde_json::Value =
                serde_json::from_slice(&fs::read(cache.join("manifest.json")).unwrap()).unwrap();
            assert_eq!(manifest["base_versions"], serde_json::json!({}));
            fs::write(missing, &resources["client/content.wgpkg.002"]).unwrap();
            success(sandbox.run(&game, &["extract"]));
            assert_eq!(service.requests(), before);
        }
        assert_installed(&game, Release::V1);
    }
}
