use super::{Fixture, Release};
use std::{collections::BTreeMap, fs, path::Path};

/// A source-dependent update that leaves the locale component unchanged.
pub struct DeltaFixture {
    pub resources: BTreeMap<String, Vec<u8>>,
    pub payload: Vec<u8>,
}

impl DeltaFixture {
    /// WGUS-compatible delta-only response with an omitted source version.
    pub fn use_delta_only_chain(&mut self) {
        let xml = format!(
            r#"<protocol><version_name>2.0</version_name><patches_chain>
          <patch><part>client</part><version_to>2.0</version_to>
            <files><file><name>delta.wgpkg</name><size>{}</size></file></files></patch>
          </patches_chain><web_seeds><url>__BASE_URL__/</url></web_seeds></protocol>"#,
            self.resources["delta.wgpkg"].len()
        );
        self.resources
            .insert("updates/api/v1/patches_chain/".into(), xml.into_bytes());
    }

    pub fn build(initial: &Fixture, scratch: &Path, second_delta: bool) -> Self {
        fs::create_dir_all(scratch).unwrap();
        // Librsync delta: copy all but the first byte, then append a literal.
        // Reapplying against its own output produces different bytes, so retry
        // assertions detect reuse of an already patched source.
        let delta = vec![
            0x72,
            0x73,
            0x02,
            0x36,
            0x45,
            1,
            (Release::V1.payload().len() - 1) as u8,
            1,
            b'!',
            0,
        ];
        let payload = [&Release::V1.payload()[1..], b"!"].concat();
        let mut files = vec![("payload.pkg.1.deadbeef.rdiff", delta.clone())];
        if second_delta {
            files.push(("z.pkg.1.deadbeef.rdiff", delta));
        }
        let directory = scratch.join("transport");
        fs::create_dir_all(&directory).unwrap();
        let archive = scratch.join("delta.wgpkg");
        let mut writer =
            sevenz_rust::SevenZWriter::new(fs::File::create(&archive).unwrap()).unwrap();
        for (name, bytes) in files {
            let source = directory.join(name);
            fs::write(&source, bytes).unwrap();
            writer
                .push_archive_entry(
                    sevenz_rust::SevenZArchiveEntry::from_path(&source, name.to_owned()),
                    Some(fs::File::open(&source).unwrap()),
                )
                .unwrap();
        }
        writer.finish().unwrap();
        let bytes = fs::read(archive).unwrap();
        let mut resources = initial.service_resources("updates");
        // Keep the full base packages published as well: the launcher must skip
        // them when selecting the chain from the installed component version.
        let xml = format!(
            r#"<protocol><version_name>2.0</version_name><patches_chain>
          <patch><part>client</part><version_from>0</version_from><version_to>1.0</version_to>
            <files><file><name>client.wgpkg</name><size>{}</size></file></files></patch>
          <patch><part>client</part><version_from>1.0</version_from><version_to>2.0</version_to>
            <files><file><name>delta.wgpkg</name><size>{}</size></file></files></patch>
          <patch><part>locale</part><version_from>0</version_from><version_to>1.0</version_to>
            <files><file><name>locale.wgpkg</name><size>{}</size></file></files></patch>
          </patches_chain><web_seeds><url>__BASE_URL__/</url></web_seeds></protocol>"#,
            initial.packages["client.wgpkg"].len(),
            bytes.len(),
            initial.packages["locale.wgpkg"].len()
        );
        resources.insert("updates/api/v1/patches_chain/".into(), xml.into_bytes());
        resources.insert("delta.wgpkg".into(), bytes);
        Self { resources, payload }
    }
}
