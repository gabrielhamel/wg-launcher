use comfy_table::{Cell, CellAlignment, ContentArrangement, Table, presets::UTF8_FULL};
use console::{Term, style};

pub(in crate::cli) fn print_cli_update(current: &str, latest: &str, instruction: &str) {
    let available = style("CLI update available!").green().bold().for_stderr();
    let current = style(current).yellow().for_stderr();
    let latest = style(latest).green().bold().for_stderr();
    let changelog = concat!(env!("CARGO_PKG_REPOSITORY"), "/releases/latest");
    let instruction = instruction
        .split('`')
        .enumerate()
        .map(|(index, text)| {
            if index % 2 == 1 {
                style(text).cyan().bold().for_stderr().to_string()
            } else {
                text.to_owned()
            }
        })
        .collect::<String>();
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
