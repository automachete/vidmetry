# Vidmetry Software Design Description

| Field | Value |
|---|---|
| Document version | 1.8 |
| Product version | 0.4.8 |
| Status | Implemented and verified |
| Primary platform | Windows 11 x64 |
| UI languages | Japanese and English |
| Last updated | 2026-08-22 |

## 1. Purpose

Vidmetry is a lightweight desktop GUI for cropping a video's spatial frame and trimming its start/end time. Its interaction model follows the crop and frame-viewer tools in photo applications: open a video or folder, adjust the visible frame and selected duration, preview, and save with minimal intermediate UI.

The application must not imply that physical video cropping can normally use stream copy. It presents three distinct export behaviors so the user can choose compatibility, decoded-pixel fidelity, or metadata-only stream preservation.

### 1.1 Interaction terminology

The following terms are distinct and must not be shortened to an unqualified “handle” in requirements, code comments, or test names:

- **Playback-position handle**: the playhead shown over the frame strip. Its interactive control is the full-width playback scrubber. Pointer input seeks within the selected trim range; when this control has keyboard focus, Space toggles video play/pause and arrow keys seek.
- **Start trim-boundary handle / end trim-boundary handle**: the two controls that define the inclusive first frame and exclusive end frame. Pointer input or arrow keys change the saved trim range.
- **Spatial-crop handles**: the eight controls around the crop rectangle that change the exported frame area. They are unrelated to playback time.

## 2. Goals and non-goals

### 2.1 Goals

- Open and preview a local video without modifying it; replacement occurs only through explicit Save.
- Open a directory and move between its videos without returning to the picker.
- Display a draggable and resizable crop rectangle over a playable preview.
- Seek with a scrubber and play or pause while the crop overlay remains active.
- Trim the start and end on integer video-frame boundaries with velocity-sensitive pointer control.
- Show the final integer crop coordinates and output frame dimensions.
- Support arbitrary FFmpeg-decodable input by creating a temporary preview proxy when direct playback fails.
- Export with visible progress and cancellation.
- Offer compatible MP4, mathematically lossless FFV1, and metadata-only lossless modes.
- Configure export behavior once and start saving directly from the crop view.
- Remember export, language, appearance, shortcut, and loop-playback preferences between videos and sessions.
- Follow the Windows app mode and accent color by default while allowing independent user overrides.
- Keep all media processing local.
- Remain responsive while probing, proxying, and exporting.

### 2.2 Non-goals for 0.4.8

- Multi-clip timelines, internal cuts, transitions, filters, captions, or independent audio editing.
- Animated crop/keyframes.
- Cloud storage, accounts, telemetry, or automatic uploads.
- Batch-applying one crop rectangle to every file in a directory.
- Mobile builds.
- A promise that metadata-only crop works in every player.

## 3. Functional requirements

### 3.1 Import and inspection

- **FR-001** The user can select or drop one local video or a directory. The Windows folder picker hosts the native Explorer Shell view through `IExplorerBrowser`, showing supported video files alongside folders with Shell navigation and thumbnails, and provides one action in the standard confirmation position for selecting the folder currently being viewed. Shell and picker labels follow the supported locale matching the Windows UI language independently of the application's display language. Directory discovery is non-recursive and includes supported video extensions only.
- **FR-002** The backend probes the primary video stream and reports container, video/audio codecs, coded dimensions, display rotation, sample aspect ratio, pixel format, bit depth, frame-rate rationals, duration, and color metadata when present.
- **FR-003** Direct WebView playback is attempted first. On playback failure the user can generate, or the app can automatically generate, a temporary H.264 proxy from the original.
- **FR-004** A newly opened video starts with the crop rectangle covering the complete displayed frame.
- **FR-005** When a directory is active, the user can switch videos with an in-app list, previous/next controls, or Page Up/Page Down. File names are sorted case-insensitively. If playback is active, the destination video starts automatically in both regular and fullscreen preview.
- **FR-006** Both Windows packages expose Vidmetry in Open with for every supported video extension and expose Open with Vidmetry for selected directories. MSIX uses manifest associations and a packaged COM command; NSIS uses current-user registrations. Either entry point activates its installed application with the selected path.
- **FR-007** While a directory remains active, supported videos added by Vidmetry or another application are reflected without reopening the directory. Change bursts are coalesced, the current video is retained when present, and refreshes requested during export run after the export finishes.

