#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cd "$SCRIPT_DIR/.."

usage() {
    cat <<EOF
Usage: $0 [--yes] [--skip-macros] [--skip-stdlib]

  --yes, -y      Skip confirmation prompts. Non-interactive.
  --skip-macros  Do not publish freenet-macros.
  --skip-stdlib  Do not publish freenet-stdlib.
EOF
}

ASSUME_YES=0
SKIP_MACROS=0
SKIP_STDLIB=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --yes|-y) ASSUME_YES=1; shift;;
        --skip-macros) SKIP_MACROS=1; shift;;
        --skip-stdlib) SKIP_STDLIB=1; shift;;
        -h|--help) usage; exit 0;;
        *) echo "Unknown arg: $1"; usage; exit 2;;
    esac
done

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

if [[ $SKIP_MACROS -eq 0 ]]; then
    cargo publish --dry-run -p freenet-macros
    if confirm "Publish freenet-macros?"; then
        cargo publish -p freenet-macros
    else
        echo "Not publishing freenet-macros"
    fi
fi

if [[ $SKIP_STDLIB -eq 0 ]]; then
    cargo publish --dry-run -p freenet-stdlib
    if confirm "Publish freenet-stdlib?"; then
        cargo publish -p freenet-stdlib
    else
        echo "Not publishing freenet-stdlib"
    fi
fi
