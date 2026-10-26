use crate::support::{Sandbox, assert_installed_in_region, success};
use crate::{
    fixture::{Fixture, Release},
    server::WgUpdateService,
};
use std::fs;

fn resources_with_client_types(
    fixture: &Fixture,
    default_client_type: &str,
) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut resources = fixture.service_resources("updates");
    let metadata = resources
        .remove("updates/api/v1/metadata/")
        .expect("fixture metadata");
    let metadata = String::from_utf8(metadata)
        .unwrap()
        .replace(
            r#"<client_types default="sd">"#,
            &format!(r#"<client_types default="{default_client_type}">"#),
        )
        .replace(r#"<client_type id="sd""#, r#"<client_type id="large""#)
        .replace(
            "</client_types>",
            r#"<client_type id="small" arch="x64"><client_parts><client_part id="locale" lang="true" integrity="true"/></client_parts></client_type></client_types>"#,
        );
    resources.insert("updates/api/v1/metadata/".into(), metadata.into_bytes());
    resources
}

#[test]
fn omitted_client_type_prefers_the_publisher_default_then_the_smallest_variant() {
    let sandbox = Sandbox::new();
    let fixture = sandbox.fixture(Release::V1);
    let server = WgUpdateService::start(resources_with_client_types(&fixture, "large"));
    let config = sandbox.configuration(&server);
    let args = [
        "inspect",
        "--game",
        "wot",
        "--product",
        config.to_str().unwrap(),
        "--region",
        "local",
    ];

    let output = success(sandbox.run(&sandbox.path, &args));
    assert!(output.contains("Client:        large"));

    server.replace_resources(resources_with_client_types(&fixture, "missing"));
    let output = success(sandbox.run(&sandbox.path, &args));
    assert!(output.contains("Client:        small"));

    let mut explicit_args = args.to_vec();
    explicit_args.extend(["--client-type", "large"]);
    let output = success(sandbox.run(&sandbox.path, &explicit_args));
    assert!(output.contains("Client:        large"));
}

#[test]
fn game_filter_selects_one_of_multiple_games_in_the_same_region() {
    let sandbox = Sandbox::new();
    let tanks_fixture = sandbox.fixture(Release::V1);
    let ships_fixture = sandbox.fixture(Release::V2);
    let tanks = WgUpdateService::start(tanks_fixture.service_resources("updates"));
    let ships = WgUpdateService::start(ships_fixture.service_resources("updates"));
    let config = sandbox.path.join("games.json");
    fs::write(
        &config,
        format!(
            r#"[{{"game":"tanks","region":"local","name":"Local tanks","host":"{}/updates","game_id":"LOCAL.WOT","channel":"testing"}},{{"game":"ships","region":"local","name":"Local ships","host":"{}/updates","game_id":"LOCAL.WOT","channel":"testing"}}]"#,
            tanks.url(),
            ships.url()
        ),
    )
    .unwrap();

    let output = success(sandbox.run(
        &sandbox.path,
        &[
            "list",
            "--product",
            config.to_str().unwrap(),
            "--game",
            "ships",
            "--region",
            "local",
        ],
    ));

    assert!(output.contains("SHIPS"));
    assert!(output.contains("2.0"));
    assert!(!output.contains("1.0"));
    assert!(tanks.requests().is_empty());
    assert!(!ships.requests().is_empty());
}

#[test]
fn product_array_lists_all_regions_and_region_selects_the_correct_service() {
    let sandbox = Sandbox::new();
    let eu_fixture = sandbox.fixture(Release::V1);
    let na_fixture = sandbox.fixture(Release::V2);
    let eu = WgUpdateService::start(eu_fixture.service_resources("updates"));
    let na = WgUpdateService::start(na_fixture.service_resources("updates"));
    let config = sandbox.path.join("products.json");
    fs::write(&config, Fixture::catalog(&eu.url(), &na.url())).unwrap();
    let config = config.to_str().unwrap();
    let output = success(sandbox.run(&sandbox.path, &["list", "--product", config]));
    assert!(output.contains("EU"));
    assert!(output.contains("NA"));
    assert!(output.contains("1.0"));
    assert!(output.contains("2.0"));
    assert!(
        eu.requests()
            .iter()
            .all(|request| !request.path.ends_with(".wgpkg"))
    );
    assert!(
        na.requests()
            .iter()
            .all(|request| !request.path.ends_with(".wgpkg"))
    );
    let before_eu = eu.requests();
    let output = success(sandbox.run(
        &sandbox.path,
        &["list", "--product", config, "--region", "NA"],
    ));
    assert!(output.contains("NA"));
    assert!(!output.contains("1.0"));
    assert_eq!(eu.requests(), before_eu);
    let output = success(sandbox.run(
        &sandbox.path,
        &[
            "inspect",
            "--game",
            "wot",
            "--product",
            config,
            "--region",
            "na",
        ],
    ));
    assert!(output.contains("2.0"));
    assert_eq!(eu.requests(), before_eu);
    let before_na = na.requests();
    let eu_game = sandbox.path.join("eu-game");
    // The ordinary EU default also applies to custom catalogs.
    success(sandbox.run(
        &sandbox.path,
        &[
            "install",
            "--game",
            "wot",
            "--product",
            config,
            "--output",
            eu_game.to_str().unwrap(),
        ],
    ));
    assert_installed_in_region(&eu_game, Release::V1, "eu");
    assert_eq!(na.requests(), before_na);
    let na_game = sandbox.game();
    success(sandbox.run(
        &sandbox.path,
        &[
            "install",
            "--game",
            "wot",
            "--product",
            config,
            "--region",
            "na",
            "--output",
            na_game.to_str().unwrap(),
        ],
    ));
    assert_installed_in_region(&na_game, Release::V2, "na");
}

#[test]
fn custom_discovery_uses_application_id_path_prefix_and_language_without_downloading() {
    let sandbox = Sandbox::new();
    let fixture = sandbox.fixture(Release::V1);
    let server = WgUpdateService::start(fixture.service_resources("updates"));
    let config = sandbox.configuration(&server);
    let config = config.to_str().unwrap();

    let output = success(sandbox.run(&sandbox.path, &["list", "--product", config]));
    assert!(output.contains("LOCAL"));
    assert!(output.contains("testing"));
    assert!(output.contains("1.0"));
    assert!(output.contains("EN*"));
    let output = success(sandbox.run(
        &sandbox.path,
        &[
            "inspect",
            "--game",
            "wot",
            "--product",
            config,
            "--region",
            "local",
            "--language",
            "FR",
        ],
    ));
    assert!(output.contains("local"));
    assert!(output.contains("client"));
    assert!(output.contains("locale"));
    assert!(output.contains("FR"));
    let requests = server.requests();
    assert!(
        requests
            .iter()
            .all(|request| request.path.starts_with("/updates/api/v1/"))
    );
    assert!(
        requests
            .iter()
            .filter(|request| request.path.ends_with("/metadata/"))
            .all(|request| request.query.as_deref().unwrap().contains("guid=LOCAL.WOT"))
    );
    assert!(
        requests
            .iter()
            .filter(|request| request.path.ends_with("/patches_chain/"))
            .all(|request| request
                .query
                .as_deref()
                .unwrap()
                .contains("game_id=LOCAL.WOT"))
    );
    assert!(
        requests
            .iter()
            .any(|request| request.query.as_deref().unwrap().contains("lang=FR"))
    );
    assert!(!sandbox.game().exists());
}
