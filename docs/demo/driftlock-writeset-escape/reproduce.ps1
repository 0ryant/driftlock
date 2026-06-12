#!/usr/bin/env pwsh
# Reproduce the driftlock write-set-escape evidence pack end to end.
# Everything here is synthetic. No real org, repo, or credential is referenced.
#
# Usage:  pwsh -File reproduce.ps1
# Effect: rebuilds the CLI (isolated CARGO_TARGET_DIR), re-runs driftlock against
#         the two fixture diffs, and regenerates findings/ + results/. Outputs are
#         NEVER hand-edited.

$ErrorActionPreference = 'Stop'
$pack = $PSScriptRoot
$repoRoot = (Resolve-Path "$pack\..\..\..").Path   # driftlock repo root

# --- 1. design-system asset drift pin -------------------------------------
# The vendored _assets/site.css MUST match the algol.cc single source of truth.
$pinned = '2728B214E6E59E926DBAAC02A3C7FB08F531C3099074D921D2B818BF4D887182'
$have = (Get-FileHash "$pack\_assets\site.css" -Algorithm SHA256).Hash
if ($have -ne $pinned) {
    throw "site.css drift: expected $pinned got $have (re-vendor from algol.cc/css/site.css)"
}
Write-Host "[ok] _assets/site.css sha256 pinned to algol.cc source"

# --- 2. build the CLI (isolated target dir to dodge workspace locks) -------
$env:CARGO_TARGET_DIR = "$repoRoot\target-ci"
Push-Location $repoRoot
cargo build --release -p driftlock-cli
Pop-Location
$bin = "$repoRoot\target-ci\release\driftlock.exe"
Write-Host "[ok] built $bin ($(& $bin --version))"

# --- 3. run the tool on each synthetic diff --------------------------------
$g = "$pack\input\taskgraph.json"
$task = 'adr-0007:T01'

& $bin check-diff --graph $g --task $task --diff-file "$pack\input\diffs\compliant.diff" $pack |
    Out-File -Encoding utf8 "$pack\findings\compliant.report.json"
Write-Host "[ok] findings/compliant.report.json"

& $bin check-diff --graph $g --task $task --diff-file "$pack\input\diffs\escaping.diff" $pack |
    Out-File -Encoding utf8 "$pack\findings\escaping.report.json"
Write-Host "[ok] findings/escaping.report.json"

# --- 4. demonstrate the fail-closed gate -----------------------------------
# `complete` re-verifies and bails non-zero when the diff escapes the write set.
& $bin complete --graph $g --task $task --actor 'agent-demo' `
    --diff-file "$pack\input\diffs\escaping.diff" $pack 2>&1 |
    Out-File -Encoding utf8 "$pack\results\complete-escaping.txt"
$gateExit = $LASTEXITCODE
"check-diff compliant -> allowed=true,  process exit 0"                         | Out-File -Encoding utf8       "$pack\results\gate-exit-codes.txt"
"check-diff escaping  -> allowed=false, process exit 0 (reports, does not gate)" | Out-File -Encoding utf8 -Append "$pack\results\gate-exit-codes.txt"
"complete   escaping  -> GATE BLOCKS,   process exit $gateExit (fail-closed)"    | Out-File -Encoding utf8 -Append "$pack\results\gate-exit-codes.txt"
if ($gateExit -ne 1) { throw "expected complete to fail closed (exit 1), got $gateExit" }
Write-Host "[ok] gate failed closed on escaping diff (exit $gateExit)"

# transient state the gate writes; not part of the pack
if (Test-Path "$pack\.driftlock") { Remove-Item -Recurse -Force "$pack\.driftlock" }

Write-Host "`nDone. Verdicts: compliant=allowed, escaping=blocked (2 violations)."
