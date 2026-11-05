use crate::terminal::{command, human_bytes};
use comfy_table::{
    Attribute, Cell, CellAlignment, Color, ContentArrangement, Table, presets::UTF8_FULL,
};
use wg_update_client::{BuildSummary, InspectReport};

pub(crate) fn print_install_hint(product: Option<&std::path::Path>) {
    let product = product
        .map(|path| format!(" --product \"{}\"", path.display()))
        .unwrap_or_default();
    let invocation = format!(
        "wg-launcher install{product} --game <game> --region <region> --client-type <client-type> --language <language>"
    );
    println!("Install with: {}", command(&invocation));
}

pub(crate) fn print_inspection(report: &InspectReport) {
    let architecture = report.build.architecture.as_deref().unwrap_or("unknown");
    println!("Game:          {}", report.build.name);
    println!("Application:   {}", report.build.game_id);
    println!("Region:        {}", report.build.region);
    println!("Channel:       {}", report.build.channel);
    println!("Version:       {}", report.build.version);
    println!("Client:        {}", report.build.client_type);
    println!("Language:      {}", report.build.language);
    println!("Target:        {} / {architecture}", report.build.platform);
    println!("Parts:         {}", report.parts.join(", "));
    println!("Languages:     {}", report.available_languages.join(", "));
    println!("Packages:      {}", report.package_count);
    println!("Download size: {}", human_bytes(report.compressed_bytes));
    println!(
        "Default path:  {}",
        report.default_output_directory.display()
    );
    println!("\nPublished patch chain\n");
    println!("{}", patch_table(report));
}

pub(crate) fn patch_table(report: &InspectReport) -> Table {
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

    for patch in &report.patches {
        let size = patch.compressed_bytes;
        table.add_row(vec![
            Cell::new(&patch.part).add_attribute(Attribute::Bold),
            Cell::new(patch.version_from.as_deref().unwrap_or("unknown")),
            Cell::new(&patch.version_to),
            Cell::new(patch.package_count).set_alignment(CellAlignment::Right),
            Cell::new(human_bytes(size)).set_alignment(CellAlignment::Right),
        ]);
    }

    table
}

pub(crate) fn build_table(summaries: &[BuildSummary]) -> Table {
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
