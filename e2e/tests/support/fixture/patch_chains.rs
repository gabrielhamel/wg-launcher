//! Publisher chains with ambiguous selected or unused transitions.

use super::Fixture;
use std::collections::BTreeMap;

impl Fixture {
    pub fn ambiguous_transition_resources(&self, source: &str) -> BTreeMap<String, Vec<u8>> {
        let mut resources = self.service_resources("updates");
        let chain = resources.get_mut("updates/api/v1/patches_chain/").unwrap();
        // No volumes are needed for alternatives that planning must reject or ignore.
        let alternative = format!(
            "<patch><part>client</part><version_from>{source}</version_from><version_to>{}</version_to><files/></patch>",
            self.release.version(),
        );
        let alternatives = if source == "0" {
            alternative
        } else {
            alternative.repeat(2)
        };
        *chain = String::from_utf8(chain.clone())
            .unwrap()
            .replace(
                "</patches_chain>",
                &format!("{alternatives}</patches_chain>"),
            )
            .into_bytes();
        resources
    }
}
