# Vidmetry 0.4.4 Verification Record

Verification date: 2026-08-16
Platform: Windows x64  
Media engine: FFmpeg/ffprobe 9.0.1 essentials build

## Automated checks

| Check | Result |
|---|---|
| `npm run check` | Pass — 0 errors, 0 warnings |
| `npm run test:assets` | Pass — SVG, generated PNG/ICO pixels, legacy tint list, and NSIS shortcut-refresh configuration |
| `npm test` | Pass — 41 tests across 8 files |
| `npm run test:ui` | Pass — 11 Chromium scenarios and 5 screenshot baselines |
| `npm run build` | Pass — Vite production build |
| `cargo test --manifest-path src-tauri\Cargo.toml` | Pass — 17 Rust unit tests |
| `cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings` | Pass |
| `scripts/test-integration.ps1` | Pass |
| `npm run tauri build` | Pass — MSI and NSIS bundles |
| Component interaction | Pass — Windows mode/accent projection, launcher/logo, settings, save shortcuts/menu, explicit playback-position/trim-boundary terminology, focused white playback-position Space control, start/end trim-boundary 1/10-frame steps, collapsible panes, F11 state, notification dismissal, localization, and Explorer reveal |
| Playback-state regression | Pass — Playwright clicks the rendered white playback-position handle, verifies focus on its full-width scrubber, presses Space, and checks `paused` changes from true to false, a play event is emitted, and the UI displays Pause |
| Pointer alignment regression | Pass — after selecting frames `[60, 180)` of a 240-frame video, a physical timeline click and an off-center handle drag remain aligned within half-frame rendering tolerance |
| Packaged executable smoke launch | Pass — remained running until test shutdown; extracted executable icon contains only achromatic pixels |
| Installed application | Pass — NSIS updated the local installation to 0.4.4, preserved the achromatic shortcut icon, and the installed executable passed a smoke launch |

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

These artifacts were generated from the verified 0.4.4 source tree. They are build outputs and are intentionally not committed.

| Artifact | Size | SHA-256 |
|---|---:|---|
| `Vidmetry_0.4.4_x64_en-US.msi` | 75.36 MiB | `15DFA1AE381BB241F96790A12FC29A310F54D3B7D1A25D12C3C7969F926A479C` |
| `Vidmetry_0.4.4_x64-setup.exe` | 54.88 MiB | `D059FAC5469BE4580C3F9308B22FB895A5D84D66CB6C6A9DEC52E9CFF2A92243` |

## Remaining manual acceptance

- Exercise the native file/folder/save dialogs, drag/drop, Page Up/Page Down, and all crop handles with representative personal footage.
- Cover 4K HEVC 10-bit, rotated phone MOV, VFR, multi-audio MKV, Unicode paths, and low-disk/permission failures.
- Confirm metadata-only rendering in each target player because support is deliberately player-dependent.
- Confirm Explorer selection, live Windows personalization changes, pane controls, F11/Escape, and velocity-sensitive pointer feel in the packaged WebView with real personal media.
