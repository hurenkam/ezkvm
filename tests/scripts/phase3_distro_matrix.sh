#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/../.." && pwd)"

usage() {
  cat <<'EOF'
Phase 3 distro validation harness (B-52)

Usage:
  tests/scripts/phase3_distro_matrix.sh [options]

Options:
  --distro-label <label>         Override detected distro label (default: auto)
  --output-dir <dir>             Output artifact root (default: artifacts/phase3/<timestamp>-<distro>)
  --ezkvm-bin <path>             Path to ezkvm binary (default: ./target/debug/ezkvm then PATH)
  --storage-cfg <path>           Proxmox storage cfg path (default: auto-detect)
  --linux-conf <path>            Linux fixture conf path
  --windows-conf <path>          Windows UEFI+TPM fixture conf path
  --mixed-conf <path>            Capability-heavy fixture conf path
  --parity-conf <path>           Fixture used for parity dry-run guard
  --with-optional                Run optional integration checks (non-gating)
  --smoke-boot                   Run smoke boot checks (slower, requires prepared host)
  --smoke-timeout-sec <seconds>  Smoke boot timeout per VM (default: 90)
  --keep-workdir                 Keep temporary harness workdir
  -h, --help                     Show this help

Environment variable alternatives:
  EZKVM_BIN, PHASE3_DISTRO_LABEL, PHASE3_OUTPUT_DIR,
  PHASE3_LINUX_CONF, PHASE3_WINDOWS_CONF, PHASE3_MIXED_CONF, PHASE3_PARITY_CONF,
  PHASE3_STORAGE_CFG, PHASE3_WITH_OPTIONAL, PHASE3_SMOKE_BOOT

Notes:
- This harness is designed for one host distro run at a time.
- Required checks are gating; optional checks are best-effort and do not fail the run.
- Smoke-boot checks are opt-in and rely on timeout-based execution to avoid hangs.
EOF
}

detect_distro_label() {
  if [[ -n "${PHASE3_DISTRO_LABEL:-}" ]]; then
    printf '%s\n' "$PHASE3_DISTRO_LABEL"
    return
  fi

  if [[ -r /etc/os-release ]]; then
    # shellcheck disable=SC1091
    source /etc/os-release
    local id="${ID:-unknown}"
    local ver="${VERSION_ID:-unknown}"
    printf '%s-%s\n' "$id" "$ver"
    return
  fi

  printf 'unknown-host\n'
}

resolve_ezkvm_bin() {
  if [[ -n "${EZKVM_BIN:-}" ]]; then
    printf '%s\n' "$EZKVM_BIN"
    return
  fi

  if [[ -x "$REPO_ROOT/target/debug/ezkvm" ]]; then
    printf '%s\n' "$REPO_ROOT/target/debug/ezkvm"
    return
  fi

  if command -v ezkvm >/dev/null 2>&1; then
    command -v ezkvm
    return
  fi

  printf ''
}

timestamp() {
  date -u +"%Y%m%dT%H%M%SZ"
}

DISTRO_LABEL="$(detect_distro_label)"
OUTPUT_DIR="${PHASE3_OUTPUT_DIR:-}"
EZKVM_PATH=""
WITH_OPTIONAL="${PHASE3_WITH_OPTIONAL:-0}"
SMOKE_BOOT="${PHASE3_SMOKE_BOOT:-0}"
SMOKE_TIMEOUT_SEC="90"
KEEP_WORKDIR=0

LINUX_CONF="${PHASE3_LINUX_CONF:-$REPO_ROOT/tests/fixtures/proxmox_import/01-scsi-bridge-tpm.conf}"
MIXED_CONF="${PHASE3_MIXED_CONF:-$REPO_ROOT/tests/fixtures/proxmox_import/11-nested-viommu-hidden.conf}"
PARITY_CONF="${PHASE3_PARITY_CONF:-$REPO_ROOT/tests/fixtures/proxmox_import/01-scsi-bridge-tpm.conf}"
WINDOWS_CONF_DEFAULT=""
if [[ -f "$REPO_ROOT/input/felucia/108.conf" ]]; then
  WINDOWS_CONF_DEFAULT="$REPO_ROOT/input/felucia/108.conf"
