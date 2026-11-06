#!/usr/bin/env bash
# Prints the publishable workspace crates whose current version is not yet on crates.io, one per line.
# Used by release.yml to allow resuming on error.
set -euo pipefail

index=$(mktemp)
trap 'rm -f "$index"' EXIT

packages=$(cargo metadata --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.publish != []) | "\(.name) \(.version)"' \
  | sort)

if [ $# -gt 0 ]; then
  for requested in "$@"; do
    if ! [[ "$requested" =~ ^[a-zA-Z0-9_-]+$ ]]; then
      echo "::error::not a crate name: $requested" >&2
      exit 1
    fi
    if ! echo "$packages" | grep -q "^$requested "; then
      echo "::error::$requested is not a publishable crate of this workspace" >&2
      exit 1
    fi
  done
  packages=$(echo "$packages" | grep -E "^($(IFS='|'; echo "$*")) ")
fi

echo "$packages" \
  | while read -r name version; do
      lower=$(echo "$name" | tr "[:upper:]" "[:lower:]")
      path="${lower:0:2}/${lower:2:2}/$lower"
      status=$(curl --silent --output "$index" --write-out '%{http_code}' "https://index.crates.io/$path")
      case "$status" in
        200) grep -q "\"vers\":\"$version\"" "$index" || echo "$name" ;;
        *) echo "::error::unexpected status $status from the crates.io index for $name" >&2; exit 1 ;;
      esac
    done
