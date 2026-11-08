//! Compare the published release fields that must match to resume saved work.

use crate::catalog::ResolvedRelease;
use std::fmt;

/// Facts explaining why saved work cannot resume a requested release.
#[derive(Debug)]
pub struct ReleaseDifferences(Vec<ReleaseField>);

#[derive(Debug)]
enum ReleaseField {
    BuildIdentity,
    Product,
    PublisherEndpoint,
    AvailableLanguages,
    PublisherIdentifiers,
    SelectedComponents,
    ComponentTargets,
    PatchChains,
    DownloadEndpoints,
    TransportVolumes,
}

impl ReleaseDifferences {
    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for ReleaseDifferences {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, field) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            f.write_str(match field {
                ReleaseField::BuildIdentity => "build identity",
                ReleaseField::Product => "product",
                ReleaseField::PublisherEndpoint => "publisher endpoint",
                ReleaseField::AvailableLanguages => "available languages",
                ReleaseField::PublisherIdentifiers => "publisher identifiers",
                ReleaseField::SelectedComponents => "selected components",
                ReleaseField::ComponentTargets => "component targets",
                ReleaseField::PatchChains => "patch chains",
                ReleaseField::DownloadEndpoints => "download endpoints",
                ReleaseField::TransportVolumes => "transport volumes",
            })?;
        }
        Ok(())
    }
}

pub(super) fn compare_releases(
    release: &ResolvedRelease,
    other: &ResolvedRelease,
) -> ReleaseDifferences {
    // Exhaustive destructuring forces every new release field into this resume rule.
    let ResolvedRelease {
        identity,
        product,
        resolved_hostname,
        available_languages,
        publisher_ids,
        component_ids,
        component_versions,
        patches,
        web_seeds,
        volumes,
    } = release;
    let mut fields = Vec::new();
    macro_rules! compare {
        ($($field:ident => $difference:ident),* $(,)?) => {$(
            if $field != &other.$field { fields.push(ReleaseField::$difference); }
        )*};
    }
    compare!(
        identity => BuildIdentity, product => Product, resolved_hostname => PublisherEndpoint,
        available_languages => AvailableLanguages, publisher_ids => PublisherIdentifiers,
        component_ids => SelectedComponents, component_versions => ComponentTargets,
        patches => PatchChains, web_seeds => DownloadEndpoints, volumes => TransportVolumes,
    );
    ReleaseDifferences(fields)
}