### 3.2 Crop interaction

- **FR-010** The crop rectangle has four spatial-crop edge handles, four spatial-crop corner handles, and a draggable interior.
- **FR-011** The area outside the crop rectangle is visibly dimmed.
- **FR-012** The rectangle cannot leave the visible video frame and cannot become smaller than 16 by 16 source pixels.
- **FR-013** Coordinates are stored in display-oriented source pixels as `{x, y, width, height}` and normalized values are derived only for responsive rendering.
- **FR-014** For chroma-subsampled output, coordinates are snapped to the required pixel modulus; the initial supported export modulus is 2.
- **FR-015** The user can reset to full frame and choose free, source, 1:1, 4:3, 16:9, or 9:16 aspect constraints.
- **FR-016** Pixel fields accept keyboard input and update the rectangle after validation.

### 3.3 Playback and scrub

- **FR-020** The UI provides play/pause, current time, duration, mute, a frame strip, one playback-position handle backed by a full-width scrubber, and two trim-boundary handles.
- **FR-021** Start is inclusive and end is exclusive. The selected range always contains at least one integer source frame.
- **FR-022** The crop overlay remains spatially stable during playback, seek, resize, and fullscreen layout changes.
- **FR-023** Playback-scrubber clicks and trim-boundary dragging use the video's full-duration coordinate system even after the selected range is narrowed. Playback-position clicks clamp to the selection; a dragged trim-boundary handle centers on the final dispatched pointer position. Coalesced samples may estimate velocity but cannot replace that final position. Focused trim-boundary handles move by one frame with an arrow key or ten with Shift.
- **FR-024** An icon-only control toggles loop playback. The selected state is persisted and reused for subsequent videos and sessions.
- **FR-025** Playback and loop boundaries follow the selected time range. Space toggles playback when the playback-position handle/full-width scrubber has focus. Left/right keys on that control seek the preview; left/right keys on a trim-boundary handle adjust its boundary by one frame or ten with Shift.
- **FR-026** App mode and accent color independently default to Windows settings. The user can instead select Light or Dark and one of the reviewed Fluent accent colors. Changes apply immediately to WebView surfaces; mode choices also update the native window theme.
- **FR-027** The trim selection border and start/end trim-boundary handles follow the Windows accent-color setting with a computed readable foreground.
- **FR-028** The spatial crop inspector and time-trim footer can each be collapsed and restored with accessible icon-only controls.
- **FR-029** The configurable fullscreen-preview accelerator enters or exits a video-only window fullscreen preview and defaults to F11. Escape remains the standard exit action while fullscreen.

### 3.4 Export

- **FR-030** When output and source extensions differ, the header shows a direct Copy and save action with no menu. When they match, one combined Save options control opens Copy and save or confirmed in-place Save; there is no separate export-configuration screen.
- **FR-031** Compatible mode outputs H.264 or H.265 MP4, always enables fast start, and allows automatic or explicit encoder selection, quality level, encoder preset, pixel format, audio handling/bitrate, frame-rate handling, and metadata retention. A one-frame startup probe disables unavailable hardware choices for the selected codec. Automatic mode tries the bundled FFmpeg hardware encoders in preference order; an explicitly selected hardware encoder is tried directly. Both modes fall back to `libx264` or `libx265` when the hardware encoder cannot produce its first frame.
- **FR-032** Lossless mode outputs FFV1 in Matroska. The video is decoded, cropped, and encoded losslessly with CPU- and resolution-scaled slice parallelism while compatible audio/subtitle streams are copied.
- **FR-033** Metadata-only mode is available only for H.264 or HEVC. It copies encoded media while setting codec crop metadata. The UI warns that coded pixels and file size remain, and that some players ignore the crop.
- **FR-034** A physical crop filter is never combined with video stream copy.
- **FR-035** Rotation is handled explicitly so the exported rectangle matches the displayed orientation. Output rotation metadata is normalized.
- **FR-036** Audio is stream-copied when the output container supports it; compatible MP4 mode may fall back to AAC.
- **FR-037** FFmpeg progress is emitted to the UI at least twice per second when available.
- **FR-038** The user can cancel an export. A partial temporary output is not promoted to the selected final path.
- **FR-039** Successful export is written to a temporary sibling path and atomically renamed where the filesystem permits.
- **FR-040** Save is enabled only when the configured output extension equals the source extension. After explicit confirmation, the completed temporary output replaces the source.
- **FR-041** Copy and save never modifies the source and asks for a destination with an extension appropriate to the configured profile.
- **FR-042** A successful export shows a normalized display path as a link. Activating it opens File Explorer with the output file selected without launching the video.
- **FR-043** Compatible and lossless exports apply the selected ordinal frame range in FFmpeg. Late constant-frame-rate trims seek near the range before decoding and adjust the video/audio filters to preserve exact boundaries; uncertain or variable timing retains full decoding. Packet-copied audio is encoded when necessary to align it with an exact time trim.
- **FR-044** Metadata-only stream copy cannot promise arbitrary frame boundaries and is disabled while a time trim is active.
- **FR-045** A successful-save notice closes after three seconds or when another UI control is used.
- **FR-046** The configurable Copy and save and in-place Save accelerators default to Ctrl+S and Ctrl+Shift+S. In-place Save still requires confirmation and a profile that supports the source extension.
- **FR-047** A probed media descriptor is reused for export while its canonical source path, file length, and modification time remain unchanged. A changed source is probed again.
- **FR-048** After in-place Save replaces the source, the backend invalidates its probe entry and the UI reloads the media descriptor and a newly versioned preview URL so WebView and ffprobe state describe the same file generation.
- **FR-049** Starting export captures one immutable crop, trim, and export-settings snapshot before asynchronous work begins. Crop, playback scrub, and trim-boundary input cannot mutate that request while export is starting or running.

