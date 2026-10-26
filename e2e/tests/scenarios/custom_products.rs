use crate::support::{Sandbox, assert_installed, assert_installed_in_region, failure, success};
use crate::{
    fixture::{Fixture, Release},
    server::WgUpdateService,
};
use std::fs;

#[test]
fn cache_identity_uses_selected_product_independently_of_other_catalog_entries() {
    let sandbox = Sandbox::new();
    let fixture = sandbox.fixture(Release::V1);
    let server = WgUpdateService::start(fixture.service_resources("updates"));
    let config = sandbox.path.join("products.json");
    fs::write(&config, Fixture::catalog(&server.url(), &server.url())).unwrap();
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
            "na",
            "--output",
            game.to_str().unwrap(),
        ],
    ));
    let before = server.requests();
    let mut products: Vec<serde_json::Value> =
        serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    products[0]["game_id"] = "CHANGED.EU".into();
    products.reverse();
    fs::write(&config, serde_json::to_vec(&products).unwrap()).unwrap();
    success(sandbox.run(
        &sandbox.path,
        &[
            "install",
            "--game",
            "wot",
            "--product",
            config.to_str().unwrap(),
            "--region",
            "na",
            "--output",
            game.to_str().unwrap(),
        ],
    ));
    assert_installed_in_region(&game, Release::V1, "na");
    assert_eq!(server.requests(), before);
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(game.join(".wg-launcher/state.json")).unwrap()).unwrap();
    assert_eq!(state["product"]["region"], "na");
    assert!(state.get("products").is_none());
}

#[test]
fn custom_staged_installation_extracts_offline_and_refuses_another_product_cache() {
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
    let before = server.requests().len();
    failure(
        sandbox.run(
            &sandbox.path,
            &[
                "install",
                "--game",
                "wot",
                "--output",
                game.to_str().unwrap(),
            ],
        ),
        "cached build",
    );
    let mut product: serde_json::Value =
        serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    product[0]["game_id"] = "OTHER.GAME".into();
    fs::write(&config, serde_json::to_vec(&product).unwrap()).unwrap();
    failure(
        sandbox.run(
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
        ),
        "cached build",
    );
    assert_eq!(server.requests().len(), before);
    fs::remove_file(config).unwrap();
    drop(server);
    success(sandbox.run(&game, &["extract"]));
    assert_installed(&game, Release::V1);
}
