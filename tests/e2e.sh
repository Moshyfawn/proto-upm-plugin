#!/usr/bin/env bash
# Real proto CLI in a throwaway PROTO_HOME; run `cargo wasm` first.
set -euo pipefail

repo="$(cd "$(dirname "$0")/.." && pwd)"
wasm="$repo/target/wasm32-wasip1/release/upm_tool.wasm"
[[ -f "$wasm" ]] || { echo "missing $wasm, run cargo wasm first" >&2; exit 1; }

# Native proto needs Windows paths; bash needs POSIX ones on PATH.
if [[ "${OS:-}" == "Windows_NT" ]]; then
  export PROTO_HOME="$(cygpath -m "${RUNNER_TEMP:-$TEMP}")/proto-upm-e2e.$$"
  export PATH="$(cygpath -u "$PROTO_HOME")/shims:$(cygpath -u "$PROTO_HOME")/bin:$PATH"
  locator="file:///$(cygpath -m "$wasm")"
else
  export PROTO_HOME="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/proto-upm-e2e.$$"
  export PATH="$PROTO_HOME/shims:$PROTO_HOME/bin:$PATH"
  locator="file://$wasm"
fi
export PROTO_LOG="${PROTO_LOG:-error}"
workdir="$PROTO_HOME/project"
mkdir -p "$workdir"
trap 'rm -rf "$PROTO_HOME"' EXIT

# The repo's .prototools pins node; everything after runs outside it so only the added plugin is in play.
(cd "$repo" && proto install node --pin global)
cd "$workdir"

proto plugin add upm "$locator" --to global
proto install upm --pin global

version="$(upm --version)"
echo "upm --version: $version"
[[ -n "$version" ]] || { echo "upm --version printed nothing" >&2; exit 1; }
