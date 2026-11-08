use super::Recovery;
use crate::support::{assert_installed, failure, success};
use std::fs;

#[test]
fn invalid_cached_sizes_are_redownloaded_without_discovery() {
    for partial in [false, true] {
        for oversized in [false, true] {
            let recovery = Recovery::downloaded();
            let cache = recovery.cache();
            let bytes = &recovery.fixture.packages["client.wgpkg"];
            fs::remove_file(cache.join("client.wgpkg")).unwrap();
            let name = if partial {
                "client.wgpkg.part"
            } else {
                "client.wgpkg"
            };
            let size = if oversized {
                bytes.len() + 17
            } else {
                bytes.len() / 2
            };
            let data = if oversized {
                vec![0; size]
            } else {
                bytes[..size].to_vec()
            };
            fs::write(cache.join(name), data).unwrap();
            let before = recovery.server.requests().len();
            success(recovery.run("download"));
            assert_eq!(fs::read(cache.join("client.wgpkg")).unwrap(), *bytes);
            assert!(!cache.join("client.wgpkg.part").exists());
            let requests = recovery.server.requests();
            assert_eq!(requests.len(), before + 1);
            assert_eq!(requests[before].path, "/client.wgpkg");
            let range = (partial && !oversized).then(|| format!("bytes={size}-"));
            assert_eq!(requests[before].range, range);
            success(recovery.sandbox.run(&recovery.game, &["extract"]));
            assert_installed(&recovery.game, crate::fixture::Release::V1);
        }
    }
}

#[test]
fn complete_partial_volume_is_promoted_without_http() {
    let recovery = Recovery::downloaded();
    let cache = recovery.cache();
    fs::rename(cache.join("client.wgpkg"), cache.join("client.wgpkg.part")).unwrap();
    let before = recovery.server.requests();
    let downloaded = success(recovery.run("download"));
    assert!(
        downloaded.contains("1 volumes completed, 1 reused, 0 B newly retained"),
        "{downloaded}"
    );
    assert_eq!(recovery.server.requests(), before);
    fs::rename(cache.join("client.wgpkg"), cache.join("client.wgpkg.part")).unwrap();
    let installed = success(recovery.run("install"));
    assert!(
        installed.contains("1 volumes completed, 1 reused, 0 B newly retained"),
        "{installed}"
    );
    assert_eq!(recovery.server.requests(), before);
    assert_installed(&recovery.game, crate::fixture::Release::V1);
}

#[test]
fn corrupt_partial_prefix_fails_extraction_and_can_be_repaired() {
    let recovery = Recovery::downloaded();
    let cache = recovery.cache();
    let bytes = &recovery.fixture.packages["client.wgpkg"];
    fs::remove_file(cache.join("client.wgpkg")).unwrap();
    fs::write(cache.join("client.wgpkg.part"), vec![0; bytes.len() / 2]).unwrap();
    success(recovery.run("download"));
    // WGUS sizes do not authenticate bytes. Extraction must reject this archive.
    failure(
        recovery.sandbox.run(&recovery.game, &["extract"]),
        "archive",
    );
    assert!(!recovery.game.join(".wg-launcher/state.json").exists());
    assert!(cache.join("manifest.json").is_file());
    fs::remove_file(cache.join("client.wgpkg")).unwrap();
    success(recovery.run("install"));
    assert_installed(&recovery.game, crate::fixture::Release::V1);
}

#[test]
fn unavailable_package_keeps_other_volumes_and_recovers_after_service_restart() {
    let recovery = Recovery::downloaded();
    fs::remove_file(recovery.cache().join("client.wgpkg")).unwrap();
    let pinned = fs::read(recovery.cache().join("manifest.json")).unwrap();
    let mut resources = recovery.fixture.service_resources("updates");
    resources.remove("client.wgpkg");
    recovery.server.replace_resources(resources);
    failure(recovery.run("download"), "404");
    assert_eq!(
        fs::read(recovery.cache().join("manifest.json")).unwrap(),
        pinned
    );
    assert_eq!(
        fs::read(recovery.cache().join("locale.wgpkg")).unwrap(),
        recovery.fixture.packages["locale.wgpkg"]
    );
    assert!(!recovery.game.join(".wg-launcher/state.json").exists());
    recovery
        .server
        .replace_resources(recovery.fixture.service_resources("updates"));
    let before = recovery.server.requests().len();
    success(recovery.run("install"));
    let requests = recovery.server.requests();
    assert_eq!(requests.len(), before + 1);
    assert_eq!(requests[before].path, "/client.wgpkg");
    assert_installed(&recovery.game, crate::fixture::Release::V1);
}
