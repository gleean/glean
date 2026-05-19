#!/usr/bin/env bash
# Copy glean CLI to target/release/glean-<triple> for Tauri externalBin bundling.
# Usage: CARGO_TARGET=aarch64-apple-darwin PROFILE=release ./scripts/prepare_sidecar_bin.sh

set -euo pipefail

PROFILE="${PROFILE:-release}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

if [[ -n "${CARGO_TARGET:-}" ]]; then
	TARGET_TRIPLE="$CARGO_TARGET"
	SRC="target/${TARGET_TRIPLE}/${PROFILE}/glean"
	ALT="target/${TARGET_TRIPLE}/debug/glean"
else
	TARGET_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
	SRC="target/${PROFILE}/glean"
	ALT="target/debug/glean"
fi

DST_DIR="target/release"
mkdir -p "$DST_DIR"

if [[ "$TARGET_TRIPLE" == *"windows"* ]]; then
	DST="${DST_DIR}/glean-${TARGET_TRIPLE}.exe"
else
	DST="${DST_DIR}/glean-${TARGET_TRIPLE}"
fi

if [[ ! -f "$SRC" && ! -f "${SRC}.exe" ]]; then
	SRC="$ALT"
fi

if [[ "$TARGET_TRIPLE" == *"windows"* ]]; then
	SRC="${SRC}.exe"
fi

if [[ ! -f "$SRC" ]]; then
	echo "error: sidecar source binary not found (tried release and debug)" >&2
	exit 1
fi

cp -f "$SRC" "$DST"
chmod +x "$DST"
echo "Prepared Tauri externalBin: $DST"
