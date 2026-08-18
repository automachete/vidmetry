# Third-party notices

Vidmetry distributes FFmpeg and ffprobe as separate command-line programs. Vidmetry does not link to FFmpeg libraries or exchange FFmpeg-internal data structures; it invokes the executables with ordinary arguments, files, and progress text. Each Windows package is an aggregate whose components retain their own licenses.

## FFmpeg N-126168-gb16b5f2a01-20260815

- Build variant: `win64-gpl`
- License: GNU General Public License version 3 or later (`GPL-3.0-or-later`)
- Binary provider release: <https://github.com/BtbN/FFmpeg-Builds/releases/tag/autobuild-2026-08-15-13-02>
- Binary archive SHA-256: `70dc4194cf95d10036b3951f3ddbfa0604fb9208767b8dbbdc4f8a657fb3f292`
- Build definition: <https://github.com/BtbN/FFmpeg-Builds/tree/590a6612d7d961e9258429e501619e0b7d7cbedf>
- FFmpeg source: <https://github.com/FFmpeg/FFmpeg/tree/b16b5f2a01f3c4f8c9a7769d7a35e8b193946d3e>
- FFmpeg legal information: <https://ffmpeg.org/legal.html>

Every official Vidmetry release that conveys these executables also provides equivalent, no-charge access to `vidmetry-ffmpeg-N-126168-gb16b5f2a01-20260815-corresponding-source.tar.xz` and its SHA-256 file next to the Windows packages. The source archive contains the exact FFmpeg tree, the pinned BtbN build scripts and patches, and every dependency source archive selected by the resolved `win64-gpl` build graph.

Each MSIX or NSIS package includes:

- `FFmpeg/FFMPEG_LICENSE.txt`: the complete GPL license shipped with the binary;
- `FFmpeg/FFMPEG_BUILD_INFO.txt`: the exact binary, source, build, configuration, and runtime report;
- `FFmpeg/FFMPEG_CORRESPONDING_SOURCE.txt`: the direct source-archive URL for that Vidmetry release.

Vidmetry's MIT license does not replace, restrict, or modify the terms that apply to FFmpeg and ffprobe. Codec patent rights, if any apply in a jurisdiction, are separate from copyright-license permissions.

## Rust packages under MPL-2.0

The packaged Vidmetry executable includes unmodified MPL-2.0 components pulled transitively by Tauri: `cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`, and `selectors`. Every MSIX or NSIS package includes their exact crates.io Source Form archives, verified against `Cargo.lock`, together with the complete MPL-2.0 text and a source index under `LicenseSources/MPL-2.0`. All other rights in the executable remain subject to their respective licenses.

## Other application dependencies

Every MSIX or NSIS package also includes generated, package-by-package copyright and license reports for the complete Rust and production JavaScript dependency graphs under `ThirdPartyLicenses`. The reports are rebuilt from the locked dependency graph during CI and Release builds; a license outside the reviewed allowlist fails the build.
