# Vidmetry 0.4.3 Verification Record

Verification date: 2026-08-16
Platform: Windows x64  
Media engine: FFmpeg/ffprobe 9.0.1 essentials build

## Automated checks

| Check | Result |
|---|---|
| `npm run check` | Pass — 0 errors, 0 warnings |
| `npm run test:assets` | Pass — SVG, generated PNG/ICO pixels, legacy tint list, and NSIS shortcut-refresh configuration |
| `npm test` | Pass — 41 tests across 8 files |
| `npm run test:ui` | Pass — 10 Chromium scenarios and 5 screenshot baselines |
| `npm run build` | Pass — Vite production build |
| `cargo test --manifest-path src-tauri\Cargo.toml` | Pass — 17 Rust unit tests |
| `cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings` | Pass |
| `scripts/test-integration.ps1` | Pass |
| `npm run tauri build` | Pass — MSI and NSIS bundles |
| Component interaction | Pass — Windows mode/accent projection, launcher/logo, settings, save shortcuts/menu, click-to-focus and visible trim-handle selection, focused end-handle Space restart, start/end 1/10-frame steps, collapsible panes, F11 state, notification dismissal, localization, and Explorer reveal |
| Playback-state regression | Pass — the UI media mock tracks `paused`, emits play/pause events, and verifies that Space from the selected end handle moves from 7.6 s to the selected 0.3667 s start, enters playing state, advances another frame without stopping, and displays Pause |
| Pointer alignment regression | Pass — after selecting frames `[60, 180)` of a 240-frame video, a physical timeline click and an off-center handle drag remain aligned within half-frame rendering tolerance |
| Packaged executable smoke launch | Pass — remained running until test shutdown; extracted executable icon contains only achromatic pixels |
| Installed shortcut upgrade | Pass — NSIS updated the existing desktop shortcut to `shortcut-icon-achromatic-v2.ico`, refreshed its timestamp, and installed an achromatic icon |

The MSVC linker emits a localized informational message while producing the Rust `cdylib` import library. It is surfaced by Cargo as `linker_messages` but is not a compiler or Clippy diagnostic.

## Media integration fixture

The test script generates an H.264/AAC 1280×720, 30 fps source and crops `{x:100, y:100, width:640, height:360}`.

| Profile | Probed result | Additional assertion |
|---|---|---|
| Compatible | H.264, 640×360, yuv420p | Physical crop and compatible codec |
| Configured compatible | HEVC, 640×360, yuv420p10le | H.265, CRF/preset, 10-bit format, AAC, and CFR options applied |
| Lossless | FFV1, 640×360, yuv420p | Every decoded-frame MD5 matches the source crop |
| Metadata-only | H.264 stream copy, displayed 640×360 | No video encoder used |
| Time trim | H.264/AAC, 640×360, 60 frames, 2.000 s | Source frame range `[30, 90)` applied exactly |
| In-place staging | H.264, 640×360 | Temporary output replaces a copied source only after completion |

The source SHA-256 before and after all exports is identical. Temporary test media is written only under the ignored `test-results` directory.

## Local release artifacts

These artifacts were generated from the verified 0.4.3 source tree. They are build outputs and are intentionally not committed.

| Artifact | Size | SHA-256 |
|---|---:|---|
| `Vidmetry_0.4.3_x64_en-US.msi` | 75.36 MiB | `08111A531DE1E7406D77808E5F2CF0079EC840F36C2B7B0C0F74B67586B9C546` |
| `Vidmetry_0.4.3_x64-setup.exe` | 54.90 MiB | `A6E3016A607D56333F84933CF65DDDE6F6C81D75360A2DCF4C83A1E68B8204D4` |

## Remaining manual acceptance

- Exercise the native file/folder/save dialogs, drag/drop, Page Up/Page Down, and all crop handles with representative personal footage.
- Cover 4K HEVC 10-bit, rotated phone MOV, VFR, multi-audio MKV, Unicode paths, and low-disk/permission failures.
- Confirm metadata-only rendering in each target player because support is deliberately player-dependent.
- Confirm Explorer selection, live Windows personalization changes, pane controls, F11/Escape, and velocity-sensitive pointer feel in the packaged WebView with real personal media.
