# Security Policy

Driftlock coordinates agents that can edit code. Treat it as a governance and safety boundary, not a harmless todo tool.

## Threat model summary

Primary risks:

- agent overreach beyond work order scope
- stale graph safety decisions
- prompt injection through ADR or repo text
- malicious lane manifests
- unsafe MCP tool invocation
- poisoned generated artifacts
- secret leakage through resources or prompts

## Controls

- Stdio server writes only MCP JSON messages to stdout.
- Tool inputs are schema-validated by the MCP client and revalidated in process where possible.
- Resources expose only packaged skills, schemas, prompts, and selected Driftlock state.
- Diff verification rejects writes outside allowed work order scope.
- Hard conflicts block readiness.
- Unknown or low-confidence task extraction is not ready by default.

## Reporting

Open a private security issue with:

- affected version
- reproduction steps
- impact
- suggested mitigation

Do not include secrets, private repo paths, or proprietary ADR text in public reports.
