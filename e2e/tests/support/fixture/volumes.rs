//! Split a real archive into nested, uneven volumes advertised out of filename order.

use super::Fixture;
use std::collections::BTreeMap;

impl Fixture {
    pub fn split_volume_resources(&self, prefix: &str) -> BTreeMap<String, Vec<u8>> {
        let mut resources = self.service_resources(prefix);
        let archive = resources.remove("client.wgpkg").unwrap();
        let split = archive.len() / 3;
        let volumes = [
            ("client/content.wgpkg.001", &archive[..split]),
            ("client/content.wgpkg.002", &archive[split..]),
        ];
        let files = volumes
            .iter()
            .rev()
            .map(|(name, bytes)| {
                format!(
                    "<file><name>{name}</name><size>{}</size></file>",
                    bytes.len()
                )
            })
            .collect::<String>();
        let route = format!("{}/api/v1/patches_chain/", prefix.trim_matches('/'));
        let chain = resources.get_mut(route.trim_start_matches('/')).unwrap();
        *chain = String::from_utf8(chain.clone())
            .unwrap()
            .replace(
                &format!(
                    "<file><name>client.wgpkg</name><size>{}</size></file>",
                    archive.len()
                ),
                &files,
            )
            .into_bytes();
        for (name, bytes) in volumes {
            resources.insert(name.into(), bytes.to_vec());
        }
        resources
    }
}
