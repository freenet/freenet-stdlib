#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
TS_DIR="$SCRIPT_DIR/../typescript"

usage() {
    cat <<EOF
Usage: $0 [--dry-run] [--yes] [--otp <code>] [--skip-push] [--skip-tests]

  --dry-run     Build and pack preview only. No publish, no tag.
  --yes         Skip confirmation prompts. Non-interactive.
  --otp <code>  Pass npm 2FA one-time password to 'npm publish'.
  --skip-push   Create tag locally, do not push to origin.
  --skip-tests  Skip 'npm test' (use only if tests just passed).
EOF
}

DRY_RUN=0
ASSUME_YES=0
OTP=""
SKIP_PUSH=0
SKIP_TESTS=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run) DRY_RUN=1; shift;;
        --yes|-y) ASSUME_YES=1; shift;;
        --otp) OTP="${2:-}"; shift 2;;
        --skip-push) SKIP_PUSH=1; shift;;
        --skip-tests) SKIP_TESTS=1; shift;;
        -h|--help) usage; exit 0;;
        *) echo "Unknown arg: $1"; usage; exit 2;;
    esac
done

cd "$TS_DIR"

PKG_NAME=$(node -p "require('./package.json').name")
PKG_VERSION=$(node -p "require('./package.json').version")
TAG="typescript-v${PKG_VERSION}"

[[ $DRY_RUN -eq 1 ]] && echo "== DRY RUN =="
echo "Package: ${PKG_NAME}@${PKG_VERSION}"
echo "Tag: ${TAG}"

if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "Working tree dirty. Commit or stash first."
    [[ $DRY_RUN -eq 1 ]] || exit 1
fi

if git rev-parse -q --verify "refs/tags/${TAG}" > /dev/null; then
    echo "Tag ${TAG} already exists."
    [[ $DRY_RUN -eq 1 ]] || exit 1
fi

if npm view "${PKG_NAME}@${PKG_VERSION}" version > /dev/null 2>&1; then
    echo "${PKG_NAME}@${PKG_VERSION} already published on npm."
    [[ $DRY_RUN -eq 1 ]] || exit 1
fi

echo "Installing deps..."
npm install

if [[ $SKIP_TESTS -eq 0 ]]; then
    echo "Running tests..."
    npm test
else
    echo "Skipping tests (--skip-tests)."
fi

echo "Building..."
npm run build

echo "Pack preview:"
npm pack --dry-run

if [[ $DRY_RUN -eq 1 ]]; then
    echo "Dry run complete. No publish, no tag."
    exit 0
fi

confirm() {
    local msg="$1"
    if [[ $ASSUME_YES -eq 1 ]]; then
        echo "${msg} [auto-yes]"
        return 0
    fi
    read -p "${msg} " -n 1 -r
    echo
    [[ $REPLY =~ ^[Yy]$ ]]
}

if ! confirm "Publish ${PKG_NAME}@${PKG_VERSION} to npm?"; then
    echo "Aborted. No publish, no tag."
    exit 0
fi

PUBLISH_ARGS=()
if [[ -n "$OTP" ]]; then
    PUBLISH_ARGS+=("--otp=$OTP")
fi

npm publish "${PUBLISH_ARGS[@]}"

git tag -a "${TAG}" -m "Release ${PKG_NAME}@${PKG_VERSION}"
echo "Tag ${TAG} created locally."

if [[ $SKIP_PUSH -eq 1 ]]; then
    echo "Skip push (--skip-push). Push manually: git push origin ${TAG}"
    exit 0
fi

if confirm "Push tag ${TAG} to origin?"; then
    git push origin "${TAG}"
else
    echo "Tag not pushed. Push manually: git push origin ${TAG}"
fi
