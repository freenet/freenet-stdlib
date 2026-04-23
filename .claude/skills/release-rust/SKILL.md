---
name: release-rust
description: Release the freenet-stdlib and/or freenet-macros Rust crates to crates.io. Use when the user says "release the rust crate", "publish freenet-stdlib to crates.io", "cut a rust release", "bump and publish rust", "release freenet-macros", or similar. Can run locally via scripts/release-rust-ver.sh or remotely via the Release workflow on a pushed tag.
---

# Release freenet-stdlib / freenet-macros

Two crates live in this repo:
- `freenet-stdlib` at `rust/Cargo.toml`
- `freenet-macros` at `rust-macros/Cargo.toml`

Target registry: crates.io. Auth: `CARGO_REGISTRY_TOKEN` (local) or GitHub secret `CARGO_REGISTRY_TOKEN` (CI).

## Two release paths

### Path A — CI (preferred)

1. Bump version(s) in Cargo.toml.
2. Open PR, merge to main.
3. Tag and push:
   - `rust-v<version>` → publishes `freenet-stdlib`
   - `rust-macros-v<version>` → publishes `freenet-macros`
4. `.github/workflows/release.yml` runs: verifies tag matches Cargo.toml version, dry-runs, then `cargo publish`.

If releasing both, tag macros first (stdlib depends on it) and wait for publish before tagging stdlib.

### Path B — Local

```
bash scripts/release-rust-ver.sh [--yes] [--skip-macros] [--skip-stdlib]
```

Flags:
- `--yes` / `-y` — non-interactive
- `--skip-macros` — only publish stdlib
- `--skip-stdlib` — only publish macros

Requires `cargo login` or `CARGO_REGISTRY_TOKEN` env var in shell.

## Inputs to gather

1. **Which crate(s)**: stdlib, macros, or both.
2. **Version bump**: semver. Show current version from Cargo.toml first.
3. **Path**: CI or local.

## Pre-flight

- `git status` clean
- On `main` (or deliberately on another branch)
- `cargo publish --dry-run -p <crate>` passes
- `cargo test --workspace` passes
- If stdlib depends on a new macros version, macros must be published first
- `crates.io` version does not already exist: `cargo search freenet-stdlib`

## Bump + commit

1. Edit `rust/Cargo.toml` or `rust-macros/Cargo.toml` `version`.
2. If stdlib bumps its dep on macros, update that too.
3. Commit:
   ```
   git add rust/Cargo.toml
   git commit -m "chore(rust): bump freenet-stdlib to <version>"
   ```

## Tag + push (CI path)

```
git tag -a rust-v<version> -m "Release freenet-stdlib <version>"
git push origin rust-v<version>
```

For macros: tag `rust-macros-v<version>`.

Workflow verifies tag matches Cargo.toml version. Mismatch = fail-fast.

## Post-publish verification

- `cargo search freenet-stdlib` — should list new version
- https://crates.io/crates/freenet-stdlib — should show new version
- Actions tab — release workflow green

## Common failures

- **Tag/Cargo.toml version mismatch**: workflow fails fast. Retag after fix.
- **Publish blocked by uncommitted files in workspace**: `cargo publish` requires clean state.
- **Macros dep version not published yet**: publish macros first, wait for index refresh, then stdlib.
- **`error: crate version X is already uploaded`**: version exists. Bump again.
- **Rate limit**: crates.io throttles publishes; wait and retry.

## Do not

- Never `cargo yank` casually — only for critical bugs/security.
- Never skip `--dry-run` on manual local releases.
- Never push a tag before the bump commit is on main.
- Never store `CARGO_REGISTRY_TOKEN` in the repo.
