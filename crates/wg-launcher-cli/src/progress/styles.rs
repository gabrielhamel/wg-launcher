use indicatif::{ProgressStyle, style::TemplateError};

pub(super) fn overall_style() -> Result<ProgressStyle, TemplateError> {
    Ok(ProgressStyle::with_template(
        "{spinner:.green} {msg:.bold} [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} • {bytes_per_sec} • ETA {eta}",
    )?
    .progress_chars("━━╸")
    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "✓"]))
}

pub(super) fn spinner_style() -> Result<ProgressStyle, TemplateError> {
    Ok(
        ProgressStyle::with_template("{spinner:.green} {msg:.bold}")?
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    )
}

pub(super) fn file_style() -> Result<ProgressStyle, TemplateError> {
    Ok(ProgressStyle::with_template(
        "  {spinner:.green} {msg:38} [{bar:24.cyan/blue}] {bytes:>9}/{total_bytes:9} {bytes_per_sec:>10} ETA {eta:>4}",
    )?
    .progress_chars("━━╸")
    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "✓"]))
}

pub(super) fn error_style() -> Result<ProgressStyle, TemplateError> {
    ProgressStyle::with_template("  {msg:.bold.red}")
}

pub(super) fn extraction_style() -> Result<ProgressStyle, TemplateError> {
    Ok(ProgressStyle::with_template(
        "{spinner:.green} {msg:.bold} {wide_bar:.cyan/blue} {pos}/{len}",
    )?
    .progress_chars("━━╸")
    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "✓"]))
}

pub(super) fn extraction_completed_style() -> ProgressStyle {
    ProgressStyle::with_template("{msg:.bold.green}")
        .unwrap_or_else(|_| ProgressStyle::default_spinner())
}
