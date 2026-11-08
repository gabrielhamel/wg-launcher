use crate::support::{Sandbox, assert_installed, failure, success};
use crate::{fixture::Release, server::WgUpdateService};

#[test]
fn ambiguous_required_transition_fails_before_packages_or_installation_writes() {
    let sandbox = Sandbox::new();
    let fixture = sandbox.fixture(Release::V1);
    let server = WgUpdateService::start(fixture.ambiguous_transition_resources("0"));
    let config = sandbox.configuration(&server);
    let game = sandbox.game();
    // Inspection reports publisher data without selecting an executable chain.
    success(sandbox.run(
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
    for command in ["download", "install"] {
        let mut args = vec![
            command,
            "--game",
            "wot",
            "--product",
            config.to_str().unwrap(),
            "--region",
            "local",
        ];
        args.extend(["--output", game.to_str().unwrap()]);
        failure(
            sandbox.run(&sandbox.path, &args),
            "ambiguous patch chain for client at version 0",
        );
        assert!(!game.join(".wg-launcher/download/manifest.json").exists());
        assert!(!game.join(".wg-launcher/state.json").exists());
        assert!(!game.join("payload.pkg").exists());
    }
    assert!(
        server
            .requests()
            .iter()
            .all(|request| request.path.starts_with("/updates/api/v1/"))
    );
}

#[test]
fn unused_ambiguous_transitions_do_not_prevent_installation() {
    let sandbox = Sandbox::new();
    let fixture = sandbox.fixture(Release::V1);
    let server = WgUpdateService::start(fixture.ambiguous_transition_resources("obsolete"));
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
    assert_installed(&game, Release::V1);
}
