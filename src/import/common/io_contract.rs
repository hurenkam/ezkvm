use std::path::Path;

pub fn default_output_path(input_path: &str) -> String {
    let input = Path::new(input_path);
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("imported-vm");

    format!("{}.yaml", stem)
}

pub fn write_output_if_needed(
    output_path: &str,
    rendered_yaml: &str,
    dry_run: bool,
) -> Result<(), String> {
    if dry_run {
        return Ok(());
    }

    std::fs::write(output_path, rendered_yaml)
        .map_err(|e| format!("unable to write output file '{}': {}", output_path, e))
}

pub fn enforce_strict_mode<T, F>(
    strict: bool,
    warnings: &[T],
    format_warning: F,
) -> Result<(), String>
where
    F: Fn(&T) -> String,
{
    if !strict || warnings.is_empty() {
        return Ok(());
    }

    let joined = warnings
        .iter()
        .map(format_warning)
        .collect::<Vec<_>>()
        .join("; ");

    Err(format!(
        "strict import failed due to {} warning(s): {}",
        warnings.len(),
        joined
    ))
}
