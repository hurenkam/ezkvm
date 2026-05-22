use super::error::ImportError;
use super::model::{QemuCmdModel, QemuCmdOption, QemuCmdOptionValue, QemuCsvPart};

const CSV_FLAGS: &[&str] = &[
    "-drive", "-device", "-netdev", "-chardev", "-machine", "-cpu", "-object", "-spice",
];
const JSON_OR_CSV_FLAGS: &[&str] = &["-blockdev", "-object"];
const NO_VALUE_FLAGS: &[&str] = &[
    "-daemonize",
    "-nodefaults",
    "-no-shutdown",
    "-nographic",
    "-enable-kvm",
    "-S",
];

pub fn parse_qemu_cmd(input: &str) -> Result<QemuCmdModel, ImportError> {
    let tokens = tokenize_shell_line(input)?;
    if tokens.is_empty() {
        return Err(ImportError::ParseError(
            "qemu command line is empty".to_string(),
        ));
    }

    let executable = tokens[0].clone();
    let options = parse_options(&tokens)?;

    Ok(QemuCmdModel {
        executable,
        options,
    })
}

fn parse_options(tokens: &[String]) -> Result<Vec<QemuCmdOption>, ImportError> {
    let mut index = 1;
    let mut options = Vec::new();

    while index < tokens.len() {
        let flag = tokens[index].clone();
        if !flag.starts_with('-') {
            return Err(ImportError::ParseError(format!(
                "unexpected token '{}' at position {}; expected option flag",
                flag, index
            )));
        }

        let should_take_value = option_takes_value(&flag);
        let raw_value =
            if should_take_value && index + 1 < tokens.len() && !is_flag(&tokens[index + 1]) {
                index += 1;
                Some(tokens[index].clone())
            } else {
                None
            };

        let value = parse_option_value(&flag, raw_value.as_deref())?;
        options.push(QemuCmdOption {
            flag,
            raw_value,
            value,
        });
        index += 1;
    }

    Ok(options)
}

fn option_takes_value(flag: &str) -> bool {
    !NO_VALUE_FLAGS.contains(&flag)
}

fn is_flag(token: &str) -> bool {
    token.starts_with('-')
}

fn parse_option_value(
    flag: &str,
    raw_value: Option<&str>,
) -> Result<QemuCmdOptionValue, ImportError> {
    let Some(value) = raw_value else {
        return Ok(QemuCmdOptionValue::None);
    };

    if JSON_OR_CSV_FLAGS.contains(&flag) && looks_like_json(value) {
        let parsed_json = serde_json::from_str(value).map_err(|error| {
            ImportError::ParseError(format!(
                "failed to parse JSON payload for {}: {}",
                flag, error
            ))
        })?;
        return Ok(QemuCmdOptionValue::Json(parsed_json));
    }

    if CSV_FLAGS.contains(&flag) || JSON_OR_CSV_FLAGS.contains(&flag) {
        return Ok(QemuCmdOptionValue::Csv(parse_csv_parts(value)));
    }

    Ok(QemuCmdOptionValue::Scalar(value.to_string()))
}

fn looks_like_json(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.starts_with('{') && trimmed.ends_with('}')
}

fn parse_csv_parts(value: &str) -> Vec<QemuCsvPart> {
    value
        .split(',')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(|segment| match segment.split_once('=') {
            Some((key, val)) => QemuCsvPart::KeyValue {
                key: key.trim().to_string(),
                value: val.trim().to_string(),
            },
            None => QemuCsvPart::Bare(segment.to_string()),
        })
        .collect()
}

