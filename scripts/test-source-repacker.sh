#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
work_root="$(mktemp -d)"
trap 'rm -rf -- "$work_root"' EXIT

for variant in one two; do
  source_root="$work_root/$variant/source/project"
  mkdir -p "$source_root/.git"
  printf 'stable source\n' > "$source_root/main.c"
  printf 'checkout-specific %s\n' "$variant" > "$source_root/.git/index"
  if [[ "$variant" == "one" ]]; then
    archive_mtime='UTC 2024-01-01'
  else
    archive_mtime='UTC 2025-01-01'
  fi
  tar --format=gnu --sort=name --mtime="$archive_mtime" \
    -cJf "$work_root/$variant.tar.xz" -C "$work_root/$variant/source" .
  "$SCRIPT_DIR/repack-source-archive.sh" \
    "$work_root/$variant.tar.xz" \
    "$work_root/$variant-normalized.tar.xz"
done

cmp "$work_root/one-normalized.tar.xz" "$work_root/two-normalized.tar.xz"
tar -tJf "$work_root/one-normalized.tar.xz" > "$work_root/normalized-entries.txt"
grep -Fxq './project/main.c' "$work_root/normalized-entries.txt"
if grep -Eq '(^|/)\.git(/|$)' "$work_root/normalized-entries.txt"; then
  echo 'Normalized dependency source archive retained VCS administration data.' >&2
  exit 1
fi

echo 'Dependency source archive normalization is reproducible.'