### 3.5 Common settings and localization

- **FR-050** An icon-only common-settings button is always available in the header.
- **FR-051** Settings persist locally and cover save profile, applicable encoder options, audio, frame rate, file metadata, loop playback, appearance, shortcuts, and File Explorer integration. Every valid change is applied and saved immediately through an ordered queue; there is no Apply action.
- **FR-052** Language mode is an exclusive choice between Windows language and manual selection. Manual selection supports Japanese and English; unsupported Windows languages fall back to English.
- **FR-053** All product-owned UI and runtime-error text comes from the Japanese/English locale resources. Rust commands and events return a stable language-neutral error code plus optional diagnostic detail; the presentation layer resolves that code in the active UI language. Language is the final item in the settings category navigation.
- **FR-054** File Explorer directory integration is enabled on a fresh installation. Disabling it hides only Open with Vidmetry for directories and survives application updates without changing another application's file associations. The first restored NSIS update migrates the legacy preference into package-specific state. Video Open with entries remain available until the installed package is removed.
- **FR-055** The desktop UI follows the Windows effective-pixel type ramp with 14/20 body text and 12/16 captions as its compact-text floor, 40-pixel primary control targets, and four-pixel layout increments. Its panes and responsive layout scale with a 1280×900 default and 960×720 minimum window. Common settings use a stable 920×760 frame at the default size, clamp to 24-pixel viewport gutters at the minimum size, and scroll only the detail page so category changes and expanded choices cannot resize the frame. Settings controls in the same grid row align at their top edge, and the scroll viewport reserves enough inset for every focus visual. Accent color communicates actions and selection rather than repeating a nearby heading.
- **FR-056** The preview provides configurable accelerators for Open video, Open folder, common settings, all three export profiles, both save actions, previous/next video, play/pause, one- and ten-frame seek in either direction, and fullscreen preview. The settings list orders them by expected use frequency while keeping related pairs adjacent: playback and seek, save, playlist navigation, open, profile selection, fullscreen, and common settings. Defaults remain Ctrl+O, Ctrl+Shift+O, Ctrl+Comma, Alt+1/2/3, Ctrl+S, Ctrl+Shift+S, Page Up/Down, Space, Left/Right, Shift+Left/Right, and F11. Tooltips disclose the active accelerators and update immediately after reassignment. Shortcut recording waits for physical key input, rejects Windows-key, duplicate, invalid, and reserved standard-action chords, supports Escape cancellation, and can reset all assignments.

## 4. Quality semantics

| Profile | Physical dimensions changed | Video re-encoded | Fidelity | Compatibility |
|---|---:|---:|---|---|
| Compatible MP4 | Yes | Yes | High, not mathematically lossless | High |
| Lossless FFV1/MKV | Yes | Yes | Lossless after source decode, provided pixel format and color metadata are preserved | Moderate/low |
| Metadata-only | Display-only | No | Original coded stream retained | Codec/player dependent |

