//! Publisher responses with multiple chains or ambiguous transitions.

use super::Fixture;
use std::collections::BTreeMap;

pub fn advertise_preload(resources: &mut BTreeMap<String, Vec<u8>>, production_version: &str) {
    let size = resources.get("preload.wgpkg").map_or(1000, Vec::len);
    let chain = resources.get_mut("updates/api/v1/patches_chain/").unwrap();
    let preload = format!(
        r#"<patches_chain type="preload">
          <patch><part>client</part><version_from>{production_version}</version_from><version_to>3.0</version_to>
            <files><file><name>preload.wgpkg</name><size>{size}</size></file></files></patch>
        </patches_chain>"#,
    );
    let xml = String::from_utf8(chain.clone()).unwrap();
    let xml = if xml.contains("<patches_chain>") {
        xml.replace(
            "<patches_chain>",
            &format!(r#"{preload}<patches_chain type="install">"#),
        )
    } else {
        xml.replace("</protocol>", &format!("{preload}</protocol>"))
    };
    *chain = xml
        .replace(
            "</protocol>",
            "<delay_preload>True</delay_preload></protocol>",
        )
        .into_bytes();
}

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