elif [[ -f "$REPO_ROOT/108.conf" ]]; then
  WINDOWS_CONF_DEFAULT="$REPO_ROOT/108.conf"
fi
WINDOWS_CONF="${PHASE3_WINDOWS_CONF:-$WINDOWS_CONF_DEFAULT}"

STORAGE_CFG="${PHASE3_STORAGE_CFG:-}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --distro-label)
      DISTRO_LABEL="$2"
      shift 2
      ;;
    --output-dir)
      OUTPUT_DIR="$2"
      shift 2
      ;;
    --ezkvm-bin)
      EZKVM_PATH="$2"
      shift 2
      ;;
    --storage-cfg)
      STORAGE_CFG="$2"
      shift 2
      ;;
    --linux-conf)
      LINUX_CONF="$2"
      shift 2
      ;;
    --windows-conf)
      WINDOWS_CONF="$2"
      shift 2
      ;;
    --mixed-conf)
      MIXED_CONF="$2"
      shift 2
      ;;
    --parity-conf)
      PARITY_CONF="$2"
      shift 2
      ;;
    --with-optional)
      WITH_OPTIONAL=1
      shift
      ;;
    --smoke-boot)
      SMOKE_BOOT=1
      shift
      ;;
    --smoke-timeout-sec)
      SMOKE_TIMEOUT_SEC="$2"
      shift 2
      ;;
    --keep-workdir)
      KEEP_WORKDIR=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ -z "$EZKVM_PATH" ]]; then
  EZKVM_PATH="$(resolve_ezkvm_bin)"
fi

if [[ -z "$EZKVM_PATH" ]]; then
  echo "ERROR: ezkvm binary not found. Build it (cargo build) or pass --ezkvm-bin." >&2
  exit 1
fi

if [[ ! -x "$EZKVM_PATH" ]]; then
  echo "ERROR: ezkvm binary is not executable: $EZKVM_PATH" >&2
  exit 1
fi

if [[ -z "$OUTPUT_DIR" ]]; then
  OUTPUT_DIR="$REPO_ROOT/artifacts/phase3/$(timestamp)-$DISTRO_LABEL"
fi

if [[ -z "$STORAGE_CFG" ]]; then
  if [[ -f "$REPO_ROOT/tests/fixtures/proxmox_import/storage.cfg" ]]; then
    STORAGE_CFG="$REPO_ROOT/tests/fixtures/proxmox_import/storage.cfg"
  elif [[ -f "$REPO_ROOT/storage.cfg" ]]; then
    STORAGE_CFG="$REPO_ROOT/storage.cfg"
  fi
fi

mkdir -p "$OUTPUT_DIR"
WORKDIR="$(mktemp -d "$OUTPUT_DIR/work.XXXXXX")"
SUMMARY_FILE="$OUTPUT_DIR/summary.txt"
ENV_FILE="$OUTPUT_DIR/environment.txt"

cleanup() {
  if [[ "$KEEP_WORKDIR" -eq 0 ]]; then
    rm -rf "$WORKDIR"
  fi
}
trap cleanup EXIT

assert_file() {
  local path="$1"
  local label="$2"
  if [[ ! -f "$path" ]]; then
    echo "ERROR: Missing $label file: $path" >&2
    exit 1
  fi
}

assert_file "$LINUX_CONF" "linux fixture"
assert_file "$MIXED_CONF" "mixed fixture"
assert_file "$PARITY_CONF" "parity fixture"
if [[ -z "$WINDOWS_CONF" ]]; then
  echo "ERROR: Windows fixture not resolved. Pass --windows-conf explicitly." >&2
  exit 1
fi
assert_file "$WINDOWS_CONF" "windows fixture"

