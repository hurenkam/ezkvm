#!/bin/bash
# Script: compare-proxmox-qemu.sh
# Purpose: Diff QEMU command lines from Proxmox reference vs ezkvm generated
# Usage: compare-proxmox-qemu.sh <proxmox_cmd_file> <ezkvm_cmd_file>

set -euo pipefail

if [[ $# -ne 2 ]]; then
    echo "Usage: $0 <proxmox_reference.cmd> <ezkvm_generated.cmd>"
    echo ""
    echo "Example:"
    echo "  $0 proxmox_reference.cmd ezkvm_output.cmd"
    echo ""
    echo "This script normalizes both commands for comparison:"
    echo "  - Removes PID file paths (runtime-specific)"
    echo "  - Removes absolute paths to host-specific resources"
    echo "  - Sorts arguments for deterministic comparison"
    echo "  - Shows minimal diff output"
    exit 1
fi

proxmox_file="${1}"
ezkvm_file="${2}"

if [[ ! -f "${proxmox_file}" ]]; then
    echo "Error: Proxmox reference file not found: ${proxmox_file}"
    exit 1
fi

if [[ ! -f "${ezkvm_file}" ]]; then
    echo "Error: ezkvm command file not found: ${ezkvm_file}"
    exit 1
fi

# Function to normalize a QEMU command for comparison
normalize_command() {
    local cmd_file="$1"
    
    cat "${cmd_file}" \
        | tr ' ' '\n' \
        | grep -v '^\s*$' \
        | sed 's|/var/run/qemu-server/[0-9]*\.pid|<PID_FILE>|g' \
        | sed 's|/var/run/qemu-server/[0-9]*.swtpm|<SWTPM_SOCKET>|g' \
        | sed 's|/var/run/qemu-server/[0-9]*.qga|<QGA_SOCKET>|g' \
        | sed 's|/tmp/.*qemu|<QEMU_TMP>|g' \
        | sed 's|/dev/shm/.*|<SHM_PATH>|g' \
        | sort
}

echo "=== Normalizing commands for comparison ==="
echo ""

# Create temp files for normalized output
proxmox_norm=$(mktemp)
ezkvm_norm=$(mktemp)

trap "rm -f ${proxmox_norm} ${ezkvm_norm}" EXIT

echo "Proxmox reference:"
normalize_command "${proxmox_file}" > "${proxmox_norm}"
echo "  $(wc -l < "${proxmox_norm}") arguments"

echo "ezkvm generated:"
normalize_command "${ezkvm_file}" > "${ezkvm_norm}"
echo "  $(wc -l < "${ezkvm_norm}") arguments"

echo ""
echo "=== Diff (Proxmox < | ezkvm >) ==="
diff -u "${proxmox_norm}" "${ezkvm_norm}" || true

echo ""
echo "=== Summary ==="
if diff -q "${proxmox_norm}" "${ezkvm_norm}" > /dev/null; then
    echo "✓ Commands are identical (modulo runtime paths)"
    exit 0
else
    echo "✗ Commands differ. Review diffs above for:"
    echo "  - Device topology changes (bus, addr placements)"
    echo "  - Serial controller model changes"
    echo "  - Unexpected argument removals or additions"
    exit 1
fi
