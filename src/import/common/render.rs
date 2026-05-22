pub fn apply_optional_compaction<E, F>(
    yaml: String,
    compact_lists: bool,
    compact_fn: F,
) -> Result<String, E>
where
    F: FnOnce(&str) -> Result<String, E>,
{
    if compact_lists {
        return compact_fn(&yaml);
    }

    Ok(yaml)
}

pub fn prepend_preamble(yaml: String, preamble: Option<&str>) -> String {
    if let Some(text) = preamble
        && !text.is_empty()
    {
        return format!("{}\n{}", text, yaml);
    }

    yaml
}
