use crate::support::{Sandbox, assert_installed_in_region, failure, success};
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

#[test]
fn redirected_discovery_retains_product_and_component_versions_and_ignores_self_redirects() {
    let sandbox = Sandbox::new();
    let fixture = sandbox.fixture(Release::V1);
    let mirror =
        WgUpdateService::start(fixture.redirected_resources("mirror", "__BASE_URL__/mirror/"));
    let redirect = format!("{}/mirror", mirror.url());
    let origin = WgUpdateService::start(fixture.redirected_resources("updates", &redirect));
    let config = sandbox.configuration(&origin);
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
    assert_installed_in_region(&game, Release::V1, "local");
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(game.join(".wg-launcher/state.json")).unwrap()).unwrap();
    assert_eq!(
        state["product"]["host"],
        format!("{}/updates", origin.url())
    );
    assert!(
        origin
            .requests()
            .iter()
            .all(|request| request.path.starts_with("/updates/api/v1/"))
    );
    assert!(
        mirror
            .requests()
            .iter()
            .any(|request| request.path == "/client.wgpkg")
    );

    let updated = sandbox.fixture(Release::V2);
    mirror.replace_resources(updated.redirected_resources("mirror", "__BASE_URL__/mirror/"));
    let before = mirror.requests().len();
    let output = success(sandbox.run(&game, &["status"]));
    assert!(output.contains("Update available."));
    success(sandbox.run(&game, &["update"]));
    assert_installed_in_region(&game, Release::V2, "local");
    let requests = mirror.requests();
    let chains = requests[before..]
        .iter()
        .filter(|request| request.path.ends_with("/patches_chain/"))
        .collect::<Vec<_>>();
    assert_eq!(chains.len(), 2);
    for request in chains {
        let query = request.query.as_deref().unwrap();
        assert!(query.contains("client_current_version=1.0"), "{query}");
        assert!(query.contains("locale_current_version=1.0"), "{query}");
        assert!(query.contains("lang=EN"), "{query}");
    }
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(game.join(".wg-launcher/state.json")).unwrap()).unwrap();
    assert_eq!(
        state["product"]["host"],
        format!("{}/updates", origin.url())
    );
}

#[test]
fn cyclic_discovery_redirects_stop_without_downloading_packages() {
    let sandbox = Sandbox::new();
    let fixture = sandbox.fixture(Release::V1);
    let origin = WgUpdateService::start(fixture.service_resources("updates"));
    let mirror = WgUpdateService::start(
        fixture.redirected_resources("mirror", &format!("{}/updates", origin.url())),
    );
    origin.replace_resources(
        fixture.redirected_resources("updates", &format!("{}/mirror", mirror.url())),
    );
    let config = sandbox.configuration(&origin);
    crate::support::failure(
        sandbox.run(
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
        ),
        "WGUS redirect limit exceeded",
    );
    // Omitted client type first fetches metadata to choose the publisher default.
    assert_eq!(origin.requests().len(), 5);
    assert_eq!(mirror.requests().len(), 4);
    assert!(!sandbox.game().exists());
}

#[test]
fn smallest_variant_ties_keep_publisher_order() {
    let sandbox = Sandbox::new();
    let fixture = sandbox.fixture(Release::V1);
    let mut resources = resources_with_client_types(&fixture, "missing");
    let metadata = resources.get_mut("updates/api/v1/metadata/").unwrap();
    *metadata = String::from_utf8(metadata.clone())
        .unwrap()
        .replace("<client_part id=\"client\" integrity=\"true\"/>", "")
        .into_bytes();
    let service = WgUpdateService::start(resources);
    let config = sandbox.configuration(&service);
    let output = success(sandbox.run(
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
    assert!(output.contains("Client:        large"), "{output}");
}

#[test]
fn missing_component_targets_preserve_first_variant_failure() {
    let sandbox = Sandbox::new();
    let fixture = sandbox.fixture(Release::V1);
    let mut resources = resources_with_client_types(&fixture, "missing");
    let chain = resources.get_mut("updates/api/v1/patches_chain/").unwrap();
    *chain = String::from_utf8(chain.clone())
        .unwrap()
        .replace("<part>client</part>", "<part>unselected-client</part>")
        .replace("<part>locale</part>", "<part>unselected-locale</part>")
        .into_bytes();
    let service = WgUpdateService::start(resources);
    let config = sandbox.configuration(&service);
    failure(
        sandbox.run(
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
        ),
        "cannot plan client: the release has no target version for this component",
    );
    assert!(
        service
            .requests()
            .iter()
            .all(|request| !request.path.ends_with(".wgpkg"))
    );
}
