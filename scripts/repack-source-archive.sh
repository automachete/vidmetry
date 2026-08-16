#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: $0 <input.tar.xz> <output.tar.xz>" >&2
  exit 2
fi

input="$1"
output="$2"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
[[ -f "$input" ]]
[[ "$input" == *.tar.xz ]]
[[ "$output" == *.tar.xz ]]
[[ ! -e "$output" ]]

for tool in find grep tar; do
  command -v "$tool" >/dev/null
done

work_root="$(mktemp -d)"
trap 'rm -rf -- "$work_root"' EXIT
extract_root="$work_root/source"
entry_list="$work_root/entries.txt"
mkdir -p "$extract_root" "$(dirname -- "$output")"

tar -tJf "$input" > "$entry_list"
if grep -Eq '(^/|(^|/)\.\.(/|$)|\\)' "$entry_list"; then
  echo "Refusing to normalize an archive with an unsafe member path: $input" >&2
  exit 1
fi

tar -xJf "$input" -C "$extract_root" --no-same-owner --no-same-permissions

# VCS administration data is not source and contains checkout-specific indexes,
# databases, locks, and timestamps. Removing the administration formats used by
# the supported upstream fetchers makes independently fetched source trees
# byte-reproducible while retaining the preferred source form used by the build.
while IFS= read -r -d '' vcs_entry; do
  rm -rf -- "$vcs_entry"
done < <(
  find "$extract_root" -depth \
    \( -name .git -o -name .svn -o -name .hg -o -name .bzr \
       -o -name _darcs -o -name CVS -o -name RCS -o -name SCCS \) \
    -print0
)

"$SCRIPT_DIR/sanitize-source-tree.sh" "$extract_root" remove-and-record

normalized="$work_root/normalized.tar.xz"
tar --format=gnu --sort=name --mtime='UTC 1970-01-01' \
  --owner=0 --group=0 --numeric-owner \
  --mode='u+rwX,go+rX,go-w' --hard-dereference \
  -cJf "$normalized" -C "$extract_root" .
mv -- "$normalized" "$output"
