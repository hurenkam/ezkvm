use std::path::PathBuf;
use super::options::{CliOptions, OutputSpec, SourceSpec};

pub fn parse_cli_options(args: &[String]) -> Result<CliOptions, String> {
    let mut source: Option<SourceSpec> = None;
    let mut output: Option<OutputSpec> = None;
    let mut validate = false;
    let mut show_runtime = false;
    let mut unknown_args: Vec<String> = Vec::new();

    for arg in args {
        if let Some(parsed_source) = parse_source_flag(arg)? {
            if source.replace(parsed_source).is_some() {
                return Err("multiple import/input source flags were provided".to_string());
            }
            continue;
        }

        if let Some(parsed_output) = parse_output_flag(arg)? {
            if output.replace(parsed_output).is_some() {
                return Err("multiple output flags were provided".to_string());
            }
            continue;
        }

        match arg.as_str() {
            "--validate" => validate = true,
            "--show-runtime" => show_runtime = true,
            _ => unknown_args.push(arg.clone()),
        }
    }

    if !unknown_args.is_empty() {
        return Err(format!(
            "unrecognized arguments: {}",
            unknown_args.join(", ")
        ));
    }

    let source = source.ok_or("missing source; use --input:type=... or --import:type=...")?;

    if !validate && !show_runtime && output.is_none() {
        return Err(
            "no action requested; use --validate, --show-runtime, or --output:type=...".to_string(),
        );
    }

    Ok(CliOptions {
        source,
        output,
        validate,
        show_runtime,
    })
}

pub fn parse_source_flag(arg: &str) -> Result<Option<SourceSpec>, String> {
    const INPUT_PREFIX: &str = "--input:type=";
    const IMPORT_PREFIX: &str = "--import:type=";
    const LEGACY_IMPORT_PREFIX: &str = "--import:";

    let payload = if let Some(value) = arg.strip_prefix(INPUT_PREFIX) {
        value
    } else if let Some(value) = arg.strip_prefix(IMPORT_PREFIX) {
        value
    } else if let Some(value) = arg.strip_prefix(LEGACY_IMPORT_PREFIX) {
        value
    } else {
        return Ok(None);
    };

    let mut tokens = payload.split(',').filter(|s| !s.trim().is_empty());
    let importer = tokens
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("source flag requires an importer type")?
        .to_string();

    let importer_args: Vec<String> = tokens.map(|token| token.trim().to_string()).collect();
    let source_config_path = infer_config_path(&importer_args);

    Ok(Some(SourceSpec {
        importer,
        importer_args,
        source_config_path,
    }))
}

pub fn parse_output_flag(arg: &str) -> Result<Option<OutputSpec>, String> {
    const OUTPUT_PREFIX: &str = "--output:type=";

    let Some(payload) = arg.strip_prefix(OUTPUT_PREFIX) else {
        return Ok(None);
    };

    let mut tokens = payload.split(',').filter(|token| !token.trim().is_empty());
    let output_type = tokens
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("output flag requires an output type")?
        .to_string();

    let mut output_path: Option<PathBuf> = None;
    for token in tokens {
        let Some((key, value)) = token.split_once('=') else {
            return Err(format!(
                "invalid output argument '{}'; expected key=value",
                token
            ));
        };

        if key != "path" || value.trim().is_empty() || output_path.is_some() {
            return Err(format!("unsupported output argument '{}'", token));
        }

        output_path = Some(PathBuf::from(value.trim()));
    }

    Ok(Some(OutputSpec {
        output_type,
        output_path,
    }))
}

fn infer_config_path(importer_args: &[String]) -> Option<PathBuf> {
    for arg in importer_args {
        if let Some(value) = arg.strip_prefix("config=")
            && !value.trim().is_empty()
        {
            return Some(PathBuf::from(value.trim()));
        }
    }

    importer_args.first().and_then(|value| {
        if value.contains('=') {
            None
        } else {
            Some(PathBuf::from(value))
        }
    })
}
