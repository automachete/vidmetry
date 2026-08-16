#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
MANIFEST="$SCRIPT_DIR/ffmpeg-sidecars.json"
OUTPUT=""
CACHE_DIR=""
VALIDATE_ONLY=0

usage() {
  echo "Usage: $0 --output <archive.tar.xz> [--cache-dir <directory>] [--validate-only]" >&2
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --output)
      [[ $# -ge 2 ]] || { usage; exit 2; }
      OUTPUT="$2"
      shift 2
      ;;
    --cache-dir)
      [[ $# -ge 2 ]] || { usage; exit 2; }
      CACHE_DIR="$2"
      shift 2
      ;;
    --validate-only)
      VALIDATE_ONLY=1
      shift
      ;;
    *)
      usage
      exit 2
      ;;
  esac
done

command -v jq >/dev/null
[[ -f "$MANIFEST" ]]

schema_version="$(jq -er '.schemaVersion' "$MANIFEST")"
engine_id="$(jq -er '.engine.id' "$MANIFEST")"
license="$(jq -er '.engine.license' "$MANIFEST")"
variant="$(jq -er '.engine.variant' "$MANIFEST")"
binary_url="$(jq -er '.archive.url' "$MANIFEST")"
binary_sha256="$(jq -er '.archive.sha256' "$MANIFEST")"
archive_name="$(jq -er '.correspondingSource.archiveName' "$MANIFEST")"
archive_sha256="$(jq -er '.correspondingSource.archiveSha256' "$MANIFEST")"
asset_tag="$(jq -er '.correspondingSource.assetTag' "$MANIFEST")"
source_toolchain_image="$(jq -er '.correspondingSource.sourceToolchainImage' "$MANIFEST")"
build_repository="$(jq -er '.correspondingSource.buildRepository' "$MANIFEST")"
build_commit="$(jq -er '.correspondingSource.buildCommit' "$MANIFEST")"
ffmpeg_repository="$(jq -er '.correspondingSource.ffmpegRepository' "$MANIFEST")"
ffmpeg_commit="$(jq -er '.correspondingSource.ffmpegCommit' "$MANIFEST")"

[[ "$schema_version" == "2" ]]
[[ "$license" == "GPL-3.0-or-later" ]]
[[ "$variant" == "win64-gpl" ]]
[[ "$binary_url" == https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-* ]]
[[ "$binary_url" != */latest/* ]]
[[ "$binary_sha256" =~ ^[0-9a-f]{64}$ ]]
[[ "$archive_name" =~ ^vidmetry-ffmpeg-[A-Za-z0-9.-]+-corresponding-source\.tar\.xz$ ]]
[[ "$archive_sha256" =~ ^[0-9a-f]{64}$ ]]
[[ "$asset_tag" =~ ^ffmpeg-source-[A-Za-z0-9.-]+$ ]]
[[ "$source_toolchain_image" =~ ^ghcr\.io/[a-z0-9._/-]+@sha256:[0-9a-f]{64}$ ]]
source_toolchain_tag="${source_toolchain_image%@sha256:*}:latest"
[[ "$source_toolchain_tag" == "ghcr.io/btbn/ffmpeg-builds/base:latest" ]]
[[ "$build_repository" == "https://github.com/BtbN/FFmpeg-Builds.git" ]]
[[ "$ffmpeg_repository" == "https://github.com/FFmpeg/FFmpeg.git" ]]
[[ "$build_commit" =~ ^[0-9a-f]{40}$ ]]
[[ "$ffmpeg_commit" =~ ^[0-9a-f]{40}$ ]]

if [[ "$VALIDATE_ONLY" == "1" ]]; then
  echo "Validated corresponding-source manifest for $engine_id."
  exit 0
fi

[[ -n "$OUTPUT" ]] || { usage; exit 2; }
[[ "$(basename -- "$OUTPUT")" == "$archive_name" ]] || {
  echo "Output filename must match manifest archiveName: $archive_name" >&2
  exit 2
}

for tool in docker git sha256sum tar xz; do
  command -v "$tool" >/dev/null
done

# BtbN's source acquisition entry point invokes its base image by a moving
# `latest` tag. Resolve that name locally to the manifest-pinned digest before
# any upstream download command runs, so both acquisition and normalization use
# one immutable toolchain image.
docker pull "$source_toolchain_image" >/dev/null
docker tag "$source_toolchain_image" "$source_toolchain_tag"

work_root="$(mktemp -d)"
trap 'rm -rf -- "$work_root"' EXIT
checkout_root="$work_root/checkouts"
stage_root="$work_root/stage"
source_root_name="${archive_name%.tar.xz}"
source_root="$stage_root/$source_root_name"
build_checkout="$checkout_root/FFmpeg-Builds"
ffmpeg_checkout="$checkout_root/FFmpeg"
mkdir -p "$checkout_root" "$source_root/build-scripts/.cache/downloads" "$source_root/ffmpeg"

git clone --filter=blob:none --no-checkout "$build_repository" "$build_checkout"
git -C "$build_checkout" fetch --depth=1 origin "$build_commit"
git -C "$build_checkout" checkout --detach "$build_commit"
[[ "$(git -C "$build_checkout" rev-parse HEAD)" == "$build_commit" ]]

if [[ -n "$CACHE_DIR" ]]; then
  mkdir -p "$CACHE_DIR" "$build_checkout/.cache"
  CACHE_DIR="$(cd -- "$CACHE_DIR" && pwd)"
  rm -rf -- "$build_checkout/.cache/downloads"
  ln -s "$CACHE_DIR" "$build_checkout/.cache/downloads"
fi

(
  cd "$build_checkout"
  env -u GITHUB_REPOSITORY -u REGISTRY_OVERRIDE -u DOCKER_TAG_SUFFIX \
    ./download.sh
  env -u GITHUB_REPOSITORY -u REGISTRY_OVERRIDE -u DOCKER_TAG_SUFFIX \
    ./generate.sh win64 gpl
)

mapfile -t dependency_archives < <(
  grep -oE '\.cache/downloads/[A-Za-z0-9._-]+\.tar\.xz' "$build_checkout/Dockerfile" | sort -u
)
if [[ "${#dependency_archives[@]}" -lt 20 ]]; then
  echo "The selected GPL build graph exposed too few dependency sources: ${#dependency_archives[@]}" >&2
  exit 1
fi

git -C "$build_checkout" archive --format=tar HEAD | tar -xf - -C "$source_root/build-scripts"
cp -- "$build_checkout/Dockerfile" "$source_root/build-scripts/Dockerfile.vidmetry-source-graph"
repack_jobs="${SOURCE_REPACK_JOBS:-4}"
if [[ ! "$repack_jobs" =~ ^[1-8]$ ]]; then
  echo "SOURCE_REPACK_JOBS must be an integer from 1 through 8." >&2
  exit 2
fi
dependency_list="$work_root/dependency-archives.txt"
for relative_archive in "${dependency_archives[@]}"; do
  source_archive="$build_checkout/$relative_archive"
  [[ -f "$source_archive" ]] || {
    echo "Missing dependency source archive selected by the build graph: $relative_archive" >&2
    exit 1
  }
  basename -- "$source_archive" >> "$dependency_list"
done

# The marker is written only after the complete source graph has been acquired.
# Workflows may persist this cache even when a later normalization or checksum
# check fails, avoiding another network-bound acquisition on the next audit.
if [[ -n "$CACHE_DIR" ]]; then
  cache_marker="$CACHE_DIR/.vidmetry-source-cache-v1"
  cache_marker_tmp="$cache_marker.tmp"
  {
    printf 'engine=%s\n' "$engine_id"
    printf 'buildCommit=%s\n' "$build_commit"
    printf 'sourceToolchainImage=%s\n' "$source_toolchain_image"
    printf 'dependencyArchiveCount=%s\n' "${#dependency_archives[@]}"
    sort -- "$dependency_list"
  } > "$cache_marker_tmp"
  mv -- "$cache_marker_tmp" "$cache_marker"
fi

resolved_downloads="$(cd -P -- "$build_checkout/.cache/downloads" && pwd)"
resolved_scripts="$(cd -P -- "$SCRIPT_DIR" && pwd)"
resolved_dependency_list="$(cd -P -- "$(dirname -- "$dependency_list")" && pwd)/$(basename -- "$dependency_list")"
resolved_dependency_output="$(cd -P -- "$source_root/build-scripts/.cache/downloads" && pwd)"

docker run --rm --network none --read-only --cap-drop ALL \
  --security-opt no-new-privileges --user "$(id -u):$(id -g)" \
  --tmpfs /tmp:rw,nosuid,nodev \
  --mount "type=bind,src=$resolved_scripts,dst=/opt/vidmetry-scripts,readonly" \
  --mount "type=bind,src=$resolved_downloads,dst=/input,readonly" \
  --mount "type=bind,src=$resolved_dependency_list,dst=/archive-list.txt,readonly" \
  --mount "type=bind,src=$resolved_dependency_output,dst=/output" \
  "$source_toolchain_image" \
  bash /opt/vidmetry-scripts/repack-source-directory.sh \
    /input /output /archive-list.txt "$repack_jobs"

git clone --filter=blob:none --no-checkout "$ffmpeg_repository" "$ffmpeg_checkout"
git -C "$ffmpeg_checkout" fetch --depth=1 origin "$ffmpeg_commit"
git -C "$ffmpeg_checkout" checkout --detach "$ffmpeg_commit"
[[ "$(git -C "$ffmpeg_checkout" rev-parse HEAD)" == "$ffmpeg_commit" ]]
git -C "$ffmpeg_checkout" archive --format=tar HEAD | tar -xf - -C "$source_root/ffmpeg"

jq -n \
  --arg engineId "$engine_id" \
  --arg license "$license" \
  --arg variant "$variant" \
  --arg sourceToolchainImage "$source_toolchain_image" \
  --arg binaryUrl "$binary_url" \
  --arg binarySha256 "$binary_sha256" \
  --arg buildRepository "$build_repository" \
  --arg buildCommit "$build_commit" \
  --arg ffmpegRepository "$ffmpeg_repository" \
  --arg ffmpegCommit "$ffmpeg_commit" \
  --argjson dependencyArchiveCount "${#dependency_archives[@]}" \
  '{
    schemaVersion: 2,
    engineId: $engineId,
    license: $license,
    variant: $variant,
    sourceToolchainImage: $sourceToolchainImage,
    conveyedBinary: { url: $binaryUrl, sha256: $binarySha256 },
    buildDefinition: { repository: $buildRepository, commit: $buildCommit },
    ffmpegSource: { repository: $ffmpegRepository, commit: $ffmpegCommit },
    dependencyArchiveCount: $dependencyArchiveCount
  }' > "$source_root/SOURCE_METADATA.json"

cat > "$source_root/README.md" <<'EOF'
# FFmpeg Complete Corresponding Source

This archive accompanies the FFmpeg and ffprobe object code conveyed in the same Vidmetry release.

- `ffmpeg/` is the exact FFmpeg source revision used by the binary.
- `build-scripts/` is the exact BtbN/FFmpeg-Builds revision containing the Windows GPL build controls, patches, Docker definitions, and license-selection logic.
- `build-scripts/.cache/downloads/` contains every dependency source archive selected by the generated `win64-gpl` build graph.
- Dependency archives contain the preferred source trees with checkout-specific `.git` administration data removed and deterministic metadata applied.
- External symbolic links are removed from dependency archives and recorded in an affected archive's `.vidmetry-normalization/removed-external-symlinks.tsv`; internal links are preserved.
- The digest-pinned source toolchain image used for dependency acquisition and deterministic archive creation is recorded in `SOURCE_METADATA.json`.
- `build-scripts/Dockerfile.vidmetry-source-graph` records the resolved dependency graph and configuration used to select those archives.
- `SOURCE_SHA256SUMS` authenticates every file in the archive other than the checksum list itself.

The upstream build entry points are `build-scripts/makeimage.sh win64 gpl` and `build-scripts/build.sh win64 gpl`. The included FFmpeg tree replaces the moving branch checkout in the upstream build script when reproducing this historical binary. General-purpose build tools and operating-system system libraries are not included.

The sources retain their original licenses. The binary's GPL license text is distributed next to the installed executables and with the Vidmetry repository notices.
EOF

mkdir -p "$(dirname -- "$OUTPUT")"
resolved_stage_root="$(cd -P -- "$stage_root" && pwd)"
resolved_output_parent="$(mkdir -p "$(dirname -- "$OUTPUT")" && cd -P -- "$(dirname -- "$OUTPUT")" && pwd)"
docker run --rm --network none --read-only --cap-drop ALL \
  --security-opt no-new-privileges --user "$(id -u):$(id -g)" \
  --tmpfs /tmp:rw,nosuid,nodev \
  --mount "type=bind,src=$resolved_scripts,dst=/opt/vidmetry-scripts,readonly" \
  --mount "type=bind,src=$resolved_stage_root,dst=/input" \
  --mount "type=bind,src=$resolved_output_parent,dst=/output" \
  "$source_toolchain_image" \
  bash /opt/vidmetry-scripts/archive-source-tree.sh \
    /input "$source_root_name" "/output/$(basename -- "$OUTPUT")"
(
  cd "$(dirname -- "$OUTPUT")"
  sha256sum "$(basename -- "$OUTPUT")" > "$(basename -- "$OUTPUT").sha256"
)

actual_archive_sha256="$(sha256sum "$OUTPUT" | cut -d ' ' -f 1)"
if [[ "$actual_archive_sha256" != "$archive_sha256" ]]; then
  echo "Corresponding-source archive checksum mismatch. Expected $archive_sha256 but generated $actual_archive_sha256." >&2
  echo "Deterministic source-tree fingerprints:" >&2
  sha256sum "$source_root/SOURCE_SHA256SUMS" >&2
  grep -F '  ./ffmpeg/' "$source_root/SOURCE_SHA256SUMS" | sha256sum >&2
  grep -F '  ./build-scripts/.cache/downloads/' \
    "$source_root/SOURCE_SHA256SUMS" | sha256sum >&2
  grep -F '  ./build-scripts/' "$source_root/SOURCE_SHA256SUMS" \
    | grep -Fv '  ./build-scripts/.cache/downloads/' | sha256sum >&2
  grep -F '  ./build-scripts/.cache/downloads/' \
    "$source_root/SOURCE_SHA256SUMS" >&2
  exit 1
fi

echo "Created $OUTPUT with ${#dependency_archives[@]} dependency source archives."
