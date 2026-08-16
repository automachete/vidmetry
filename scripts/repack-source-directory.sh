#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

if [[ $# -ne 4 ]]; then
  echo "Usage: $0 <input-directory> <output-directory> <archive-list> <jobs>" >&2
  exit 2
fi

input_directory="$1"
output_directory="$2"
archive_list="$3"
job_limit="$4"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

[[ -d "$input_directory" ]]
[[ -d "$output_directory" ]]
[[ -f "$archive_list" ]]
[[ "$job_limit" =~ ^[1-8]$ ]]

mapfile -t archive_names < "$archive_list"
if [[ "${#archive_names[@]}" -lt 20 ]]; then
  echo "Too few dependency source archives were selected: ${#archive_names[@]}" >&2
  exit 1
fi
for archive_name in "${archive_names[@]}"; do
  [[ "$archive_name" =~ ^[A-Za-z0-9._-]+\.tar\.xz$ ]] || {
    echo "Invalid dependency archive name: $archive_name" >&2
    exit 1
  }
done
if [[ "$(printf '%s\n' "${archive_names[@]}" | sort -u | wc -l)" -ne "${#archive_names[@]}" ]]; then
  echo "Dependency archive names must be unique." >&2
  exit 1
fi

active_jobs=0
repack_failed=0
for archive_name in "${archive_names[@]}"; do
  "$SCRIPT_DIR/repack-source-archive.sh" \
    "$input_directory/$archive_name" \
    "$output_directory/$archive_name" &
  active_jobs=$((active_jobs + 1))

  if [[ "$active_jobs" -eq "$job_limit" ]]; then
    if ! wait -n; then
      repack_failed=1
    fi
    active_jobs=$((active_jobs - 1))
  fi
done
while [[ "$active_jobs" -gt 0 ]]; do
  if ! wait -n; then
    repack_failed=1
  fi
  active_jobs=$((active_jobs - 1))
done
[[ "$repack_failed" -eq 0 ]]
