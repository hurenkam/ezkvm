pub(crate) fn substitute_env_vars(content: &str) -> anyhow::Result<String> {
    let mut result = content.to_string();

    // Find all ${VAR} patterns.
    let re = regex::Regex::new(r"\$\{([^}]+)\}").unwrap();
    let mut replacements = Vec::new();

    for cap in re.captures_iter(content) {
        let full_match = cap.get(0).unwrap();
        let var_name = cap.get(1).unwrap().as_str();

        match std::env::var(var_name) {
            Ok(value) => {
                replacements.push((full_match.as_str().to_string(), value));
            }
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "Environment variable '{}' not found",
                    var_name
                ));
            }
        }
    }

    for (pattern, value) in replacements {
        result = result.replace(&pattern, &value);
    }

    // Handle $VAR syntax too.
    let re_simple = regex::Regex::new(r"\$([A-Z_][A-Z0-9_]*)").unwrap();
    let mut replacements_simple = Vec::new();

    for cap in re_simple.captures_iter(&result) {
        let full_match = cap.get(0).unwrap();
        let var_name = cap.get(1).unwrap().as_str();

        // Skip if it's part of a ${VAR} pattern that was already processed.
        if result.contains(&format!("${{{}}}", var_name)) {
            continue;
        }

        match std::env::var(var_name) {
            Ok(value) => {
                replacements_simple.push((full_match.as_str().to_string(), value));
            }
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "Environment variable '{}' not found",
                    var_name
                ));
            }
        }
    }

    for (pattern, value) in replacements_simple {
        result = result.replace(&pattern, &value);
    }

    Ok(result)
}