The Compatible profile defaults to H.264, automatic encoder selection, quality level 17, `medium`, source pixel format, source cadence, automatic audio, and metadata retention. Automatic encoder selection prefers `nvenc`, `qsv`, and `amf` before the `libx264` or `libx265` fallback. Compatible MP4 output is always finalized with fast start. These defaults are not a guarantee of visual transparency for every source or encoder.

The Lossless profile must avoid an unconditional conversion to 8-bit `yuv420p`. The backend retains a compatible source pixel format and carries color primaries, transfer characteristics, matrix, and range where FFmpeg exposes them.

## 5. Architecture

```text
Tauri 2 desktop shell
  ├─ Svelte + TypeScript presentation layer
  │    ├─ Video viewport and crop overlay
  │    ├─ Playback/scrub and directory navigation
  │    ├─ Coalesced directory refresh and export-state coordination
  │    ├─ Persistent settings and localization
  │    ├─ Typed application-error localization
  │    └─ Direct save actions and progress
  ├─ Typed Tauri IPC
  └─ Rust application/media service
       ├─ Probe and media descriptor mapping
       ├─ Crop validation and orientation mapping
       ├─ Preview proxy lifecycle
       ├─ Selected-directory change watcher
       ├─ Export profile/argument builder
       ├─ FFmpeg/ffprobe sidecar process manager
       ├─ Windows appearance/accent adapter
       ├─ File Explorer reveal adapter
       └─ File Explorer directory-command visibility adapter
```

No localhost HTTP service or database is required. Tauri IPC carries small structured messages only; video bytes never pass through IPC.

### 5.1 Technology choices

- **Tauri 2**: small native shell, native dialogs, Rust process ownership.
- **Svelte 5 + Vite + TypeScript**: compact reactive crop UI.
- **Rust**: validation, process safety, progress parsing, filesystem operations.
- **FFmpeg/ffprobe**: broad media decoding and deterministic export.
- **Vitest + Testing Library**: frontend domain and component-interaction tests.
- **Playwright**: Chromium interaction, accessible control semantics, state transitions, and requirement-specific layout geometry.
- **Cargo test**: backend domain and argument-builder tests.

### 5.2 Repository layout

```text
vidmetry/
  docs/SDD.md
  scripts/setup-ffmpeg.ps1
  src/
    lib/
      crop.ts
      app-error.ts
      export.ts
      i18n.ts
      media.ts
      settings.ts
      appearance.ts
    App.svelte
  src-tauri/
    binaries/
    src/
      app_error.rs
      export.rs
      ffmpeg.rs
      media.rs
      appearance.rs
      directory_watch.rs
      selection.rs
      shell_integration.rs
      lib.rs
    windows/
      hooks.nsh
      msix-explorer-command/
        ExplorerCommand.cpp
        ExplorerCommand.vcxproj
  scripts/
    build-msix.ps1
    setup-ffmpeg.ps1
    test-nsis-package.ps1
    test-msix-package.ps1
    test-integration.ps1
```

## 6. Domain model

```text
MediaDescriptor
  sourcePath: string
  durationSeconds: number
  frameCount: positive integer counted by ffprobe
  codedWidth/codedHeight: integer
  displayWidth/displayHeight: integer
  rotationDegrees: integer
  videoCodec/pixelFormat: string
  frameRate: rational
  hasAudio: boolean

CropRect
  x/y/width/height: integer display-oriented source pixels

TrimRange
  startFrame: inclusive integer source-frame index
  endFrame: exclusive integer source-frame index

ExportRequest
  sourcePath/outputPath: string
  crop: CropRect
  trim: TrimRange
  settings: ExportSettings
  overwrite/inPlace: boolean

ExportSettings
  profile: compatible | lossless | metadata
  videoCodec: h264 | h265
  encoder: automatic | nvidia | intel | amd | software
  crf/preset/pixelFormat
  audioMode/audioBitrateKbps
  frameRateMode/constantFrameRate
  preserveMetadata/copySubtitles

AppSettings
  languageMode/language
  appearance: system/manual theme and accent choices
  shortcuts: sixteen canonical physical-key chords
  loopPlayback
  explorerIntegration
  export: ExportSettings
```

Crop rectangles use the displayed orientation. The backend owns the conversion to the filter coordinate system. All untrusted numbers are checked for finiteness, positivity, bounds, minimum size, and codec modulus.

## 7. IPC contract

