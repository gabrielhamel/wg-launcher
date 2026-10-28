mod delta;
pub use delta::DeltaFixture;

use std::{collections::BTreeMap, fs, path::Path};

const METADATA: &str = r#"<protocol wgc_publisher_id="fixture">
  <version>1</version>
  <predefined_section>
    <app_id>LOCAL.WOT</app_id>
    <supported_languages>EN,FR</supported_languages>
    <default_language>EN</default_language>
    <name>Local test game</name>
    <fs_name>LocalGame</fs_name>
    <environment_name>testing</environment_name>
    <client_types default="sd">
      <client_type id="sd" arch="x64">
        <client_parts>
          <client_part id="client" integrity="true"/>
          <client_part id="locale" lang="true" integrity="true"/>
        </client_parts>
      </client_type>
    </client_types>
  </predefined_section>
</protocol>"#;

const PATCH_CHAIN: &str = r#"<protocol>
  <version_name>__VERSION__</version_name>
  <patches_chain>
    <patch>
      <part>client</part><version_from>0</version_from><version_to>__VERSION__</version_to>
      <files><file><name>client.wgpkg</name><size>__CLIENT_SIZE__</size></file></files>
    </patch>
    <patch>
      <part>locale</part><version_from>0</version_from><version_to>__VERSION__</version_to>
      <files><file><name>locale.wgpkg</name><size>__LOCALE_SIZE__</size></file></files>
    </patch>
  </patches_chain>
  <web_seeds><url name="fixture" threads="1">__BASE_URL__/</url></web_seeds>
</protocol>"#;

/// Synthetic releases with different payload contents and installed identities.
#[derive(Clone, Copy)]
pub enum Release {
    /// Initial installation.
    V1,
    /// Full installation chain used to retry an in-place update.
    V2,
}

impl Release {
    /// Version recorded in the pinned manifest.
    pub fn version(self) -> &'static str {
        match self {
            Self::V1 => "1.0",
            Self::V2 => "2.0",
        }
    }

    /// Expected installed game-owned package, which must not be unpacked recursively.
    pub fn payload(self) -> &'static [u8] {
        match self {
            Self::V1 => {
                b"Synthetic game-owned asset package, version one. Keep this file packed.\n"
            }
            Self::V2 => {
                b"Synthetic game-owned asset package, version two. Keep this file packed.\n"
            }
        }
    }

    /// Expected installed language file.
    pub fn locale(self) -> &'static [u8] {
        match self {
            Self::V1 => b"English fixture, version one.\n",
            Self::V2 => b"English fixture, version two.\n",
        }
    }
}

/// Tiny real transport archives shared by tests and a future update service.
pub struct Fixture {
    /// Release represented by the archives.
    pub release: Release,
    /// Package filename to transport bytes, suitable for serving over HTTP.
    pub packages: BTreeMap<String, Vec<u8>>,
}

impl Fixture {
    /// Shared custom product configuration for a service mounted at `/updates`.
    pub fn product(base_url: &str) -> String {
        format!(
            r#"{{"game":"wot","region":"local","name":"Local test game","host":"{base_url}/updates","game_id":"LOCAL.WOT","channel":"testing"}}"#
        )
    }

    /// Single-region custom catalog for CLI workflows selected with `--region local`.
    pub fn products(base_url: &str) -> String {
        format!("[{}]", Self::product(base_url))
    }

    /// Two-region catalog pointing at independently hosted releases.
    pub fn catalog(eu_url: &str, na_url: &str) -> String {
        format!(
            r#"[{{"game":"wot","region":"eu","name":"Local EU game","host":"{eu_url}/updates","game_id":"LOCAL.WOT","channel":"testing"}},{{"game":"wot","region":"na","name":"Local NA game","host":"{na_url}/updates","game_id":"LOCAL.WOT","channel":"testing"}}]"#
        )
    }

    /// Package resources plus metadata and patch-chain XML beneath a URL path prefix.
    /// The server replaces the XML's origin placeholder after binding its port.
    pub fn service_resources(&self, prefix: &str) -> BTreeMap<String, Vec<u8>> {
        let prefix = prefix.trim_matches('/');
        let route = |name| {
            if prefix.is_empty() {
                format!("api/v1/{name}/")
            } else {
                format!("{prefix}/api/v1/{name}/")
            }
        };
        let mut resources = self.packages.clone();
        resources.insert(route("metadata"), METADATA.as_bytes().to_vec());
        resources.insert(
            route("patches_chain"),
            PATCH_CHAIN
                .replace("__VERSION__", self.release.version())
                .replace(
                    "__CLIENT_SIZE__",
                    &self.packages["client.wgpkg"].len().to_string(),
                )
                .replace(
                    "__LOCALE_SIZE__",
                    &self.packages["locale.wgpkg"].len().to_string(),
                )
                .into_bytes(),
        );
        resources
    }

    /// Builds real 7z transport volumes in a caller-owned scratch directory.
    pub fn build(release: Release, scratch: &Path) -> Self {
        fs::create_dir_all(scratch).unwrap();
        let mut packages = BTreeMap::new();
        for (part, name, contents) in [
            ("client", "payload.pkg", release.payload()),
            ("locale", "locale.txt", release.locale()),
        ] {
            let source = scratch.join(name);
            fs::write(&source, contents).unwrap();
            let archive = scratch.join(format!("{part}.wgpkg"));
            sevenz_rust::compress_to_path(&source, &archive).unwrap();
            packages.insert(format!("{part}.wgpkg"), fs::read(archive).unwrap());
        }
        Self { release, packages }
    }
}
