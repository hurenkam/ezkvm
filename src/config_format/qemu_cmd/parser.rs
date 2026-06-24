//! Parser stage: qemu command text -> `QemuCommandSchema`.

use crate::config_format::{qemu_cmd::schema::QemuKnownFields, stages::Parser};

use super::schema::QemuCommandSchema;

/// Parses qemu command-file text into `QemuCommandSchema`.
pub struct QemuParser;

impl Parser for QemuParser {
    type Schema = QemuCommandSchema;
    type Error = String;

    fn parse(&self, source: &str) -> Result<QemuCommandSchema, String> {
        let tokens = tokenize_qemu_command(source)?;
        if tokens.is_empty() {
            return Err("qemu command is empty".to_string());
        }

        let executable = tokens[0].clone();
        let args = tokens[1..].to_vec();
        let known = parse_known_fields(&args);

        Ok(QemuCommandSchema::new(executable, args, known))
    }
}

pub(crate) fn parse_known_fields(args: &[String]) -> QemuKnownFields {
    let mut known = QemuKnownFields::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        let next = args.get(i + 1);

        match arg.as_str() {
            "-name" => {
                if let Some(value) = next {
                    known.name = Some(value.clone());
                    i += 2;
                    continue;
                }
            }
            "-machine" => {
                if let Some(value) = next {
                    known.machine = Some(value.clone());
                    i += 2;
                    continue;
                }
            }
            "-m" => {
                if let Some(value) = next {
                    known.memory_mb = parse_memory_mb(value);
                    i += 2;
                    continue;
                }
            }
            "-cpu" => {
                if let Some(value) = next {
                    known.cpu_model = Some(value.clone());
                    i += 2;
                    continue;
                }
            }
            "-smp" => {
                if let Some(value) = next {
                    let (cores, sockets, threads) = parse_smp(value);
                    known.cores = Some(cores);
                    known.sockets = Some(sockets);
                    known.threads = Some(threads);
                    i += 2;
                    continue;
                }
            }
            _ => {}
        }

        i += 1;
    }

    known
}

fn parse_memory_mb(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    if let Some(v) = trimmed.strip_suffix('M') {
        return v.parse::<u64>().ok();
    }
    if let Some(v) = trimmed.strip_suffix('G') {
        return v.parse::<u64>().ok().map(|gb| gb * 1024);
    }
    trimmed.parse::<u64>().ok()
}

fn parse_smp(value: &str) -> (u8, u8, u8) {
    let mut cores: u8 = 1;
    let mut sockets: u8 = 1;
    let mut threads: u8 = 1;

    for token in value.split(',') {
        if let Some(v) = token.strip_prefix("cores=") {
            cores = v.parse().unwrap_or(1);
        }
        if let Some(v) = token.strip_prefix("sockets=") {
            sockets = v.parse().unwrap_or(1);
        }
        if let Some(v) = token.strip_prefix("threads=") {
            threads = v.parse().unwrap_or(1);
        }
    }

    (cores, sockets, threads)
}

fn tokenize_qemu_command(source: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for ch in source.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' if !in_single => {
                escaped = true;
            }
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            c if c.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }

    if escaped {
        return Err("unterminated escape in qemu command".to_string());
    }
    if in_single || in_double {
        return Err("unterminated quote in qemu command".to_string());
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use crate::config_format::stages::Parser;

    use super::QemuParser;

    #[test]
    fn parses_basic_command_with_quoted_args() {
        let source =
            "qemu-system-x86_64 -name demo -m 4096M -cpu host -smp 4,sockets=1,cores=4,threads=1";
        let schema = QemuParser.parse(source).expect("command should parse");

        assert_eq!(schema.executable, "qemu-system-x86_64");
        assert_eq!(schema.known.name.as_deref(), Some("demo"));
        assert_eq!(schema.known.memory_mb, Some(4096));
        assert_eq!(schema.known.cpu_model.as_deref(), Some("host"));
        assert_eq!(schema.known.cores, Some(4));
    }

    #[test]
    fn rejects_unterminated_quote() {
        let source = "qemu-system-x86_64 -name 'demo";
        let err = QemuParser.parse(source).expect_err("parse should fail");
        assert!(err.contains("unterminated quote"));
    }
}
