---
name: release-typescript
description: Release the @freenetorg/freenet-stdlib npm package. Use when the user says "release the typescript package", "publish stdlib to npm", "cut a typescript release", "bump and publish ts", "release ts sdk", or similar. Runs scripts/release-typescript-ver.sh end-to-end including version bump, build, publish, tag, and push.
---

# Release @freenetorg/freenet-stdlib

Release the TypeScript SDK to npm. Package: `@freenetorg/freenet-stdlib`. Registry: npmjs.org. Publish requires 2FA OTP.

## Inputs to gather before starting

Ask the user (only what is not already clear from the request):

1. **Version bump**: major / minor / patch / explicit (e.g. `0.3.0`). Default suggestion: minor if API additions, patch if fixes only. Show current version from `typescript/package.json` first.
2. **OTP**: 6-digit code from authenticator. Required for `npm publish`. Ask right before publish, not at start (codes expire in ~30s).
3. **Push tag after publish?** Default yes.

## Pre-flight checks

Run before touching anything:

- `git status` — working tree must be clean.
- `git rev-parse --abbrev-ref HEAD` — confirm on `main` (or ask user if branch is intentional).
- `npm whoami` — confirm logged in. If not, ask user to run `npm login` in their terminal (interactive, they must do it).
- `npm view @freenetorg/freenet-stdlib@<new-version> version` — must fail (version must not already exist).
- `git tag -l "typescript-v<new-version>"` — must be empty.

If any check fails, stop and surface the problem.

## Bump + commit

1. Edit `typescript/package.json` — update `version` field only.
2. Dry-run the release script to verify the tarball contents:
   ```
   bash scripts/release-typescript-ver.sh --dry-run
   ```
   Expected: ~240+ files, all under `dist/src/`, no `dist/tests/` leakage. Size ~70-80 kB packed. If tests appear in tarball, stop — `files` globs in package.json broken.
3. Commit:
   ```
   git add typescript/package.json
   git commit -m "chore(typescript): bump to <new-version>"
   ```

## Publish

Two auth paths. Prefer token (non-interactive, no timer):

**Token (preferred):**
```
bash scripts/release-typescript-ver.sh --yes --token <npm_token>
```
Or set `NPM_TOKEN` env var and omit the flag. Token must have publish scope for the package and "bypass 2FA" enabled if org enforces 2FA.

**OTP (interactive auth):**
```
bash scripts/release-typescript-ver.sh --yes --otp <6-digit-code>
```
Ask user for OTP right before running — codes expire in ~30s.

Flags:
- `--yes` — skip confirmation prompts (required under Bash tool)
- `--token <tok>` — npm access token; auth via temp userconfig (not written to `$HOME/.npmrc`)
- `--otp <code>` — 2FA code passed to `npm publish --otp`

Never commit tokens. Never write token to `.npmrc` in repo. Script uses `mktemp` + `trap` to clean temp userconfig.

On 403 E2FA/EOTP: OTP expired or token lacks 2FA bypass. Retry with fresh value.

On success: script publishes, creates tag `typescript-v<version>`, prompts for push. With `--yes` it auto-pushes. If user wanted to skip push, add `--skip-push` and push manually later.

## Post-publish verification

- `npm view @freenetorg/freenet-stdlib version` — should show new version.
- `git log -1 --format=%H` + `git tag --points-at HEAD` — tag should point at bump commit.
- `git push origin main` — push the bump commit itself (tag push is separate from branch push).

## Script reference

`scripts/release-typescript-ver.sh` flags:
- `--dry-run` — build + pack preview only
- `--yes` / `-y` — non-interactive (required when invoked via Bash tool)
- `--otp <code>` — npm 2FA code
- `--token <tok>` — npm access token (also reads `NPM_TOKEN` env var)
- `--skip-push` — local tag only
- `--skip-tests` — skip `npm test` (use when tests just ran)

## Common failures

- **Dirty working tree**: commit or stash first.
- **Tag exists**: version already released. Bump again.
- **npm 403 E2FA**: OTP missing/expired. Pass fresh `--otp`.
- **npm 403 EOTP**: same, fresh OTP.
- **Test failures in CI but not locally**: `npm test` uses `mock-socket` + `ws`; check `tests/websocket-interface.test.ts` if WebSocket flakes.
- **Tarball has `dist/tests/`**: `files` globs in `package.json` too broad; must be `dist/src/**` not `dist/**`.

## Do not

- Never amend the bump commit after tagging — tag would point at orphaned SHA.
- Never `npm unpublish` — blocked by npm after 72h and bad practice. If a bad version ships, publish a patch.
- Never run with `--yes` without user confirmation on version + OTP.
- Never pipe OTP into stdin of `npm publish`; use `--otp` flag.
