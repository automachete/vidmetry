# Vidmetry 0.2.0 Verification Record

Verification date: 2026-08-16
Platform: Windows x64  
Media engine: FFmpeg/ffprobe 9.0.1 essentials build

## Automated checks

| Check | Result |
|---|---|
| `npm run check` | Pass — 0 errors, 0 warnings |
| `npm test` | Pass — 20 tests across 5 files |
| `npm run build` | Pass — Vite production build |
| `cargo test --manifest-path src-tauri\Cargo.toml` | Pass — 12 Rust unit tests |
| `cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings` | Pass |
| `scripts/test-integration.ps1` | Pass |
| `npm run tauri build` | Pass — MSI and NSIS bundles |
| Settings component interaction | Pass — English/Japanese rendering and loop persistence |
| Packaged executable smoke launch | Pass — remained running until test shutdown |

The MSVC linker emits a localized informational message while producing the Rust `cdylib` import library. It is surfaced by Cargo as `linker_messages` but is not a compiler or Clippy diagnostic.

## Media integration fixture

The test script generates an H.264/AAC 1280×720, 30 fps source and crops `{x:100, y:100, width:640, height:360}`.

| Profile | Probed result | Additional assertion |
|---|---|---|
| Compatible | H.264, 640×360, yuv420p | Physical crop and compatible codec |
| Configured compatible | HEVC, 640×360, yuv420p10le | H.265, CRF/preset, 10-bit format, AAC, and CFR options applied |
| Lossless | FFV1, 640×360, yuv420p | Every decoded-frame MD5 matches the source crop |
| Metadata-only | H.264 stream copy, displayed 640×360 | No video encoder used |
| In-place staging | H.264, 640×360 | Temporary output replaces a copied source only after completion |

The source SHA-256 before and after all exports is identical. Temporary test media is written only under the ignored `test-results` directory.

## Local release artifacts

These artifacts were generated from the verified 0.2.0 source tree. They are build outputs and are intentionally not committed.

| Artifact | Size | SHA-256 |
|---|---:|---|
| `Vidmetry_0.2.0_x64_en-US.msi` | 75.33 MiB | `B8F721970CB1D47ABB889A61A23F00F7DC71812F6B31EC26A13FE244CF623C2B` |
| `Vidmetry_0.2.0_x64-setup.exe` | 54.88 MiB | `5D43B9DB02E3BB56574A4FE5C12A5FC5C43822FF93DEED6EDC82F87D12CADE63` |

## Remaining manual acceptance

- Exercise the native file/folder/save dialogs, drag/drop, Page Up/Page Down, and all crop handles with representative personal footage.
- Cover 4K HEVC 10-bit, rotated phone MOV, VFR, multi-audio MKV, Unicode paths, and low-disk/permission failures.
- Confirm metadata-only rendering in each target player because support is deliberately player-dependent.