| Command/event | Direction | Purpose |
|---|---|---|
| `inspect_selection(path)` | UI → Rust | Resolve a selected file or sorted directory playlist |
| `supported_video_extensions()` | UI → Rust | Return the single supported-extension list used by native file filters |
| `windows_ui_language()` | UI → Rust | Resolve the supported locale matching the Windows UI language for native-dialog labels |
| `pick_video_folder(title, selectFolderLabel, cancelLabel, initialDirectory?)` | UI → Rust | Host the native Explorer Shell view with supported videos visible and return the folder currently being viewed |
| `watch_directory(path?)` | UI → Rust | Replace or stop the non-recursive watcher for the active directory |
| `probe_video(path)` | UI → Rust | Return `MediaDescriptor` from ffprobe JSON |
| `create_preview(path)` | UI → Rust | Create/reuse local proxy and return its path |
| `create_timeline_strip(path, durationSeconds)` | UI → Rust | Create/reuse a 12-frame contact sheet for the trim bar |
| `start_export(request)` | UI → Rust | Validate request, start job, return job ID |
| `cancel_export(jobId)` | UI → Rust | Terminate matching child process |
| `reveal_in_explorer(path)` | UI → Rust | Open File Explorer with a completed output selected |
| `startup_selection()` | UI → Rust | Read an existing file or directory passed by a Shell launch |
| `set_explorer_integration(enabled)` | UI → Rust | Show or hide the packaged directory context command |
| `system_accent_color()` | UI → Rust | Return the current Windows DWM accent as a CSS RGB color |
| `export-progress` | Rust → UI | `{jobId, fraction, outTimeSeconds}` |
| `export-complete` | Rust → UI | Final output path |
| `directory-changed` | Rust → UI | Active directory root whose contents changed |
| command errors | Rust → UI | `{code, detail?}`; code is stable and language-neutral |
| `export-error` | Rust → UI | `{jobId, error: {code, detail?}, cancelled}` |

Only the Rust layer constructs FFmpeg argument arrays. Paths and crop values are never interpolated into a shell command string.

## 8. User interface

The main window uses three regions:

1. A narrow header with the Vidmetry icon/wordmark, source details in the editor, icon-only Open/Folder/Settings actions, and one context-sensitive save control.
2. A flexible, neutrally colored video stage containing optional directory navigation, the video, and crop overlay.
3. A collapsible frame-strip footer with a playback-position handle, start/end trim-boundary handles, persistent loop, and mute, plus a collapsible spatial inspector with coordinates, dimensions, aspect ratio, and Reset.

The first-run state is an accessible file/folder drop target. Common settings use one flat left-side category navigation with one detail page at a time rather than nested or cross-cutting tabs. Export settings do not interrupt each save, and all setting changes save immediately. CSS type-ramp and control-size tokens keep English and Japanese text, buttons, fields, pane controls, and status surfaces in one Windows-scaled hierarchy; the settings title is not repeated as decorative accent text. A strict Zod schema is the runtime and compile-time source for `AppSettings`; unknown, obsolete, partial, or out-of-range shapes are rejected. Valid settings are persisted in `settings.json` under Tauri's application-data directory by the official Store plugin. Windows or manual mode/accent choices are projected through CSS variables; fixed app-icon artwork is strictly achromatic. MSIX registers supported-video Open with activation and a packaged COM directory command; NSIS creates equivalent current-user video and directory registrations. The application stores and applies only the directory command's visibility state, while video Open with registration remains installed. Shell-registration changes refresh the Windows Shell cache. Keyboard focus indicators are visible, icon-only actions have accessible labels, accelerators appear in tooltips, and errors appear inline.

## 9. Preview strategy

The application first reads duration, frame rate, and a declared exact frame count with ffprobe. Only when the container does not report a count does it run ffprobe's frame-counting pass. These values are required for the frame-index contract; the application reports a typed error instead of estimating a missing value. It then exposes the selected file through Tauri's scoped asset protocol and asks the native WebView media element to play it. If decoding fails, `create_preview` produces an orientation-normalized, square-pixel, maximum-1280-pixel H.264 MP4 proxy with frequent keyframes. The proxy is for interaction only; final export always reads the original.

