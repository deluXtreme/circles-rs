#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="dry-run"
START_AT="circles-types"

CRATES=(
  "circles-types"
  "circles-utils"
  "circles-rpc"
  "circles-pathfinder"
  "circles-transfers"
  "circles-sdk"
)

usage() {
  cat <<'EOF'
Usage: scripts/publish-crates.sh [--dry-run|--publish] [--start-at <crate>]

Modes:
  --dry-run   Print the publish order and versions (default)
  --publish   Run `cargo publish` for each crate in publish order

Options:
  --start-at <crate>  Resume from the given crate name
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      MODE="dry-run"
      shift
      ;;
    --publish)
      MODE="publish"
      shift
      ;;
    --start-at)
      START_AT="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

crate_manifest() {
  case "$1" in
    circles-types) echo "$ROOT_DIR/crates/types/Cargo.toml" ;;
    circles-utils) echo "$ROOT_DIR/crates/utils/Cargo.toml" ;;
    circles-rpc) echo "$ROOT_DIR/crates/rpc/Cargo.toml" ;;
    circles-pathfinder) echo "$ROOT_DIR/crates/pathfinder/Cargo.toml" ;;
    circles-transfers) echo "$ROOT_DIR/crates/transfers/Cargo.toml" ;;
    circles-sdk) echo "$ROOT_DIR/crates/sdk/Cargo.toml" ;;
    *)
      echo "unknown crate: $1" >&2
      exit 1
      ;;
  esac
}

crate_version() {
  awk -F '"' '/^version = / { print $2; exit }' "$(crate_manifest "$1")"
}

wait_for_crates_io() {
  local crate="$1"
  local version="$2"
  local attempt

  for attempt in $(seq 1 30); do
    if curl -fsSL "https://crates.io/api/v1/crates/${crate}" | grep -q "\"num\":\"${version}\""; then
      echo "crates.io index now exposes ${crate} ${version}"
      return 0
    fi
    echo "waiting for crates.io to expose ${crate} ${version} (attempt ${attempt}/30)"
    sleep 10
  done

  echo "timed out waiting for ${crate} ${version} to appear on crates.io" >&2
  return 1
}

should_start=0
for crate in "${CRATES[@]}"; do
  if [[ "$crate" == "$START_AT" ]]; then
    should_start=1
  fi

  if [[ $should_start -eq 0 ]]; then
    continue
  fi

  version="$(crate_version "$crate")"
  echo "==> ${MODE}: ${crate} ${version}"

  if [[ "$MODE" == "dry-run" ]]; then
    continue
  else
    cargo publish --locked -p "$crate" --manifest-path "$ROOT_DIR/Cargo.toml"
    wait_for_crates_io "$crate" "$version"
  fi
done
