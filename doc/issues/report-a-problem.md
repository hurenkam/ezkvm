# Report a Problem

Use this guide for:
- runtime failures
- import/parity regressions
- packaging/install issues
- documentation defects
- test or validation failures that indicate broken behavior

## What to Include

1. Short problem summary.
2. Expected behavior.
3. Actual behavior.
4. Reproduction steps.
5. Host environment details:
   - distro and version
   - relevant package/runtime versions
   - whether the issue is portable runtime, Proxmox import, qemu-cmd import, packaging, or docs related
6. Evidence:
   - relevant config or command line snippet
   - error output or log excerpt
   - failing test/check command if applicable

## Minimum Reproduction Checklist

- identify the exact input config or fixture
- provide the command used
- state whether the issue is deterministic or intermittent
- note the first known good / first known bad version when available

## Routing

- User-facing defects should map to a backlog ticket when confirmed.
- Docs-only defects may still be tracked in backlog when they affect discoverability or migration work.
- Cross-distro packaging problems should link to packaging epic `K` tickets when relevant.