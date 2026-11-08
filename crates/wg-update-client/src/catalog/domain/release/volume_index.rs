//! Bind selected archive declarations to unambiguous, consistent download sources.

use super::{
    Patch, PatchVolume, TransportVolume,
    error::{ReleaseError, Result},
};
use std::collections::HashMap;

pub(super) struct VolumeIndex<'a> {
    by_name: HashMap<&'a str, Vec<&'a TransportVolume>>,
}

impl<'a> VolumeIndex<'a> {
    pub(super) fn new(volumes: &'a [TransportVolume]) -> Self {
        let mut by_name: HashMap<_, Vec<_>> = HashMap::new();
        for volume in volumes {
            by_name
                .entry(volume.name.as_str())
                .or_default()
                .push(volume);
        }
        Self { by_name }
    }

    /// Validate only volumes in the chosen chain; unused alternatives remain unchecked.
    pub(super) fn bind(
        &self,
        patch: &Patch,
        declaration: &PatchVolume,
    ) -> Result<&'a TransportVolume> {
        let candidates = self
            .by_name
            .get(declaration.name.as_str())
            .ok_or_else(|| ReleaseError::MissingPatchAsset(declaration.name.clone()))?;
        let mut matching = candidates.iter().copied().filter(|volume| {
            volume.component_id == patch.component_id
                && volume.source == patch.source
                && volume.version_to == patch.version_to
        });
        let conflict = || ReleaseError::ConflictingPatchAsset(declaration.name.clone());
        let volume = matching.next().ok_or_else(conflict)?;
        if matching.next().is_some()
            || volume.size != declaration.size
            || volume.unpacked_size != declaration.unpacked_size
            || volume.diffs_size != declaration.diffs_size
        {
            return Err(conflict());
        }
        Ok(volume)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::PatchSource;

    fn patch() -> Patch {
        Patch {
            component_id: "client".into(),
            source: PatchSource::Full,
            version_to: "1.0".into(),
            torrent_hash: None,
            torrent_urls: Vec::new(),
            volumes: vec![PatchVolume {
                name: "client.wgpkg".into(),
                size: 100,
                unpacked_size: Some(200),
                diffs_size: None,
            }],
        }
    }

    fn volume() -> TransportVolume {
        TransportVolume {
            component_id: "client".into(),
            source: PatchSource::Full,
            version_to: "1.0".into(),
            name: "client.wgpkg".into(),
            url: "https://example.test/client.wgpkg".parse().unwrap(),
            size: 100,
            unpacked_size: Some(200),
            diffs_size: None,
        }
    }

    #[test]
    fn binds_by_patch_identity_and_declaration_instead_of_last_filename_match() {
        let patch = patch();
        let mut other_component = volume();
        other_component.component_id = "locale".into();
        let candidates = [volume(), other_component];
        let index = VolumeIndex::new(&candidates);
        assert_eq!(
            index.bind(&patch, &patch.volumes[0]).unwrap(),
            &candidates[0]
        );
    }

    #[test]
    fn rejects_ambiguous_sources_and_inconsistent_archive_facts() {
        let patch = patch();
        let duplicates = [volume(), volume()];
        assert!(matches!(
            VolumeIndex::new(&duplicates).bind(&patch, &patch.volumes[0]),
            Err(ReleaseError::ConflictingPatchAsset(_))
        ));
        let mut mismatches = Vec::new();
        let mut wrong = volume();
        wrong.component_id = "locale".into();
        mismatches.push(wrong);
        let mut wrong = volume();
        wrong.source = PatchSource::from("0.5".to_owned());
        mismatches.push(wrong);
        let mut wrong = volume();
        wrong.version_to = "2.0".into();
        mismatches.push(wrong);
        let mut wrong = volume();
        wrong.size = 99;
        mismatches.push(wrong);
        let mut wrong = volume();
        wrong.unpacked_size = None;
        mismatches.push(wrong);
        let mut wrong = volume();
        wrong.diffs_size = Some(1);
        mismatches.push(wrong);
        for wrong in mismatches {
            assert!(matches!(
                VolumeIndex::new(&[wrong]).bind(&patch, &patch.volumes[0]),
                Err(ReleaseError::ConflictingPatchAsset(_))
            ));
        }
    }

    #[test]
    fn ignores_unused_conflicts_but_reports_missing_selected_volumes() {
        let patch = patch();
        let mut unused = volume();
        unused.name = "unused.wgpkg".into();
        let candidates = [volume(), unused.clone(), unused];
        assert!(
            VolumeIndex::new(&candidates)
                .bind(&patch, &patch.volumes[0])
                .is_ok()
        );
        assert!(matches!(
            VolumeIndex::new(&[]).bind(&patch, &patch.volumes[0]),
            Err(ReleaseError::MissingPatchAsset(_))
        ));
    }
}
