use super::error::{ReleaseError as Error, Result};
use super::{Patch, PatchSource};
use crate::catalog::ComponentVersions;
use std::collections::{HashMap, HashSet};

pub(super) fn ordered_patches<'a>(
    component: &super::ComponentRelease<'a>,
    base_versions: &ComponentVersions,
) -> Result<Vec<&'a Patch>> {
    let mut ordered = Vec::new();
    let component_id = component.id;
    let target = component.target_version();
    let mut source = PatchSource::from(base_versions.version_or_zero(component_id).to_owned());
    let transitions = PatchTransitions::new(component.patches());
    // A publisher may replace an obsolete delta chain with a full package.
    if source.version() != Some(target)
        && !transitions.contains_source(&source)
        && transitions.has_full_install()
    {
        source = PatchSource::Full;
    }
    let mut seen = HashSet::new();
    while source.version() != Some(target) {
        let version = source
            .version()
            .expect("planned sources are full or versioned");
        if !seen.insert(source.clone()) {
            return Err(Error::BrokenPatchChain {
                part: component_id.to_owned(),
                version: version.to_owned(),
            });
        }
        let next = transitions
            .successor_from(component_id, &source)?
            .ok_or_else(|| Error::MissingUpdatePatch {
                part: component_id.to_owned(),
                version: version.to_owned(),
                target: target.to_owned(),
            })?;
        ordered.push(next);
        source = PatchSource::from(next.version_to.clone());
    }
    Ok(ordered)
}

/// Published transitions indexed by source, retaining ambiguity until a step is needed.
struct PatchTransitions<'a> {
    by_source: HashMap<PatchSource, Vec<&'a Patch>>,
}

impl<'a> PatchTransitions<'a> {
    fn new(patches: impl Iterator<Item = &'a Patch>) -> Self {
        let mut by_source: HashMap<PatchSource, Vec<&Patch>> = HashMap::new();
        for patch in patches {
            if patch.source != PatchSource::Unspecified {
                by_source
                    .entry(patch.source.clone())
                    .or_default()
                    .push(patch);
            }
        }
        Self { by_source }
    }

    fn contains_source(&self, source: &PatchSource) -> bool {
        self.by_source.contains_key(source)
    }

    fn has_full_install(&self) -> bool {
        self.contains_source(&PatchSource::Full)
    }

    fn successor_from(&self, component: &str, source: &PatchSource) -> Result<Option<&'a Patch>> {
        match self.by_source.get(source).map(Vec::as_slice) {
            None => Ok(None),
            Some([patch]) => Ok(Some(*patch)),
            Some(_) => Err(Error::AmbiguousPatchTransition {
                part: component.to_owned(),
                version: source.version().unwrap_or("?").to_owned(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::ComponentRelease;
    use super::*;
    use crate::catalog::PatchSource;

    fn patch(from: &str, to: &str) -> Patch {
        Patch {
            component_id: "client".into(),
            source: PatchSource::from(from.to_owned()),
            version_to: to.into(),
            volumes: Vec::new(),
            torrent_hash: None,
            torrent_urls: Vec::new(),
        }
    }

    #[test]
    fn rejects_ambiguous_full_fallback_and_intermediate_transitions() {
        for (patches, starting, source) in [
            (vec![patch("0", "2"), patch("0", "2")], "0", "0"),
            (vec![patch("0", "2"), patch("0", "2")], "old", "0"),
            (
                vec![patch("0", "1"), patch("1", "2"), patch("1", "3")],
                "0",
                "1",
            ),
        ] {
            let component = ComponentRelease::new("client", Some("2"), &patches).unwrap();
            let versions = [("client".to_owned(), starting.to_owned())]
                .into_iter()
                .collect();
            assert!(matches!(
                ordered_patches(&component, &versions),
                Err(Error::AmbiguousPatchTransition { part, version })
                    if part == "client" && version == source
            ));
        }
    }

    #[test]
    fn ignores_unused_ambiguity_and_already_reached_targets() {
        let patches = [patch("0", "2"), patch("old", "1"), patch("old", "2")];
        let component = ComponentRelease::new("client", Some("2"), &patches).unwrap();
        let selected = ordered_patches(&component, &ComponentVersions::default()).unwrap();
        assert_eq!(selected, vec![&patches[0]]);
        let versions = [("client".to_owned(), "2".to_owned())]
            .into_iter()
            .collect();
        assert!(ordered_patches(&component, &versions).unwrap().is_empty());
    }

    #[test]
    fn prefers_available_delta_over_ambiguous_full_packages_and_ignores_unspecified_sources() {
        let mut unspecified = patch("1", "2");
        unspecified.source = PatchSource::Unspecified;
        let patches = [
            patch("0", "2"),
            patch("0", "3"),
            patch("1", "2"),
            unspecified,
        ];
        let component = ComponentRelease::new("client", Some("2"), &patches).unwrap();
        let versions = [("client".to_owned(), "1".to_owned())]
            .into_iter()
            .collect();
        assert_eq!(
            ordered_patches(&component, &versions).unwrap(),
            vec![&patches[2]]
        );
    }

    #[test]
    fn retains_full_fallback_and_cycle_detection() {
        let patches = [patch("0", "2")];
        let component = ComponentRelease::new("client", Some("2"), &patches).unwrap();
        let versions = [("client".to_owned(), "old".to_owned())]
            .into_iter()
            .collect();
        assert_eq!(
            ordered_patches(&component, &versions).unwrap(),
            vec![&patches[0]]
        );
        let cycle = [patch("0", "1"), patch("1", "0")];
        let component = ComponentRelease::new("client", Some("2"), &cycle).unwrap();
        assert!(matches!(
            ordered_patches(&component, &ComponentVersions::default()),
            Err(Error::BrokenPatchChain { .. })
        ));
    }
}
