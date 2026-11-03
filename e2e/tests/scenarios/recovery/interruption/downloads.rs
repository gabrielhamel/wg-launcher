use super::*;

#[test]
fn killed_concurrent_downloads_resume_and_release_the_installation_lock() {
    interrupted_transfer(false);
}

#[test]
fn service_loss_during_transfer_recovers_on_the_same_address() {
    interrupted_transfer(true);
}

fn interrupted_transfer(stop_service: bool) {
    let sandbox = Sandbox::new();
    let mut fixture = sandbox.fixture(Release::V1);
    let mut expected = transfer_payload();
    expected.truncate(256 * 1024);
    for (part, name) in [("client", "payload.pkg"), ("locale", "locale.txt")] {
        fixture.packages.insert(
            format!("{part}.wgpkg"),
            archive(&sandbox.path.join(part), [(name.into(), expected.clone())]),
        );
    }
    let server = WgUpdateService::start(fixture.service_resources("updates"));
    let upstream = server
        .url()
        .strip_prefix("http://")
        .unwrap()
        .parse()
        .unwrap();
    let gate = crate::support::transfer_gate::TransferGate::new(upstream);
    let mut resources = fixture.service_resources("updates");
    let chain = String::from_utf8(resources["updates/api/v1/patches_chain/"].clone())
        .unwrap()
        .replace("__BASE_URL__", &gate.url());
    resources.insert("updates/api/v1/patches_chain/".into(), chain.into_bytes());
    server.replace_resources(resources.clone());
    let config = sandbox.configuration(&server);
    let game = sandbox.game();
    let args = [
        "download",
        "--game",
        "wot",
        "--product",
        config.to_str().unwrap(),
        "--region",
        "local",
        "--output",
        game.to_str().unwrap(),
    ];
    let cache = game.join(".wg-launcher/download");
    let partial = cache.join("client.wgpkg.part");
    let mut child = sandbox.spawn(&sandbox.path, &args);
    child.until(|| {
        ["client", "locale"].iter().all(|part| {
            let path = cache.join(format!("{part}.wgpkg.part"));
            size(&path) > 0 && size(&path) < fixture.packages[&format!("{part}.wgpkg")].len() as u64
        })
    });
    // Run a competing real CLI while both responses remain gated. Even if
    // this controller is descheduled, the first download cannot complete.
    crate::support::failure(
        sandbox.run(&sandbox.path, &args),
        "another installation is already running",
    );
    if stop_service {
        server.stop();
        gate.disconnect();
    }
    let output = if stop_service {
        // Keep the bounded production network retry budget for this outage case.
        child.wait_with_timeout(std::time::Duration::from_secs(150))
    } else {
        child.kill()
    };
    assert!(!output.status.success());
    let prefix = size(&partial);
    assert!(
        prefix > 0 && prefix < fixture.packages["client.wgpkg"].len() as u64,
        "missed in-flight transfer: {prefix}"
    );
    assert!(!game.join(".wg-launcher/state.json").exists());
    if stop_service {
        server.replace_resources(resources);
    }
    let cached_before = ["client.wgpkg", "locale.wgpkg"].map(|name| {
        let complete = cache.join(name).is_file();
        let partial = size(&cache.join(format!("{name}.part")));
        (name, complete, partial)
    });
    gate.open();
    let before = server.requests().len();
    success(sandbox.run(&sandbox.path, &args));
    let requests = server.requests();
    assert!(
        requests[before..]
            .iter()
            .any(|r| r.path == "/client.wgpkg" && r.range == Some(format!("bytes={prefix}-")))
    );
    assert!(
        requests[before..]
            .iter()
            .all(|r| r.path.ends_with(".wgpkg")),
        "retry performed discovery"
    );
    for (name, complete, partial) in cached_before {
        let path = format!("/{name}");
        if complete {
            assert!(
                requests[before..].iter().all(|r| r.path != path),
                "completed concurrent volume was redownloaded"
            );
        } else if partial > 0 && partial < fixture.packages[name].len() as u64 {
            assert!(
                requests[before..]
                    .iter()
                    .any(|r| r.path == path && r.range == Some(format!("bytes={partial}-")))
            );
        }
    }
    for (name, bytes) in &fixture.packages {
        assert_eq!(fs::read(cache.join(name)).unwrap(), *bytes);
    }
    server.stop();
    success(sandbox.run(&game, &["extract"]));
    for name in ["payload.pkg", "locale.txt"] {
        assert_eq!(fs::read(game.join(name)).unwrap(), expected);
    }
    assert_state(&game, "1.0", "1.0");
    assert!(!cache.exists());
    assert!(!game.join(".wg-launcher/installing").exists());
}