fn tokenize_shell_line(input: &str) -> Result<Vec<String>, ImportError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    for character in input.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }

        if !in_single_quote && character == '\\' {
            escaped = true;
            continue;
        }

        if !in_double_quote && character == '\'' {
            in_single_quote = !in_single_quote;
            continue;
        }

        if !in_single_quote && character == '"' {
            in_double_quote = !in_double_quote;
            continue;
        }

        if !in_single_quote && !in_double_quote && character.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            continue;
        }

        current.push(character);
    }

    if escaped {
        return Err(ImportError::ParseError(
            "unterminated escape sequence in qemu command".to_string(),
        ));
    }
    if in_single_quote || in_double_quote {
        return Err(ImportError::ParseError(
            "unterminated quoted string in qemu command".to_string(),
        ));
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::parse_qemu_cmd;
    use crate::import::qemu_cmd::model::{QemuCmdOptionValue, QemuCsvPart};

    #[test]
    fn parses_quoted_and_repeated_flags() {
        let parsed = parse_qemu_cmd(
            "/usr/bin/kvm -name 'vm,debug-threads=on' -device 'virtio-net-pci,id=net0' -device 'virtio-balloon-pci,id=balloon0'",
        )
        .expect("parse should succeed");

        assert_eq!(parsed.executable, "/usr/bin/kvm");
        assert_eq!(parsed.options_for_flag("-device").count(), 2);

        let name_option = parsed
            .options_for_flag("-name")
            .next()
            .expect("-name should be present");
        assert_eq!(
            name_option.value,
            QemuCmdOptionValue::Scalar("vm,debug-threads=on".to_string())
        );
    }

    #[test]
    fn parses_json_payload_options() {
        let parsed = parse_qemu_cmd(
            "/usr/bin/kvm -blockdev '{\"driver\":\"raw\",\"node-name\":\"pflash0\"}'",
        )
        .expect("parse should succeed");

        let blockdev = parsed
            .options_for_flag("-blockdev")
            .next()
            .expect("-blockdev should exist");

        match &blockdev.value {
            QemuCmdOptionValue::Json(value) => {
                assert_eq!(value.get("driver").and_then(|v| v.as_str()), Some("raw"));
                assert_eq!(
                    value.get("node-name").and_then(|v| v.as_str()),
                    Some("pflash0")
                );
            }
            _ => panic!("expected JSON payload for -blockdev"),
        }
    }

    #[test]
    fn parses_representative_fixtures_into_intermediate_model() {
        let fixtures = [
            "/home/hurenkam/Workspace/ezkvm/input/felucia/108.qemu.cmd",
            "/home/hurenkam/Workspace/ezkvm/input/zbp-server-mh2/201.qemu.cmd",
            "/home/hurenkam/Workspace/ezkvm/input/coruscant/505.qemu.cmd",
        ];
        let mut seen_drive_or_blockdev = false;
        let mut seen_device = false;
        let mut seen_netdev = false;
        let mut seen_chardev = false;
        let mut seen_machine = false;
        let mut seen_cpu = false;
        let mut seen_object = false;
        let mut seen_spice = false;

        for fixture in fixtures {
            let input = std::fs::read_to_string(fixture).expect("fixture should be readable");
            let parsed = parse_qemu_cmd(&input).expect("fixture should parse");

            let has_drive = parsed.options_for_flag("-drive").count() > 0;
            let has_blockdev = parsed.options_for_flag("-blockdev").count() > 0;
            seen_drive_or_blockdev |= has_drive || has_blockdev;
            seen_device |= parsed.options_for_flag("-device").count() > 0;
            seen_chardev |= parsed.options_for_flag("-chardev").count() > 0;
            seen_netdev |= parsed.options_for_flag("-netdev").count() > 0;
            seen_machine |= parsed.options_for_flag("-machine").count() > 0;
            seen_cpu |= parsed.options_for_flag("-cpu").count() > 0;
            seen_object |= parsed.options_for_flag("-object").count() > 0;
            seen_spice |= parsed.options_for_flag("-spice").count() > 0;
        }

        assert!(seen_drive_or_blockdev);
        assert!(seen_device);
        assert!(seen_netdev);
        assert!(seen_chardev);
        assert!(seen_machine);
        assert!(seen_cpu);
        assert!(seen_object);
        assert!(seen_spice);
    }

    #[test]
    fn parses_csv_into_bare_and_key_value_parts() {
        let parsed = parse_qemu_cmd("/usr/bin/kvm -cpu host,hv_time,+kvm_pv_eoi")
            .expect("parse should succeed");

        let cpu = parsed
            .options_for_flag("-cpu")
            .next()
            .expect("-cpu should exist");

        match &cpu.value {
            QemuCmdOptionValue::Csv(parts) => {
                assert_eq!(parts[0], QemuCsvPart::Bare("host".to_string()));
                assert_eq!(parts[1], QemuCsvPart::Bare("hv_time".to_string()));
                assert_eq!(parts[2], QemuCsvPart::Bare("+kvm_pv_eoi".to_string()));
            }
            _ => panic!("expected CSV option for -cpu"),
        }
    }

    #[test]
    fn reports_unterminated_quotes() {
        let error = parse_qemu_cmd("/usr/bin/kvm -name 'broken").expect_err("must fail");
        let message = error.to_string();
        assert!(message.contains("unterminated quoted string"));
    }

    #[test]
    fn reports_invalid_json_payload() {
        let error =
            parse_qemu_cmd("/usr/bin/kvm -blockdev '{\"driver\":}'").expect_err("must fail");
        let message = error.to_string();
        assert!(message.contains("failed to parse JSON payload"));
    }
}
