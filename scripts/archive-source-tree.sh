#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

if [[ $# -ne 3 ]]; then
  echo "Usage: $0 <input-directory> <source-root-name> <output.tar.xz>" >&2
  exit 2
fi

input_directory="$1"
source_root_name="$2"
output="$3"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

[[ -d "$input_directory/$source_root_name" ]]
[[ "$source_root_name" =~ ^vidmetry-ffmpeg-[A-Za-z0-9.-]+-corresponding-source$ ]]
[[ "$output" == *.tar.xz ]]
[[ ! -e "$output" ]]

"$SCRIPT_DIR/sanitize-source-tree.sh" \
  "$input_directory/$source_root_name" reject

(
  cd "$input_directory/$source_root_name"
  find . -type f ! -path ./SOURCE_SHA256SUMS -print0 | sort -z | xargs -0 sha256sum
) > "$input_directory/$source_root_name/SOURCE_SHA256SUMS"

temporary_output="$output.tmp"
trap 'rm -f -- "$temporary_output"' EXIT
tar --format=gnu --sort=name --mtime='UTC 1970-01-01' \
  --owner=0 --group=0 --numeric-owner --hard-dereference \
  -cJf "$temporary_output" -C "$input_directory" "$source_root_name"
mv -- "$temporary_output" "$output"
