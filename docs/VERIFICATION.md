# Vidmetry 0.4.8 Verification Record

Verification date: 2026-08-24
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
| `npm run test:licenses` | Pass — GPL corresponding-source, MPL source, third-party notice, Windows-package resources, and fail-closed Release contracts |
| `npm run check` | Pass — 0 errors, 0 warnings |
| `npm run test:assets` | Pass — SVG, generated PNG/ICO pixels, and legacy tint list |
| `npm run test:shell` | Pass — MSIX and NSIS cover all 16 supported video extensions and selected directories; MSI bundling remains absent |
| `npm run test:nsis` | Pass — package filename and version contract |
| `npm run test:nsis -- --LiveInstall` | Not run locally — the existing current-user Vidmetry 0.4.7 installation was preserved; the clean Release runner performs this test before publication |
| `npm run test:msix` | Pass — unpacked manifest, classic-app activation, all 16 file associations, packaged COM directory command, x64 PE and unmarked Tauri payload, locked FFmpeg sidecars, license/source payloads, and build-file exclusions |
| `npm run test:runtime` | Pass — pinned Node/npm/Rust, immutable FFmpeg manifest, sidecar hashes/notices, and full-SHA GitHub Actions references |
| `npm test` | Pass — 69 tests across 11 files |
| `npm run test:ui` | Pass — 17 Chromium scenarios covering semantic controls, immediate settings including the standard-default/Explorer-view-Beta folder-picker switch, all 16 shortcut assignments and live tooltip updates, both folder-dialog contracts, stable settings dimensions, paired-control alignment and contained focus visuals, computed styles, every English/Japanese settings category without clipping or unintended wrapping, and minimum-window overflow |
| `npm run test:coverage` | Pass — 94.28% statements, 81.09% branches, 97.67% functions, 95.33% lines |
| `npm run build` | Pass — Vite production build |
| `npm run tauri build -- --no-bundle` | Pass — optimized Windows application build |
| `cargo fmt --check --manifest-path src-tauri\Cargo.toml` | Pass |
| `cargo test --manifest-path src-tauri\Cargo.toml` | Pass — 48 Rust unit tests, including neutral export staging and explicit muxer selection, bounded sharing-lock retries against both simulated and real Windows locks, the complete supported-video filter, Windows UI language/color-mode selection, and native Explorer picker view/navigation behavior |
| `cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings` | Pass |
| `scripts/test-integration.ps1` | Pass — H.264/H.265 selected available hardware encoders; FFV1 used parallel slices and an explicit Matroska muxer through neutral `.tmp` staging; late input seek preserved all 60 frame hashes and decoded audio samples |
| `npm audit --audit-level=high` | Pass — 0 vulnerabilities |
| `cargo audit --file src-tauri\Cargo.lock` | Pass — 0 vulnerabilities; 17 allowed informational warnings in Tauri's cross-target dependency graph |
| `actionlint` 1.7.12 | Pass — CI and release workflows |
| Immutable FFmpeg source architecture | Pass — engine tag and archive SHA-256 are manifest-pinned; source assembly has read-only repository permission and no persisted checkout credential; the write-capable publisher receives only the reverified two-file artifact and never overwrites it |
| Application Release architecture | Pass — public/version/source preflight and Rust license audit precede parallel source transfer and Windows verification; the isolated publisher requires one NSIS, one MSIX, the corresponding-source archive, and its checksum, and rechecks public access immediately before publication |
| `scripts/build-msix.ps1 -SkipAppBuild` | Pass — x64 MSIX with packaged-classic activation and File Explorer COM integration; the newest versioned Windows SDK is selected even when an unversioned SDK tool path exists |
| Windows-package license payload | Pass — both packages contain the FFmpeg license/source notice, complete dependency reports, MPL text, and all 5 MPL source archives |
| Component interaction | Pass — Windows/manual mode and Fluent accent projection, Windows type/control scale and responsive pane balance, fixed one-level settings geometry, ordered immediate saving without a redundant saved label, frequency-ordered key capture and all 16 preview accelerator defaults, launcher/logo, Explorer-integration toggling, startup Shell paths, save shortcuts/menu, focused playback-position control, regular/fullscreen directory playback carry, live directory additions, locked trim export ranges, authoritative ffprobe timing, same-path overwrite reload, collapsible panes, fullscreen state, notification dismissal, structured command/event error localization, Explorer reveal, event-registration failure, rejected playback, durable-settings failure, multi-path drop validation, and playlist rollback |
| Localization boundary | Pass — i18next resolves typed Japanese/English resources; backend errors serialize generated stable codes with optional details; Japanese product text outside the resource is rejected automatically |
| Persistence and cache lifecycle | Pass — strict Zod rejection of incomplete, unknown, obsolete, or out-of-range settings; serialized immediate writes and Explorer rollback on failure; both folder-picker modes restore the last confirmed path, while the Beta picker reapplies its selected Shell view during directory navigation and restores its view mode and icon size across repeated selection and application restart; package-specific directory-command visibility retained across updates while video Open with entries remain installed; unchanged media-probe reuse, explicit post-replacement invalidation, and fresh WebView asset revisions; non-empty staged cache promotion; retained-entry-safe count/size/age pruning |
| Playback-state regression | Pass — Playwright verifies Space playback and confirms that previous/next navigation resumes the destination video in regular and fullscreen directory preview |
| Directory synchronization | Pass — native change events are filtered and coalesced; component and Chromium tests reflect Explorer additions and completed copy saves while retaining the current item |
| Export-state regression | Pass — an in-place replacement reloads changed geometry, duration, frame count, and a fresh media URL; export snapshots the exact trim range and disables its controls before asynchronous saving; sibling staging files end in `.tmp`, stay outside video enumeration, and Windows finalization survives a temporary sharing lock without retrying permanent I/O errors |
| Pointer alignment regression | Pass — after selecting frames `[60, 180)` of a 240-frame video, a physical timeline click and an off-center handle drag remain aligned within half-frame rendering tolerance |
| Release executable smoke launch | Pass — remained running until test shutdown; extracted executable icon contains only achromatic pixels |
| Native folder-picker smoke | Pass — on a Japanese Windows UI in Dark mode, the opt-in `IExplorerBrowser` Beta host rendered localized Shell navigation and content; its title bar, compact Back/Forward/Up row, clickable address breadcrumbs, and confirmation row used the same Dark mode; legacy command/detail/preview panes and the old bottom Details/Large-icons controls were absent; one top-right View button showed the localized tooltip and expanded a Dark-mode menu containing Extra large icons, Large icons, Medium icons, Small icons, List, Details, Tiles, and Content with the active view checked; folders and supported video files remained visible. In the release build, Details stayed selected after clicking a parent breadcrumb and after Back returned through history; cancellation left the persisted folder path, mode, view mode, and icon size unchanged |