Proxy and timeline contact-sheet entries are stored under the operating-system cache directory and keyed by canonical path, file size, and last-write timestamp. Entries are reusable across sessions. Each cache has count, total-size, and age limits; least-recently-used entries are removed after creation or reuse. FFmpeg writes to unique staging paths, and only non-empty completed files are promoted to reusable entries. Successful in-place replacement explicitly invalidates the source probe entry; every media load uses a new asset-URL revision so the WebView cannot reuse bytes from an older generation at the same path.

## 10. Error handling and recovery

- Product-owned prose is not transported across IPC. The Rust layer returns generated `AppError` codes and optional OS/FFmpeg diagnostics. The i18next resource layer owns the Japanese and English messages and interpolates diagnostics only after selecting the active language.
- Invalid or missing input: return a typed probe error without changing current UI state.
- Unsupported direct preview: offer/perform proxy creation.
- FFmpeg missing: display setup guidance and disable export.
- Existing copy destination: native dialog confirmation and backend overwrite permission are both required.
- In-place save: explicit application confirmation is required; encoding completes to a sibling temporary path before replacement.
- Directory watcher unavailable: retain the opened video and report a typed localized error instead of silently presenting a stale live view.
- Disk full or permission failure: retain the original and remove only the known partial file.
- App closes during export: terminate owned child processes.
- Sidecar output is logged without exposing unrelated environment variables.
- Recoverable frontend fallbacks and backend cleanup failures are written through Tauri's official Log plugin to the operating-system application log directory.

## 11. Security, privacy, and distribution

- No network requests occur during normal application use.
- The source is opened read-only during probe, preview, and encoding. Only confirmed in-place Save can replace it after successful encoding.
- File access is limited to user-selected paths and the app cache.
- FFmpeg is invoked as an allowlisted sidecar with structured arguments.
- The FFmpeg build identifier, immutable dated GitHub Release URL, archive and executable hashes, GPL license hash, full source commits, required configuration flags and encoders, corresponding-source asset tag, and corresponding-source archive hash are pinned in `scripts/ffmpeg-sidecars.json`. Existing files and fresh downloads are both verified.
- Vidmetry does not link FFmpeg libraries or exchange internal data structures. The independent executables are invoked through ordinary command-line arguments, files, and progress text, and every Windows package carries the GPL text, exact build report, and release-specific corresponding-source URL.
- FFmpeg binaries are not committed to Git. When source-determining inputs change on `main`, a read-only job with no persisted checkout credentials assembles and audits the exact FFmpeg source, pinned public build definition and patches, and every dependency source archive selected by the resolved Windows GPL build graph. The digest-pinned source toolchain image is assigned to the upstream acquisition command before it can resolve a moving image tag; the same image repacks dependency trees and the outer archive without network access. Checkout-specific VCS administration data is removed, external links are recorded and removed, special filesystem entries are rejected, and ordering, ownership, timestamps, and portable executable/non-executable permissions are canonicalized. A separate publication job receives only the verified archive and checksum, publishes them once under the pinned engine-specific tag, and never overwrites that asset.
- Node.js, npm, and Rust are version-constrained. GitHub Actions are pinned to full commit SHAs; npm audit, JavaScript and Rust license allowlists, RustSec, and distribution-contract tests run in CI, and Dependabot proposes dependency and Action updates. Locked Rust and production JavaScript graphs generate package-level license reports for every Windows package. MPL-2.0 dependencies are additionally matched exactly to a source manifest, and their verified Source Form archives and license are bundled.
- A pushed `vX.Y.Z` tag must match every application version file. After a fail-closed public-distribution preflight, the release workflow verifies the pinned immutable corresponding-source asset and builds one application binary. It first packages and tests the unpatched binary as one x64 MSIX with the Partner Center identity, then applies Tauri's NSIS bundle marker and packages and live-tests one x64 NSIS; it never regenerates FFmpeg source for an unchanged engine. A final publisher creates or resumes a draft Release and publishes both packages only after the complete corresponding-source archive and its SHA-256 are attached and all four assets are re-read from GitHub. Repository visibility is checked again immediately before publication. The same unsigned MSIX is submitted to Partner Center and receives its production signature from Microsoft Store; the unsigned NSIS remains a manually updated direct-download option. Hyphenated versions are marked as prereleases.

## 12. Performance requirements

