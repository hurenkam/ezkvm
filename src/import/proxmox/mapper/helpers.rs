use super::super::model::ProxmoxVmConfig;
use std::collections::BTreeMap;

pub(super) fn parse_machine_and_options(machine: Option<&String>) -> (String, Vec<String>) {
	let raw = machine.map(String::as_str).unwrap_or("q35").trim();
	if raw.is_empty() {
		return ("q35".to_string(), Vec::new());
	}

	let mut parts = raw
		.split(',')
		.map(str::trim)
		.filter(|part| !part.is_empty())
		.collect::<Vec<_>>();

	if parts.is_empty() {
		return ("q35".to_string(), Vec::new());
	}

	if let Some(machine_part) = parts.first()
		&& let Some(machine_type) = machine_part.strip_prefix("type=")
	{
		let machine = machine_type.trim().to_string();
		parts.remove(0);
		return (machine, parts.iter().map(|p| p.to_string()).collect());
	}

	let machine = parts.remove(0).to_string();
	let options = parts.iter().map(|p| p.to_string()).collect();
	(machine, options)
}

pub(super) fn parse_cpu_model_and_features(cpu: Option<&String>) -> (String, Vec<String>) {
	let raw = cpu.map(String::as_str).unwrap_or("host").trim();
	if raw.is_empty() {
		return ("host".to_string(), Vec::new());
	}

	let mut parts = raw
		.split(',')
		.map(str::trim)
		.filter(|part| !part.is_empty())
		.collect::<Vec<_>>();

	if parts.is_empty() {
		return ("host".to_string(), Vec::new());
	}

	let model = parts.remove(0).to_string();
	let features = parts.iter().map(|p| p.to_string()).collect();
	(model, features)
}

pub(super) fn is_q35_machine(machine: &str) -> bool {
	machine == "q35" || machine.contains("q35")
}

pub(super) fn is_enabled(value: Option<&String>) -> bool {
	matches!(
		value.map(String::as_str),
		Some("1") | Some("on") | Some("yes") | Some("true")
	)
}

pub(super) fn infer_proxmox_vmid(proxmox: &ProxmoxVmConfig) -> Option<u32> {
	if let Some(vmid) = proxmox
		.scalars
		.get("vmid")
		.and_then(|value| value.trim().parse::<u32>().ok())
	{
		return Some(vmid);
	}

	for source in proxmox.disks.iter().map(|disk| disk.source.as_str()) {
		if let Some(vmid) = extract_vmid_from_source(source) {
			return Some(vmid);
		}
	}

	for key in ["efidisk0", "tpmstate0"] {
		if let Some(raw) = proxmox.scalars.get(key) {
			let (source, _) = parse_source_and_options(raw);
			if let Some(vmid) = extract_vmid_from_source(source) {
				return Some(vmid);
			}
		}
	}

	None
}

pub(super) fn extract_vmid_from_source(source: &str) -> Option<u32> {
	let marker = "vm-";
	let start = source.find(marker)? + marker.len();
	let tail = &source[start..];
	let digit_count = tail.chars().take_while(|ch| ch.is_ascii_digit()).count();
	if digit_count == 0 {
		return None;
	}
	tail[..digit_count].parse::<u32>().ok()
}

pub(super) fn parse_source_and_options(raw: &str) -> (&str, BTreeMap<String, String>) {
	let mut tokens = raw.split(',').map(str::trim).filter(|t| !t.is_empty());
	let source = tokens.next().unwrap_or("");
	let mut options = BTreeMap::new();

	for token in tokens {
		if let Some((k, v)) = token.split_once('=') {
			options.insert(k.trim().to_string(), v.trim().to_string());
		}
	}

	(source, options)
}

pub(super) fn parse_human_size_to_bytes(raw: &str) -> Option<u64> {
	let trimmed = raw.trim();
	if trimmed.is_empty() {
		return None;
	}

	let mut digits_end = 0;
	for (idx, ch) in trimmed.char_indices() {
		if ch.is_ascii_digit() {
			digits_end = idx + ch.len_utf8();
		} else {
			break;
		}
	}

	if digits_end == 0 {
		return None;
	}

	let number = trimmed[..digits_end].parse::<u64>().ok()?;
	let suffix = trimmed[digits_end..].trim().to_ascii_lowercase();

	let multiplier = match suffix.as_str() {
		"" | "b" => 1,
		"k" | "kb" => 1024,
		"m" | "mb" => 1024_u64.pow(2),
		"g" | "gb" => 1024_u64.pow(3),
		"t" | "tb" => 1024_u64.pow(4),
		_ => return None,
	};

	number.checked_mul(multiplier)
}

pub(super) fn shell_split(raw: &str) -> Vec<String> {
	let mut tokens = Vec::new();
	let mut current = String::new();
	let mut in_single = false;
	let mut in_double = false;

	for ch in raw.chars() {
		match ch {
			'\'' if !in_double => in_single = !in_single,
			'"' if !in_single => in_double = !in_double,
			c if c.is_whitespace() && !in_single && !in_double => {
				if !current.is_empty() {
					tokens.push(std::mem::take(&mut current));
				}
			}
			_ => current.push(ch),
		}
	}

	if !current.is_empty() {
		tokens.push(current);
	}

	tokens
}

pub(super) fn parse_prefixed_options(raw: &str) -> (String, BTreeMap<String, String>) {
	let mut tokens = raw.split(',').map(str::trim).filter(|t| !t.is_empty());
	let prefix = tokens.next().unwrap_or("").to_string();
	let mut options = BTreeMap::new();

	for token in tokens {
		if let Some((k, v)) = token.split_once('=') {
			options.insert(k.trim().to_string(), v.trim().to_string());
		}
	}

	(prefix, options)
}

pub(super) fn parse_options(raw: &str) -> BTreeMap<String, String> {
	raw.split(',')
		.map(str::trim)
		.filter(|token| !token.is_empty())
		.filter_map(|token| {
			token
				.split_once('=')
				.map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
		})
		.collect()
}