capture_host_environment() {
  {
    echo "distro_label=$DISTRO_LABEL"
    echo "timestamp_utc=$(timestamp)"
    echo "repo_root=$REPO_ROOT"
    echo "ezkvm_bin=$EZKVM_PATH"
    echo "storage_cfg=${STORAGE_CFG:-<none>}"
    echo "linux_conf=$LINUX_CONF"
    echo "windows_conf=$WINDOWS_CONF"
    echo "mixed_conf=$MIXED_CONF"
    echo "parity_conf=$PARITY_CONF"
    echo "with_optional=$WITH_OPTIONAL"
    echo "smoke_boot=$SMOKE_BOOT"
    echo "smoke_timeout_sec=$SMOKE_TIMEOUT_SEC"
    if [[ -r /etc/os-release ]]; then
      echo
      echo "[os-release]"
      cat /etc/os-release
    fi
    echo
    echo "[tool-paths]"
    for tool in qemu-system-x86_64 swtpm qemu-bridge-helper looking-glass-client; do
      if command -v "$tool" >/dev/null 2>&1; then
        echo "$tool=$(command -v "$tool")"
      else
        echo "$tool=<missing>"
      fi
    done
    echo
    echo "[tool-versions]"
    if command -v qemu-system-x86_64 >/dev/null 2>&1; then
      qemu-system-x86_64 --version | head -n 1
    fi
    if command -v swtpm >/dev/null 2>&1; then
      swtpm --version | head -n 1
    fi
  } > "$ENV_FILE"
}

run_cmd_capture() {
  local out_file="$1"
  shift
  set +e
  "$@" >"$out_file" 2>&1
  local rc=$?
  set -e
  return $rc
}

record_summary() {
  local scenario="$1"
  local phase="$2"
  local status="$3"
  local detail="$4"
  printf '%s\t%s\t%s\t%s\n' "$scenario" "$phase" "$status" "$detail" >> "$SUMMARY_FILE"
}

import_with_target() {
  local conf="$1"
  local target="$2"
  local out_yaml="$3"
  local out_log="$4"

  local args=("$EZKVM_PATH" import-proxmox "$conf" --runtime-target "$target" --output "$out_yaml")
  if [[ -n "${STORAGE_CFG:-}" ]]; then
    args+=(--proxmox-storage "$STORAGE_CFG")
  fi

  if run_cmd_capture "$out_log" "${args[@]}"; then
    return 0
  fi
  return 1
}

import_dry_run_target() {
  local conf="$1"
  local target="$2"
  local out_log="$3"

  local args=("$EZKVM_PATH" import-proxmox "$conf" --runtime-target "$target" --dry-run)
  if [[ -n "${STORAGE_CFG:-}" ]]; then
    args+=(--proxmox-storage "$STORAGE_CFG")
  fi

  if run_cmd_capture "$out_log" "${args[@]}"; then
    return 0
  fi
  return 1
}

start_dry_run() {
  local yaml_path="$1"
  local out_log="$2"
  if run_cmd_capture "$out_log" "$EZKVM_PATH" start "$yaml_path" --dry-run; then
    return 0
  fi
  return 1
}

smoke_boot_check() {
  local yaml_path="$1"
  local out_log="$2"
  if [[ "$SMOKE_BOOT" -eq 0 ]]; then
    echo "smoke boot skipped (enable with --smoke-boot)" > "$out_log"
    return 0
  fi

  set +e
  timeout --preserve-status "$SMOKE_TIMEOUT_SEC" "$EZKVM_PATH" start "$yaml_path" >"$out_log" 2>&1
  local rc=$?
  set -e
  if [[ $rc -eq 0 || $rc -eq 143 || $rc -eq 124 ]]; then
    # 124 (timeout) and 143 (terminated) are acceptable for smoke validation mode.
    return 0
  fi
  return 1
}

