# Vidmetry 0.1.0 Verification Record

Verification date: 2026-08-15  
Platform: Windows x64  
Media engine: FFmpeg/ffprobe 9.0.1 essentials build

## Automated checks

| Check | Result |
|---|---|
| `npm run check` | Pass — 0 errors, 0 warnings |
| `npm test` | Pass — 12 tests across 3 files |
| `npm run build` | Pass — Vite production build |
| `cargo test --manifest-path src-tauri\Cargo.toml` | Pass — 8 Rust unit tests |
| `cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings` | Pass |
| `scripts/test-integration.ps1` | Pass |
| `npm run tauri build` | Pass — MSI and NSIS bundles |
| Tauri development window launch | Pass — responsive native window |

The MSVC linker emits a localized informational message while producing the Rust `cdylib` import library. It is surfaced by Cargo as `linker_messages` but is not a compiler or Clippy diagnostic.

## Media integration fixture

The test script generates an H.264/AAC 1280×720, 30 fps source and crops `{x:100, y:100, width:640, height:360}`.

| Profile | Probed result | Additional assertion |
|---|---|---|
| Compatible | H.264, 640×360, yuv420p | Physical crop and compatible codec |
| Lossless | FFV1, 640×360, yuv420p | Every decoded-frame MD5 matches the source crop |
| Metadata-only | H.264 stream copy, displayed 640×360 | No video encoder used |

The source SHA-256 before and after all exports is identical. Temporary test media is written only under the ignored `test-results` directory.

## Local release artifacts

These artifacts were generated from the verified 0.1.0 source tree. They are build outputs and are intentionally not committed.

| Artifact | Size | SHA-256 |
|---|---:|---|
| `Vidmetry_0.1.0_x64_en-US.msi` | 75.31 MiB | `833BC7126090AB99F843B2174B156EC9302A5672E3AB2767FAE5C5A94074F18A` |
| `Vidmetry_0.1.0_x64-setup.exe` | 54.85 MiB | `50FB7E51469A90E5915BB63EFF7555CD33CE323A491546B3A81EA1A5423A81EE` |

## Remaining manual acceptance

- Exercise the native open/save dialogs and all crop handles with representative personal footage.
- Cover 4K HEVC 10-bit, rotated phone MOV, VFR, multi-audio MKV, Unicode paths, and low-disk/permission failures.
- Confirm metadata-only rendering in each target player because support is deliberately player-dependent.
