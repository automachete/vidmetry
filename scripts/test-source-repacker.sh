#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
work_root="$(mktemp -d)"
trap 'rm -rf -- "$work_root"' EXIT

for variant in one two; do
  source_root="$work_root/$variant/source/project"
  mkdir -p "$source_root/.git"
  printf 'stable source\n' > "$source_root/main.c"
  printf '#!/usr/bin/env sh\nexit 0\n' > "$source_root/build.sh"
  ln -s main.c "$source_root/internal-link"
  printf 'checkout-specific %s\n' "$variant" > "$source_root/.git/index"
  if [[ "$variant" == "one" ]]; then
    archive_mtime='UTC 2024-01-01'
    chmod 0700 "$source_root" "$source_root/build.sh"
    chmod 0600 "$source_root/main.c"
  else
    archive_mtime='UTC 2025-01-01'
    chmod 0775 "$source_root" "$source_root/build.sh"
    chmod 0664 "$source_root/main.c"
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
tar -tvJf "$work_root/one-normalized.tar.xz" \
  | grep -Fq './project/internal-link -> main.c'
tar -tvJf "$work_root/one-normalized.tar.xz" \
  | grep -Eq '^-rw-r--r-- .* ./project/main\.c$'
tar -tvJf "$work_root/one-normalized.tar.xz" \
  | grep -Eq '^-rwxr-xr-x .* ./project/build\.sh$'
tar -tvJf "$work_root/one-normalized.tar.xz" \
  | grep -Eq '^drwxr-xr-x .* ./project/$'
if grep -Eq '(^|/)\.git(/|$)' "$work_root/normalized-entries.txt"; then
  echo 'Normalized dependency source archive retained VCS administration data.' >&2
  exit 1
fi

mkdir -p "$work_root/external/source/project"
ln -s ../../outside "$work_root/external/source/project/escape"
ln -s '..\..\outside' "$work_root/external/source/project/windows-escape"
ln -s 'C:relative' "$work_root/external/source/project/drive-relative-escape"
tar -cJf "$work_root/external.tar.xz" \
  -C "$work_root/external/source" .
"$SCRIPT_DIR/repack-source-archive.sh" \
  "$work_root/external.tar.xz" "$work_root/external-normalized.tar.xz"
if tar -tJf "$work_root/external-normalized.tar.xz" \
  | grep -Eq '(^|/)(escape|windows-escape|drive-relative-escape)$'; then
  echo 'Normalized dependency source archive retained an external link.' >&2
  exit 1
fi
tar -xJOf "$work_root/external-normalized.tar.xz" \
  './.vidmetry-normalization/removed-external-symlinks.tsv' \
  | grep -Fq $'project/escape\t../../outside'
tar -xJOf "$work_root/external-normalized.tar.xz" \
  './.vidmetry-normalization/removed-external-symlinks.tsv' \
  | grep -Fq 'project/windows-escape'
tar -xJOf "$work_root/external-normalized.tar.xz" \
  './.vidmetry-normalization/removed-external-symlinks.tsv' \
  | grep -Fq $'project/drive-relative-escape\tC:relative'

mkdir -p "$work_root/special/source/project"
mkfifo "$work_root/special/source/project/pipe"
tar -cJf "$work_root/special.tar.xz" -C "$work_root/special/source" .
if "$SCRIPT_DIR/repack-source-archive.sh" \
  "$work_root/special.tar.xz" "$work_root/special-normalized.tar.xz"; then
  echo 'Dependency source normalizer accepted a special filesystem entry.' >&2
  exit 1
fi

mkdir -p "$work_root/hard-link/source/project"
printf 'linked source\n' > "$work_root/hard-link/source/project/original"
ln "$work_root/hard-link/source/project/original" \
  "$work_root/hard-link/source/project/duplicate"
tar -cJf "$work_root/hard-link.tar.xz" -C "$work_root/hard-link/source" .
if "$SCRIPT_DIR/repack-source-archive.sh" \
  "$work_root/hard-link.tar.xz" "$work_root/hard-link-normalized.tar.xz"; then
  echo 'Dependency source normalizer accepted a multiply-linked file.' >&2
  exit 1
fi

mkdir -p "$work_root/batch-input" "$work_root/batch-output"
for archive_number in $(seq -w 1 20); do
  archive_name="dependency-$archive_number.tar.xz"
  cp -- "$work_root/one.tar.xz" "$work_root/batch-input/$archive_name"
  printf '%s\n' "$archive_name" >> "$work_root/batch-list.txt"
done
"$SCRIPT_DIR/repack-source-directory.sh" \
  "$work_root/batch-input" "$work_root/batch-output" \
  "$work_root/batch-list.txt" 4
test "$(find "$work_root/batch-output" -maxdepth 1 -type f -name '*.tar.xz' | wc -l)" -eq 20

outer_root='vidmetry-ffmpeg-test-corresponding-source'
mkdir -p "$work_root/outer/$outer_root/source"
printf 'stable outer source\n' > "$work_root/outer/$outer_root/source/main.c"
ln -s main.c "$work_root/outer/$outer_root/source/internal-link"
"$SCRIPT_DIR/archive-source-tree.sh" \
  "$work_root/outer" "$outer_root" "$work_root/outer-one.tar.xz"
chmod 0775 "$work_root/outer/$outer_root" \
  "$work_root/outer/$outer_root/source"
chmod 0664 "$work_root/outer/$outer_root/source/main.c"
"$SCRIPT_DIR/archive-source-tree.sh" \
  "$work_root/outer" "$outer_root" "$work_root/outer-two.tar.xz"
cmp "$work_root/outer-one.tar.xz" "$work_root/outer-two.tar.xz"
tar -tvJf "$work_root/outer-one.tar.xz" \
  | grep -Eq '^-rw-r--r-- .*source/main\.c$'
tar -xJOf "$work_root/outer-one.tar.xz" \
  "$outer_root/SOURCE_SHA256SUMS" | grep -Fq './source/main.c'

ln -s "$work_root/outer/$outer_root/source/main.c" \
  "$work_root/outer/$outer_root/source/absolute-link"
if "$SCRIPT_DIR/archive-source-tree.sh" \
  "$work_root/outer" "$outer_root" "$work_root/outer-absolute.tar.xz"; then
  echo 'Outer source archiver accepted an absolute symbolic link.' >&2
  exit 1
fi
rm -- "$work_root/outer/$outer_root/source/absolute-link"

ln -s ../../outside "$work_root/outer/$outer_root/source/external-link"
if "$SCRIPT_DIR/archive-source-tree.sh" \
  "$work_root/outer" "$outer_root" "$work_root/outer-unsafe.tar.xz"; then
  echo 'Outer source archiver accepted an external symbolic link.' >&2
  exit 1
fi

echo 'Dependency and outer source archive normalization is reproducible and boundary-safe.'