- Main-window interactions should remain under 16 ms per pointer update on typical 1080p media.
- Crop movement uses CSS geometry; it does not run FFmpeg per pointer event.
- Scrub seeks are throttled to at most 30 requests per second.
- Probe and process work never block the UI thread.
- Export reuses the current unchanged source probe instead of starting another ffprobe process.
- Eligible late trims start decoding five seconds before the selected range; uncertain timing retains the exact full-decode path.
- FFV1 worker and slice counts scale automatically with available CPU parallelism and output resolution.
- Export remains cancellable.

## 13. Test strategy

### 13.1 Frontend unit tests

- Screen-to-source coordinate mapping.
- Move and eight spatial-crop handles at every boundary.
- Minimum size, clamping, modulus snapping, and aspect locks.
- Time formatting and seek clamping.
- Export-profile eligibility.
- Strict settings-schema rejection, ordered immediate persistence, Explorer-integration toggling, system/manual language and appearance resolution, shortcut capture/conflict detection, output-extension and in-place eligibility.

### 13.2 Component and UI regression tests

- Testing Library covers launcher content, immediate settings persistence, category navigation, appearance choices, shortcut capture/conflicts, tooltip updates and dispatch, native folder-dialog invocation, save shortcuts, Space playback from the focused playback scrubber, directory playback carry and live refresh, same-path overwrite reload, immutable trim export requests, structured synchronous/asynchronous error localization, pane collapse, F11 state, Windows appearance projection, notice dismissal, and completed-output links. A source-boundary regression test rejects product-owned Japanese text outside the locale resource or inside the Rust backend.
- Playwright exercises the same critical flows in Chromium, including fixed-size settings category navigation and encoder availability, immediate appearance/shortcut persistence, live accelerator tooltips and dispatch, the native folder-dialog contract, English/Japanese settings layout across every category, a structured backend error in the selected UI language, regular/fullscreen directory playback carry, Explorer and copy-save directory additions, clicking the rendered playback-position handle before Space, real playback-state changes, full-duration click alignment after halving the selection, locked trim export ranges, computed theme/accent projection, requirement-specific alignment, collapsible panes, F11, and notification expiry.
- Asset verification scans generated PNG/ICO pixels and source SVG colors for chromatic fixed artwork and rejects legacy green tints.
- MSIX verification unpacks the package and checks its identity, classic-app activation, x64 payloads, all supported video associations, packaged COM directory command, locked sidecars, licenses, and absence of build-only files. An elevated live mode additionally signs, installs, activates COM and the app, and uninstalls an isolated development identity. NSIS verification performs an isolated current-user installation, checks its payload and equivalent video/directory menu registrations, preserves a disabled directory command across an update, launches the app, and verifies uninstall cleanup.
- CI retains screenshots only as failure diagnostics. UI regressions are asserted through roles, accessible names, values, enabled states, computed styles, and geometry tied to explicit requirements rather than whole-screen pixel baselines.

### 13.3 Rust unit tests

- Crop validation and full-frame/no-op detection.
- Rotation-aware crop mapping for 0, 90, 180, and 270 degrees.
- FFmpeg argument generation for all three profiles.
- H.264/HEVC metadata-only eligibility.
- Progress-line parsing.
- Temporary output naming and destination validation.
- Directory filtering, non-recursive discovery, and stable sorting.
- Directory watcher event filtering.
- Hardware availability probes, automatic/manual encoder ordering, backend-specific quality arguments, audio/frame-rate argument generation, and invalid-setting rejection.
- Exact `start_frame`/`end_frame` FFmpeg filters, CFR late-seek adjustment, VFR full-decode fallback, audio alignment, and metadata-only rejection.
- Media-probe cache invalidation and FFV1 thread/slice scaling.
- Windows extended-path normalization and Explorer selection arguments.
- DWM ARGB-to-CSS accent conversion.
- Shell startup-path handling and packaged-identity detection.
- Cache staging, non-empty promotion, and count/size/age pruning.

### 13.4 Integration tests

Generated fixtures exercise H.264 compatible output, configured HEVC 10-bit output, slice-parallel FFV1, metadata-only crop, an early 60-frame trim, a late 60-frame input-seek trim, and staged in-place replacement. Tests ffprobe dimensions/codecs/pixel formats/frame count and color characteristics, compare lossless pixels plus late-trim frame/audio hashes, and verify the original fixture hashes remain unchanged.

### 13.5 Manual acceptance matrix