The MSVC linker emits a localized informational message while producing the Rust `cdylib` import library. It is surfaced by Cargo as `linker_messages` but is not a compiler or Clippy diagnostic.

## Media integration fixture

The test script generates an H.264/AAC 1280×720, 30 fps source and crops `{x:100, y:100, width:640, height:360}`.

| Profile | Probed result | Additional assertion |
|---|---|---|
| Compatible | H.264 via `nvenc`, 640×360, yuv420p | Physical crop, compatible codec, and color metadata preserved |
| Configured compatible | HEVC via `nvenc`, 640×360, yuv420p10le | H.265, quality/preset, 10-bit format, AAC, CFR, and color metadata applied |
| Lossless | FFV1 with 64 slices, 640×360, yuv420p | Every decoded-frame MD5 matches the source crop; color metadata preserved |
| Metadata-only | H.264 stream copy, displayed 640×360 | No video encoder used |
| Time trim | H.264/AAC, 640×360, 60 frames, 2.000 s | Source frame range `[30, 90)` applied exactly |
| Late time trim | FFV1/FLAC, 320×180, 60 frames, 2.000 s | Input seek starts near frame range `[3000, 3060)`; frame and decoded-audio hashes match full decoding |
| In-place staging | H.264, 640×360 | Temporary output replaces a copied source only after completion |

The source SHA-256 before and after all exports is identical. Temporary test media is written only under the ignored `test-results` directory.

## Hardware encoding benchmark

An 8-second 3840×2160 source was cropped to 3200×1800 and encoded with the bundled FFmpeg on an RTX 5070 Ti using NVIDIA driver 610.88. H.264 completed in 1.62 seconds with `nvenc` versus 2.25 seconds with `libx264` (28% shorter). H.265 completed in 1.67 seconds with `nvenc` versus 3.23 seconds with `libx265` (48% shorter).

The one-frame startup probes reported `nvenc` and `amf` available for both H.264 and H.265, and `qsv` unavailable on this machine. Running all six probes serially took 739 ms; the application runs the three encoder probes concurrently and tests the two codecs sequentially per encoder.

## Export pipeline benchmark

A 60-frame trim near the end of a 5-minute 640×360 CFR H.264 source averaged 0.065 seconds with the five-second pre-roll input seek versus 0.344 seconds when decoding from the beginning (81% shorter). The two frame-MD5 outputs were identical. Encoding a 5-second 1920×1080 FFV1 test source averaged 0.239 seconds with 32 threads and 128 slices versus 0.395 seconds with FFmpeg defaults (39% shorter), across three runs per case.

## Local release artifacts

These artifacts were generated from the verified 0.4.8 source tree. The unsigned MSIX uses the Partner Center `automachete.Vidmetry` / `CN=253F7C9B-963E-4633-A199-6AD8D2D25034` identity. The NSIS is the unsigned direct-download package. Build outputs are intentionally not committed.

| Artifact | Size | SHA-256 |
|---|---:|---|
| `Vidmetry_0.4.8_x64-setup.exe` | 81.49 MiB | `D93D76D91D8FA6A01B325B2FE12E88750E1D583220E905E86355D17839EB965E` |
| `Vidmetry_0.4.8.0_x64.msix` | 114.36 MiB | `2DF62482ABFE5F1AE2950BA09BF0C55441CF81BEAD01E1365BCD02AFC9E79EEF` |

## Remaining manual acceptance

- Exercise the native file/save dialogs, drag/drop, Page Up/Page Down, and all crop handles with representative personal footage; extend the verified folder-dialog smoke across network and removable-drive paths.
- Cover 4K HEVC 10-bit, rotated phone MOV, VFR, multi-audio MKV, Unicode paths, and low-disk/permission failures.
- Confirm metadata-only rendering in each target player because support is deliberately player-dependent.
- Confirm Explorer selection and live folder additions, post-overwrite preview state, Windows personalization changes, pane controls, F11/Escape, and velocity-sensitive pointer feel in the packaged WebView with real personal media.
- Install the Store-signed MSIX on a disposable Windows user profile and confirm video Open with entries, the selected-folder Open with Vidmetry command, directory-command toggle latency, update preservation, and uninstall cleanup in the live Shell. The MSIX automated `--LiveInstall` path requires an elevated PowerShell session and was not run in this non-elevated workspace. The NSIS current-user live path was not rerun because the existing Vidmetry 0.4.7 installation was preserved; the clean Release runner performs it before publication.
