//! Explain why a published release cannot resume the saved installation work.

use crate::catalog::ResolvedRelease;

pub(super) fn differing_release_fields(
    release: &ResolvedRelease,
    other: &ResolvedRelease,
) -> Vec<&'static str> {
    let mut fields = Vec::new();
    macro_rules! compare {
        ($($field:ident => $label:literal),* $(,)?) => {$(
            if release.$field != other.$field { fields.push($label); }
        )*};
    }
    compare!(
        identity => "build identity", product => "product", resolved_hostname => "publisher endpoint",
        available_languages => "available languages", publisher_ids => "publisher identifiers",
        component_ids => "selected components", component_versions => "component targets",
        patches => "patch chains", web_seeds => "download endpoints", volumes => "transport volumes",
    );
    fields
}
