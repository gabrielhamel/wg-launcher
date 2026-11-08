use super::Patch;
use super::error::{ReleaseError as Error, Result};
use crate::catalog::ComponentVersions;
use std::collections::{HashMap, HashSet};

pub(super) fn ordered_patches<'a>(
    component: &super::ComponentRelease<'a>,
    base_versions: &ComponentVersions,
) -> Result<Vec<&'a Patch>> {
    let mut ordered = Vec::new();
    let component_id = component.id;
    let target = component.target_version();
    let mut version = base_versions.version_or_zero(component_id);
    let transitions = PatchTransitions::new(component.patches());
    // A publisher may replace an obsolete delta chain with a full package.
    if version != target
        && !transitions.contains_source(version)
        && transitions.contains_source("0")
    {
        version = "0";
    }
    let mut seen = HashSet::new();
    while version != target {
        if !seen.insert(version) {
            return Err(Error::BrokenPatchChain {
                part: component_id.to_owned(),
                version: version.to_owned(),
            });
        }
        let next = transitions
            .unique_successor(component_id, version)?
            .ok_or_else(|| Error::MissingUpdatePatch {
                part: component_id.to_owned(),
                version: version.to_owned(),
                target: target.to_owned(),
            })?;
        ordered.push(next);
        version = &next.version_to;
    }
    Ok(ordered)
}

/// Published transitions indexed by source, retaining ambiguity until a step is needed.
struct PatchTransitions<'a> {
    by_source: HashMap<&'a str, Vec<&'a Patch>>,
}

impl<'a> PatchTransitions<'a> {
    fn new(patches: impl Iterator<Item = &'a Patch>) -> Self {
        let mut by_source: HashMap<&str, Vec<&Patch>> = HashMap::new();
        for patch in patches {
            if let Some(source) = patch.source.version() {
                by_source.entry(source).or_default().push(patch);
            }
        }
        Self { by_source }
    }

    fn contains_source(&self, source: &str) -> bool {
        self.by_source.contains_key(source)
    }

    fn unique_successor(&self, component: &str, source: &str) -> Result<Option<&'a Patch>> {
        match self.by_source.get(source).map(Vec::as_slice) {
            None => Ok(None),
            Some([patch]) => Ok(Some(*patch)),
            Some(_) => Err(Error::AmbiguousPatchTransition {
                part: component.to_owned(),
                version: source.to_owned(),
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
