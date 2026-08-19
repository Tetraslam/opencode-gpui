#!/usr/bin/env bash
set -euo pipefail

shopt -s globstar nullglob
failed=0

for file in src/**/*.rs; do
  lines=$(wc -l < "$file")
  if (( lines > 300 )); then
    printf '%s has %d lines; production source files are limited to 300\n' "$file" "$lines" >&2
    failed=1
  fi
done

exit "$failed"
