mod status;
pub(super) use status::print_status;

use comfy_table::{
    Attribute, Cell, CellAlignment, Color, ContentArrangement, Table, presets::UTF8_FULL,
};
use wg_update_client::{BuildSummary, PatchManifest};

pub(super) fn print_install_summary(report: &wg_update_client::InstallationReport) {
    println!(
        "complete: {} downloaded, {} already present, {} transferred, {} package cache released during installation",
        report.download.downloaded_files,
        report.download.skipped_files,
        human_bytes(report.download.downloaded_bytes),
        human_bytes(report.released_package_bytes),
    );
}

pub(super) fn print_plan(plan: &wg_update_client::InstallPlan) {
    println!(
        "{} {} {} {} {}: {} assets, {} -> {}",
        plan.manifest.product.game(),
        plan.manifest.region,
        plan.manifest.version,
        plan.manifest.client_type,
        plan.manifest.language,
        plan.assets.len(),
        human_bytes(plan.compressed_bytes),
        plan.root.display(),
    );
    println!("  package cache: {}", plan.cache.display());
}

pub(super) fn print_manifest(manifest: &PatchManifest) {
    let architecture = manifest.architecture.as_deref().unwrap_or("unknown");
    println!("Game:          {}", manifest.product.name);
    println!("Application:   {}", manifest.game_id);
    println!("Region:        {}", manifest.region);
    println!("Channel:       {}", manifest.channel);
    println!("Version:       {}", manifest.version);
    println!("Client:        {}", manifest.client_type);
    println!("Language:      {}", manifest.language);
    println!("Target:        {} / {architecture}", manifest.platform);
    println!("Parts:         {}", manifest.parts.join(", "));
    println!("Languages:     {}", manifest.available_languages.join(", "));
    println!("Packages:      {}", manifest.assets.len());
    println!("Download size: {}", human_bytes(manifest.download_size()));
    println!(
        "Default path:  {}",
        manifest.default_output_directory().display()
    );
    println!("\nPublished patch chain\n");
    println!("{}", patch_table(manifest));
}

pub(super) fn patch_table(manifest: &PatchManifest) -> Table {
    let mut table = Table::new();
    table
        .load_style(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(
            ["Part", "From", "To", "Packages", "Download size"].map(|heading| {
                Cell::new(heading)
                    .fg(Color::Cyan)
                    .add_attribute(Attribute::Bold)
            }),
        );

    for patch in &manifest.patches {
        let size = patch.files.iter().map(|file| file.size).sum();
        table.add_row(vec![
            Cell::new(&patch.part).add_attribute(Attribute::Bold),
            Cell::new(patch.version_from.as_deref().unwrap_or("unknown")),
            Cell::new(&patch.version_to),
            Cell::new(patch.files.len()).set_alignment(CellAlignment::Right),
            Cell::new(human_bytes(size)).set_alignment(CellAlignment::Right),
        ]);
    }

    table
}

pub(super) fn build_table(summaries: &[BuildSummary]) -> Table {
    let mut table = Table::new();
    table
        .load_style(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(
            [
                "Game",
                "Region",
                "Channel",
                "Client",
                "Version",
                "Target",
                "Size",
                "Languages",
            ]
            .map(|heading| {
                Cell::new(heading)
                    .fg(Color::Cyan)
                    .add_attribute(Attribute::Bold)
            }),
        );

    for item in summaries {
        let languages = item
            .languages
            .iter()
            .map(|language| {
                if language.eq_ignore_ascii_case(&item.default_language) {
                    format!("{language}*")
                } else {
                    language.clone()
                }
            })
            .collect::<Vec<_>>();
        let languages = languages
            .chunks(8)
            .map(|line| line.join(", "))
            .collect::<Vec<_>>()
            .join("\n");
        let target = format!(
            "{} / {}",
            item.platform,
            item.architecture.as_deref().unwrap_or("unknown")
        );
        table.add_row(vec![
            Cell::new(item.game.to_uppercase()).add_attribute(Attribute::Bold),
            Cell::new(item.region.to_string().to_uppercase()).add_attribute(Attribute::Bold),
            Cell::new(&item.channel),
            Cell::new(item.client_type.to_uppercase()),
            Cell::new(&item.version),
            Cell::new(target),
            Cell::new(human_bytes(item.compressed_bytes)).set_alignment(CellAlignment::Right),
            Cell::new(languages),
        ]);
    }

    table
}

pub(super) fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
