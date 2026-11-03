use comfy_table::{Cell, CellAlignment, ContentArrangement, Table, presets::UTF8_FULL};
use console::{Term, style};

pub(in crate::cli) fn print_cli_version(
    current: &str,
    latest: Option<&str>,
    update_available: bool,
    instruction: &str,
) {
    println!("wg-launcher");
    if update_available {
        println!("Installed:    {}", style(current).yellow());
        println!(
            "Latest:       {}",
            style(latest.unwrap_or("unavailable")).green().bold()
        );
    } else {
        println!("Installed:    {current}");
        println!("Latest:       {}", latest.unwrap_or("unavailable"));
    }
    println!(
        "Changelog:    {}",
        concat!(env!("CARGO_PKG_REPOSITORY"), "/releases/latest")
    );
    if update_available {
        println!(
            "{} {instruction}",
            style("Update available.").green().bold()
        );
    } else if latest.is_some() {
        println!("{}", style("Up to date.").green());
    }
}

pub(in crate::cli) fn print_cli_updated(version: &str) {
    println!(
        "{} wg-launcher has been updated to {}.",
        style("✓").green().bold(),
        style(version).green().bold(),
    );
    println!(
        "Changelog: {}",
        concat!(env!("CARGO_PKG_REPOSITORY"), "/releases/latest")
    );
}

pub(in crate::cli) fn print_cli_update(current: &str, latest: &str, instruction: &str) {
    let available = style("CLI update available!").green().bold().for_stderr();
    let current = style(current).yellow().for_stderr();
    let latest = style(latest).green().bold().for_stderr();
    let changelog = concat!(env!("CARGO_PKG_REPOSITORY"), "/releases/latest");
    let content =
        format!("\n{available} {current} → {latest}\n\nChangelog: {changelog}\n{instruction}\n");
    let mut table = Table::new();
    table
        .load_style(UTF8_FULL.with_rounded_corners())
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(Term::stderr().size().1.min(100))
        .add_row(vec![
            Cell::new(content).set_alignment(CellAlignment::Center),
        ]);
    if let Some(column) = table.column_mut(0) {
        column.set_padding((3, 3));
    }
    eprintln!("\n{table}\n");
}
