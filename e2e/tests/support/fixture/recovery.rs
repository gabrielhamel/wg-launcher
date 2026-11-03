//! Larger but bounded real archives make interrupted phases observable.
use std::{fs, io::Cursor, path::Path};

pub const FILE_COUNT: usize = 600;
pub const TRANSFER_SIZE: usize = 8 * 1024 * 1024;

pub fn archive(scratch: &Path, entries: impl IntoIterator<Item = (String, Vec<u8>)>) -> Vec<u8> {
    fs::create_dir_all(scratch).unwrap();
    let path = scratch.join("recovery.7z");
    let mut writer = sevenz_rust::SevenZWriter::new(fs::File::create(&path).unwrap()).unwrap();
    writer.set_content_methods(vec![sevenz_rust::lzma::LZMA2Options::with_preset(0).into()]);
    for (name, bytes) in entries {
        let mut entry = sevenz_rust::SevenZArchiveEntry::new();
        entry.name = name;
        entry.has_stream = true;
        writer
            .push_archive_entry(entry, Some(Cursor::new(bytes)))
            .unwrap();
    }
    writer.finish().unwrap();
    fs::read(path).unwrap()
}

pub fn filename(index: usize) -> String {
    format!("files/{index:04}.pkg")
}

/// Deterministic incompressible bytes keep the HTTP transfer observable.
pub fn transfer_payload() -> Vec<u8> {
    let mut seed = 0x123456789abcdef0_u64;
    (0..TRANSFER_SIZE)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed as u8
        })
        .collect()
}
