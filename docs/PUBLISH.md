# Publishing Driftlock

## Preconditions

- `cargo test --workspace` and `./scripts/harden.sh` pass on `main`.
- Version bumped in `metadata/root.project.json` and crate `Cargo.toml` files.
- `CHANGELOG.md` entry for the release.
- `CARGO_REGISTRY_TOKEN` available via `tsafe exec` (see CratesIoTsafe skill).

## crates.io (ordered)

Publish in dependency order (automated on tag in `release.yml` when `CARGO_REGISTRY_TOKEN` is set):

1. `driftlock-contracts`
2. `driftlock-git`
3. `driftlock-skills`
4. `driftlock-core`
5. `driftlock-store`
6. `driftlock-cli`
7. `driftlock-mcp`

```bash
cd /path/to/driftlock
for pkg in driftlock-contracts driftlock-git driftlock-skills driftlock-core driftlock-store driftlock-cli driftlock-mcp; do
  cargo publish -p "$pkg" --dry-run
done
tsafe exec -- bash -c 'for pkg in driftlock-contracts driftlock-git driftlock-skills driftlock-core driftlock-store driftlock-cli driftlock-mcp; do cargo publish -p "$pkg" --locked; done'
```

PRs run `publish-dry-run.yml`. The `driftlock-mcp` binary is also attached to
GitHub release artifacts; publishing it to crates.io additionally enables
`cargo install driftlock-mcp`.

## GitHub release

1. Tag `vX.Y.Z` on `main`.
2. `release.yml` builds linux + macos binaries.
3. Attach `driftlock` and `driftlock-mcp` to the release.

## Post-release

- Update sibling `PARITY_BACKLOG` / ecosystem notes if contract seams changed.
- Run `./scripts/release_smoke.sh` against the tagged artifact when available.