- 1080p H.264 MP4
- 4K HEVC 10-bit
- rotated phone MOV
- variable-frame-rate video
- MKV with multiple audio streams
- corrupt/unsupported file
- Unicode and spaces in paths
- mixed-format directory navigation and drag/drop
- Japanese/English switching and settings restart persistence
- loop persistence across video changes
- Windows light/dark and several light/dark accent colors, including runtime changes
- immediate settings writes, custom shortcut recording, conflicts, reset, and restart persistence
- output cancellation and disk/permission errors

## 14. Acceptance criteria for 0.4.8

- **AC-001** A user can open a video, drag every spatial-crop handle, scrub, play, and reset without leaving the main window.
- **AC-002** Pixel readouts match the crop shown and remain in bounds after resize.
- **AC-003** Compatible and FFV1 exports have the selected physical frame dimensions according to ffprobe.
- **AC-004** Metadata-only export never uses a video encoder and is disabled for non-H.264/HEVC input.
- **AC-005** Cancelling export leaves no final output and the UI can immediately start another export.
- **AC-006** Original fixtures are byte-identical before and after automated exports; in-place behavior uses a disposable copied fixture.
- **AC-007** Frontend tests, Rust tests, type checks, lint checks, and production builds pass.
- **AC-008** A selected/dropped directory can be navigated from the GUI and Page Up/Page Down.
- **AC-009** Common settings survive reload and affect the next direct save without an intervening export screen.
- **AC-010** Loop state follows video changes, and the normal UI can switch between Japanese and English.
- **AC-011** In-place Save is unavailable for mismatched extensions and safely replaces the source only after confirmation and successful encoding.
- **AC-012** Save controls, folder arrows, and success notification pass component and Chromium layout/visual regression tests; the notification reveals rather than launches the output.
- **AC-013** Start/end trim-boundary handles provide velocity-sensitive mouse adjustment and keyboard frame steps, and the exported video contains the selected ordinal frames.
- **AC-014** The save notice expires after three seconds or another interaction; its link opens Explorer with the saved file selected.
- **AC-015** Windows light/dark app mode and accent color drive all app surfaces and the trim bar, and fixed icon assets are achromatic.
- **AC-016** Copy and save and confirmed in-place Save use configurable accelerators with Ctrl+S and Ctrl+Shift+S defaults.
- **AC-017** Start/end trim-boundary handles move by 1 or 10 frames from keyboard focus. Space from the focused playback-position handle changes the media element to a real playing state and changes the UI to Pause. Playback-scrubber clicks and off-center trim-boundary drags remain aligned after the selected range is reduced to half the video.
- **AC-018** The crop inspector and time-trim footer collapse independently, and F11/Escape toggle a video-only window fullscreen preview.
- **AC-019** Product-owned runtime errors are transported as language-neutral codes with optional diagnostics and render from the same Japanese/English locale resources as the rest of the UI; backend and component source contain no Japanese product prose.
- **AC-020** A fresh MSIX or NSIS installation exposes every supported video and selected directories to Vidmetry from File Explorer; the common setting hides and restores only the directory command, an update preserves that state, and uninstall removes package-owned entries.
- **AC-021** Directory navigation carries active playback into the destination video in regular and fullscreen preview, and the active playlist reflects Explorer and completed copy-save additions without reopening the folder.
- **AC-022** In-place Save reloads dimensions, duration, frame count, and preview bytes from the replaced source generation rather than mixing cached generations.
- **AC-023** Export uses the crop, trim range, and settings visible when Save begins; editor controls remain locked until the request finishes or fails.
- **AC-024** Computed Chromium geometry verifies the Windows type ramp, 40-pixel controls, proportionally enlarged editor panes, default-size viewport, minimum-size reflow, and a single non-duplicated settings heading.
- **AC-025** A settings change takes effect and is durable without Apply; one-level category navigation exposes export, playback, appearance, shortcuts, Explorer, and language without clipped focus visuals, misaligned paired controls, or unintended wrapped text at the minimum window size.
- **AC-026** Windows/manual mode and accent choices update immediately, while all sixteen default and recorded preview accelerators dispatch their corresponding app commands; duplicate and reserved assignments are rejected.

## 15. Verification status

The 0.4.8 implementation satisfies AC-001 through AC-026 at automated or implementation-inspection level. Native picker interaction, live Windows personalization, Store-signed MSIX installation and Shell changes in the packaged WebView, and the wider codec/device matrix remain manual acceptance items. Exact commands, fixture results, tool versions, and produced package hashes are recorded in `docs/VERIFICATION.md`.
