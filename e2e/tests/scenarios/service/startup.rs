use crate::{
    fixture::Release,
    server::WgUpdateService,
    support::{Sandbox, success},
};
use std::{collections::BTreeSet, sync::Barrier, thread};

#[test]
fn parallel_services_own_distinct_ports_and_restart_on_the_same_address() {
    let sandbox = Sandbox::new();
    let initial = sandbox.fixture(Release::V1).service_resources("updates");
    let barrier = Barrier::new(4);
    let services = thread::scope(|scope| {
        let handles = (0..4)
            .map(|_| {
                let resources = initial.clone();
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    WgUpdateService::start(resources)
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    let addresses = services
        .iter()
        .map(WgUpdateService::url)
        .collect::<BTreeSet<_>>();
    assert_eq!(addresses.len(), services.len());

    let original = services[0].url();
    services[0].replace_resources(sandbox.fixture(Release::V2).service_resources("updates"));
    assert_eq!(services[0].url(), original);
    for (index, service) in services.iter().enumerate() {
        let config = sandbox.configuration(service);
        let listed = success(sandbox.run(
            &sandbox.path,
            &["list", "--product", config.to_str().unwrap()],
        ));
        let (expected, other) = if index == 0 {
            ("2.0", "1.0")
        } else {
            ("1.0", "2.0")
        };
        assert!(listed.contains(expected), "{listed}");
        assert!(!listed.contains(other), "{listed}");
        assert!(!service.requests().is_empty());
        assert!(
            services[index + 1..]
                .iter()
                .all(|service| service.requests().is_empty())
        );
    }
}
