use std::path::PathBuf;

use super::{ConfigArgs, ConfigImportError};

fn tokenize_import_args(config_args: ConfigArgs) -> Vec<String> {
    config_args
        .args
        .into_iter()
        .flat_map(|arg| {
            arg.split(',')
                .map(str::trim)
                .filter(|segment| !segment.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .collect()
}

pub(crate) fn extract_config_path(
    importer: &'static str,
    config_args: ConfigArgs,
) -> Result<(PathBuf, Vec<String>), ConfigImportError> {
    let tokens = tokenize_import_args(config_args);
    let mut config_path: Option<PathBuf> = None;
    let mut leftovers: Vec<String> = Vec::new();

    for token in tokens {
        if let Some((key, value)) = token.split_once('=') {
            if key == "config" {
                if value.is_empty() || config_path.is_some() {
                    return Err(ConfigImportError::UnexpectedArgs {
                        importer,
                        args: vec![token],
                    });
                }

                config_path = Some(PathBuf::from(value));
            } else {
                leftovers.push(token);
            }
        } else if config_path.is_none() {
            config_path = Some(PathBuf::from(token));
        } else {
            leftovers.push(token);
        }
    }

    let config_path = config_path.ok_or(ConfigImportError::MissingConfigPath { importer })?;
    Ok((config_path, leftovers))
}
