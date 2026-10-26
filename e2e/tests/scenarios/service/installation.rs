use crate::{
    fixture::{Fixture, Release},
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
