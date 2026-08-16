#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

if [[ $# -ne 2 ]]; then
  echo "Usage: $0 <source-root> <reject|remove-and-record>" >&2
  exit 2
fi

source_root="$1"
external_link_policy="$2"
[[ -d "$source_root" ]]
[[ "$external_link_policy" == 'reject' || "$external_link_policy" == 'remove-and-record' ]]

for tool in dirname find readlink realpath sort; do
  command -v "$tool" >/dev/null
done

resolved_root="$(cd -P -- "$source_root" && pwd)"
normalization_directory="$resolved_root/.vidmetry-normalization"
if [[ "$external_link_policy" == 'remove-and-record' && \
      -e "$normalization_directory" ]]; then
  echo 'Refusing reserved upstream .vidmetry-normalization data.' >&2
  exit 1
fi

special_entry=''
if IFS= read -r -d '' special_entry < <(
  find "$resolved_root" -xdev \
    ! -type d ! -type f ! -type l -print0
); then
  printf 'Refusing a source tree with a special filesystem entry: %q\n' \
    "${special_entry#"$resolved_root"/}" >&2
  exit 1
fi

hard_link=''
if IFS= read -r -d '' hard_link < <(
  find "$resolved_root" -xdev -type f -links +1 -print0
); then
  printf 'Refusing a source tree with a multiply-linked file: %q\n' \
    "${hard_link#"$resolved_root"/}" >&2
  exit 1
fi

declare -a external_links=()
declare -a external_targets=()
while IFS= read -r -d '' link_path; do
  link_target="$(readlink -- "$link_path")"
  if [[ "$link_target" == /* || "$link_target" == *\\* || \
        "$link_target" =~ ^[A-Za-z]: ]]; then
    resolved_target=''
  else
    target_path="$(dirname -- "$link_path")/$link_target"
    if ! resolved_target="$(realpath -m -- "$target_path")"; then
      printf 'Refusing an unresolvable symbolic link: %q\n' \
        "${link_path#"$resolved_root"/}" >&2
      exit 1
    fi
  fi
  case "$resolved_target" in
    "$resolved_root" | "$resolved_root"/*) ;;
    *)
      external_links+=("$link_path")
      external_targets+=("$link_target")
      ;;
  esac
done < <(find "$resolved_root" -xdev -type l -print0)

if [[ "${#external_links[@]}" -eq 0 ]]; then
  exit 0
fi

if [[ "$external_link_policy" == 'reject' ]]; then
  printf 'Refusing a symbolic link outside the source root: %q -> %q\n' \
    "${external_links[0]#"$resolved_root"/}" "${external_targets[0]}" >&2
  exit 1
fi

report_entries="$(mktemp)"
trap 'rm -f -- "$report_entries"' EXIT
for index in "${!external_links[@]}"; do
  relative_path="${external_links[$index]#"$resolved_root"/}"
  printf '%q\t%q\n' "$relative_path" "${external_targets[$index]}" \
    >> "$report_entries"
  rm -- "${external_links[$index]}"
done

mkdir -- "$normalization_directory"
{
  printf 'path\toriginal-target\n'
  sort -- "$report_entries"
} > "$normalization_directory/removed-external-symlinks.tsv"