run_required_scenario() {
  local scenario="$1"
  local conf="$2"
  local scenario_dir="$OUTPUT_DIR/$scenario"
  mkdir -p "$scenario_dir"

  local portable_yaml="$scenario_dir/${scenario}.portable.yaml"
  local import_log="$scenario_dir/import-portable.log"
  local import_dry_log="$scenario_dir/import-portable-dry-run.log"
  local start_log="$scenario_dir/start-portable-dry-run.log"
  local smoke_log="$scenario_dir/start-portable-smoke.log"

  if import_with_target "$conf" portable-linux "$portable_yaml" "$import_log"; then
    record_summary "$scenario" "import-portable" "PASS" "$portable_yaml"
  else
    record_summary "$scenario" "import-portable" "FAIL" "$import_log"
    return 1
  fi

  if import_dry_run_target "$conf" portable-linux "$import_dry_log"; then
    record_summary "$scenario" "import-portable-dry-run" "PASS" "$import_dry_log"
  else
    record_summary "$scenario" "import-portable-dry-run" "FAIL" "$import_dry_log"
    return 1
  fi

  if start_dry_run "$portable_yaml" "$start_log"; then
    record_summary "$scenario" "start-portable-dry-run" "PASS" "$start_log"
  else
    record_summary "$scenario" "start-portable-dry-run" "FAIL" "$start_log"
    return 1
  fi

  if smoke_boot_check "$portable_yaml" "$smoke_log"; then
    record_summary "$scenario" "start-portable-smoke" "PASS" "$smoke_log"
  else
    record_summary "$scenario" "start-portable-smoke" "FAIL" "$smoke_log"
    return 1
  fi

  return 0
}

run_parity_guard() {
  local scenario_dir="$OUTPUT_DIR/parity-guard"
  mkdir -p "$scenario_dir"
  local parity_log="$scenario_dir/import-parity-dry-run.log"

  if import_dry_run_target "$PARITY_CONF" proxmox-parity "$parity_log"; then
    record_summary "parity-guard" "import-parity-dry-run" "PASS" "$parity_log"
    return 0
  fi

  record_summary "parity-guard" "import-parity-dry-run" "FAIL" "$parity_log"
  return 1
}

run_optional_checks() {
  local scenario_dir="$OUTPUT_DIR/optional"
  mkdir -p "$scenario_dir"

  if [[ "$WITH_OPTIONAL" -eq 0 ]]; then
    record_summary "optional" "integration-checks" "SKIP" "enable with --with-optional"
    return 0
  fi

  local optional_rc=0
  local lg_log="$scenario_dir/looking-glass-discovery.log"
  if command -v looking-glass-client >/dev/null 2>&1; then
    printf 'looking-glass-client=%s\n' "$(command -v looking-glass-client)" > "$lg_log"
    record_summary "optional" "looking-glass-discovery" "PASS" "$lg_log"
  else
    echo "looking-glass-client missing on host" > "$lg_log"
    record_summary "optional" "looking-glass-discovery" "WARN" "$lg_log"
  fi

  local vfio_log="$scenario_dir/gpu-passthrough-host-check.log"
  if [[ -d /dev/vfio ]]; then
    ls -la /dev/vfio > "$vfio_log" 2>&1 || true
    record_summary "optional" "gpu-host-capability" "PASS" "$vfio_log"
  else
    echo "/dev/vfio not present" > "$vfio_log"
    record_summary "optional" "gpu-host-capability" "WARN" "$vfio_log"
  fi

  return "$optional_rc"
}

main() {
  : > "$SUMMARY_FILE"
  capture_host_environment

  local failures=0

  if ! run_required_scenario "linux" "$LINUX_CONF"; then
    failures=$((failures + 1))
  fi

  if ! run_required_scenario "windows" "$WINDOWS_CONF"; then
    failures=$((failures + 1))
  fi

  if ! run_required_scenario "mixed" "$MIXED_CONF"; then
    failures=$((failures + 1))
  fi

  if ! run_parity_guard; then
    failures=$((failures + 1))
  fi

  run_optional_checks || true

  {
    echo
    echo "Harness output: $OUTPUT_DIR"
    echo "Summary file: $SUMMARY_FILE"
    echo "Environment file: $ENV_FILE"
    echo "Required failures: $failures"
  } | tee -a "$SUMMARY_FILE"

  if [[ "$failures" -gt 0 ]]; then
    echo "Phase 3 harness finished with required failures: $failures" >&2
    exit 1
  fi

  echo "Phase 3 harness completed successfully."
}

main
