//! cargo-dist compatible manifests and tiny replacement archives, independent of CLI models.
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{Cursor, Write},
};

pub const FUTURE: &str = "99.0.0";

pub fn resources(
    mut resources: BTreeMap<String, Vec<u8>>,
    version: &str,
    corrupt: bool,
    bad_version: bool,
) -> BTreeMap<String, Vec<u8>> {
    let (target, extension) = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => ("x86_64-pc-windows-msvc", "zip"),
        ("macos", "aarch64") => ("aarch64-apple-darwin", "tar.gz"),
        ("macos", "x86_64") => ("x86_64-apple-darwin", "tar.gz"),
        ("linux", "x86_64") => ("x86_64-unknown-linux-gnu", "tar.gz"),
        _ => panic!("unsupported E2E host"),
    };
    let name = format!("wg-launcher-cli-{target}.{extension}");
    let checksum = format!("{name}.sha256");
    let payload = if bad_version {
        std::fs::read(crate::support::binary("wg-launcher")).unwrap()
    } else {
        std::fs::read(env!("CARGO_BIN_EXE_cli-update-fixture")).unwrap()
    };
    let binary_name = if cfg!(windows) {
        "wg-launcher.exe"
    } else {
        "wg-launcher"
    };
    let archive = if cfg!(windows) {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file(
            binary_name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
        zip.write_all(&payload).unwrap();
        zip.finish().unwrap().into_inner()
    } else {
        let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        let mut archive = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_mode(0o755);
        header.set_size(payload.len() as u64);
        header.set_cksum();
        archive
            .append_data(
                &mut header,
                format!("release/{binary_name}"),
                payload.as_slice(),
            )
            .unwrap();
        archive.into_inner().unwrap().finish().unwrap()
    };
    let digest = if corrupt {
        "0".repeat(64)
    } else {
        format!("{:x}", Sha256::digest(&archive))
    };
    resources.insert(checksum.clone(), format!("{digest}  {name}\n").into_bytes());
    resources.insert(name.clone(), archive);
    resources.insert("dist-manifest.json".into(), serde_json::to_vec(&serde_json::json!({
        "announcement_tag": format!("v{version}"),
        "announcement_is_prerelease": false,
        "releases": [{ "app_name": "wg-launcher-cli", "app_version": version, "artifacts": [name, checksum] }]
    })).unwrap());
    resources
}
