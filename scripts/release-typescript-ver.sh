#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
TS_DIR="$SCRIPT_DIR/../typescript"
cd "$TS_DIR"

PKG_NAME=$(node -p "require('./package.json').name")
PKG_VERSION=$(node -p "require('./package.json').version")
TAG="typescript-v${PKG_VERSION}"

DRY_RUN=0
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=1
    echo "== DRY RUN =="
fi

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

echo "Running tests..."
npm test

echo "Building..."
npm run build

echo "Pack preview:"
npm pack --dry-run

if [[ $DRY_RUN -eq 1 ]]; then
    echo "Dry run complete. No publish, no tag."
    exit 0
fi

read -p "Publish ${PKG_NAME}@${PKG_VERSION} to npm? " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted. No publish, no tag."
    exit 0
fi

npm publish

git tag -a "${TAG}" -m "Release ${PKG_NAME}@${PKG_VERSION}"
echo "Tag ${TAG} created locally."

read -p "Push tag ${TAG} to origin? " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    git push origin "${TAG}"
else
    echo "Tag not pushed. Push manually: git push origin ${TAG}"
fi
