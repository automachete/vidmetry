# Vidmetry 0.4.7 Verification Record

Verification date: 2026-08-17
Platform: Windows x64  
Media engine: FFmpeg/ffprobe N-126168-gb16b5f2a01-20260815 `win64-gpl` build

## Automated checks

| Check | Result |
|---|---|
| `npm run check:contracts` | Pass — generated TypeScript/Rust error-code contracts match the shared source |
| `npm run check:licenses` | Pass — production JavaScript dependencies are inside the reviewed license allowlist |
| `cargo deny --manifest-path src-tauri\Cargo.toml check licenses` | Pass — the complete Rust graph is inside the reviewed license allowlist |
| `scripts/setup-copyleft-sources.ps1` | Pass — 5 exact MPL-2.0 Source Form archives and license text match the locked graph and SHA-256 manifest |
| `scripts/generate-third-party-licenses.ps1` | Pass — package-level Rust and production JavaScript license reports generated from locked dependencies |
| `npm run test:licenses` | Pass — GPL corresponding-source, MPL source, third-party notice, installer resource, and fail-closed Release contracts |
| `npm run check` | Pass — 0 errors, 0 warnings |
| `npm run test:assets` | Pass — SVG, generated PNG/ICO pixels, legacy tint list, and NSIS shortcut-refresh configuration |
| `npm run test:shell` | Pass — NSIS and MSI registrations match all 16 supported video extensions and include selected-directory handling, update-state preservation, and uninstall cleanup |
| `npm run test:runtime` | Pass — pinned Node/npm/Rust, immutable FFmpeg manifest, sidecar hashes/notices, and full-SHA GitHub Actions references |
| `npm test` | Pass — 56 tests across 10 files |
| `npm run test:ui` | Pass — 13 Chromium scenarios and 5 screenshot baselines |
| `npm run test:coverage` | Pass — 94.25% statements, 81.09% branches, 97.67% functions, 95.31% lines |
| `npm run build` | Pass — Vite production build |
| `cargo fmt --check --manifest-path src-tauri\Cargo.toml` | Pass |
| `cargo test --manifest-path src-tauri\Cargo.toml` | Pass — 30 Rust unit tests |
| `cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings` | Pass |
| `scripts/test-integration.ps1` | Pass — H.264 and H.265 selected NVENC on the RTX 5070 Ti |
| `npm audit --audit-level=high` | Pass — 0 vulnerabilities |
| `cargo audit --file src-tauri\Cargo.lock` | Pass — 0 vulnerabilities; 17 allowed informational warnings in Tauri's cross-target dependency graph |
| `actionlint` | Pass — CI and release workflows |
| Immutable FFmpeg source architecture | Pass — engine tag and archive SHA-256 are manifest-pinned; source assembly has read-only repository permission and no persisted checkout credential; the write-capable publisher receives only the reverified two-file artifact and never overwrites it |
| Application Release architecture | Pass — public/version/source preflight and Rust license audit precede parallel source transfer and Windows verification; the isolated publisher requires the exact four-asset set and rechecks public access immediately before publication |
| `npm run tauri build` | Pass — MSI and NSIS bundles, including their File Explorer registration definitions |
| Installer license payload | Pass — both bundle definitions contain FFmpeg license/source notice, complete dependency reports, MPL text, and all 5 MPL source archives |
| Component interaction | Pass — Windows mode/accent projection, launcher/logo, settings and Explorer-integration toggling, startup Shell paths, save shortcuts/menu, focused playback-position Space control, trim-boundary frame steps, authoritative ffprobe timing, collapsible panes, F11 state, notification dismissal, structured command/event error localization, Explorer reveal, event-registration failure, rejected playback, durable-settings failure, multi-path drop validation, and playlist rollback |
| Localization boundary | Pass — i18next resolves typed Japanese/English resources; backend errors serialize generated stable codes with optional details; Japanese product text outside the resource is rejected automatically |
| Persistence and cache lifecycle | Pass — strict Zod rejection of incomplete, unknown, obsolete, or out-of-range settings; disabled Explorer state retained across installer updates; non-empty staged cache promotion; retained-entry-safe count/size/age pruning |
| Playback-state regression | Pass — Playwright clicks the rendered playback-position handle, verifies focus on its full-width scrubber, presses Space, and checks `paused` changes from true to false, a play event is emitted, and the UI displays Pause |
| Pointer alignment regression | Pass — after selecting frames `[60, 180)` of a 240-frame video, a physical timeline click and an off-center handle drag remain aligned within half-frame rendering tolerance |
| Packaged executable smoke launch | Pass — remained running until test shutdown; extracted executable icon contains only achromatic pixels |

The MSVC linker emits a localized informational message while producing the Rust `cdylib` import library. It is surfaced by Cargo as `linker_messages` but is not a compiler or Clippy diagnostic.

## Media integration fixture

The test script generates an H.264/AAC 1280×720, 30 fps source and crops `{x:100, y:100, width:640, height:360}`.

| Profile | Probed result | Additional assertion |
|---|---|---|
| Compatible | H.264 via NVENC, 640×360, yuv420p | Physical crop, compatible codec, and color metadata preserved |
| Configured compatible | HEVC via NVENC, 640×360, yuv420p10le | H.265, quality/preset, 10-bit format, AAC, CFR, and color metadata applied |
| Lossless | FFV1, 640×360, yuv420p | Every decoded-frame MD5 matches the source crop; color metadata preserved |
| Metadata-only | H.264 stream copy, displayed 640×360 | No video encoder used |
| Time trim | H.264/AAC, 640×360, 60 frames, 2.000 s | Source frame range `[30, 90)` applied exactly |
| In-place staging | H.264, 640×360 | Temporary output replaces a copied source only after completion |

The source SHA-256 before and after all exports is identical. Temporary test media is written only under the ignored `test-results` directory.

## Hardware encoding benchmark

An 8-second 3840×2160 source was cropped to 3200×1800 and encoded with the bundled FFmpeg on an RTX 5070 Ti using NVIDIA driver 610.88. H.264 completed in 1.62 seconds with NVENC versus 2.25 seconds with `libx264` (28% shorter). H.265 completed in 1.67 seconds with NVENC versus 3.23 seconds with `libx265` (48% shorter).

## Local release artifacts

These artifacts were generated from the verified 0.4.7 source tree. They are build outputs and are intentionally not committed.

| Artifact | Size | SHA-256 |
|---|---:|---|
| `Vidmetry_0.4.7_x64_en-US.msi` | 110.96 MiB | `D40E369F015F53E1B44F70DCBE5F2442AB8A3BBDF992C345A2CD358F9317A526` |
| `Vidmetry_0.4.7_x64-setup.exe` | 81.43 MiB | `8FB4ADC6AACB5D3CF0AB44F8F58E07CE4BE3598C262A0AB4D1E785495291224D` |

## Remaining manual acceptance

- Exercise the native file/folder/save dialogs, drag/drop, Page Up/Page Down, and all crop handles with representative personal footage.
- Cover 4K HEVC 10-bit, rotated phone MOV, VFR, multi-audio MKV, Unicode paths, and low-disk/permission failures.
- Confirm metadata-only rendering in each target player because support is deliberately player-dependent.
- Confirm Explorer selection, live Windows personalization changes, pane controls, F11/Escape, and velocity-sensitive pointer feel in the packaged WebView with real personal media.
- Install each bundle on a disposable Windows user profile and confirm video Open with entries, the selected-folder Open with Vidmetry verb, toggle latency, update preservation, and uninstall cleanup in the live Shell.
