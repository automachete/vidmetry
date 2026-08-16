# Vidmetry Software Design Description

| Field | Value |
|---|---|
| Document version | 1.4 |
| Product version | 0.4.1 |
| Status | Implemented and verified |
| Primary platform | Windows 11 x64 |
| UI languages | Japanese and English |
| Last updated | 2026-08-16 |

## 1. Purpose

Vidmetry is a lightweight desktop GUI for cropping a video's spatial frame and trimming its start/end time. Its interaction model follows the crop and frame-viewer tools in photo applications: open a video or folder, adjust the visible frame and selected duration, preview, and save with minimal intermediate UI.

The application must not imply that physical video cropping can normally use stream copy. It presents three distinct export behaviors so the user can choose compatibility, decoded-pixel fidelity, or metadata-only stream preservation.

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
- Remember language and loop-playback preferences between videos and sessions.
- Follow the Windows app mode and accent color without requiring a Vidmetry theme setting.
- Keep all media processing local.
- Remain responsive while probing, proxying, and exporting.

### 2.2 Non-goals for 0.4.1

- Multi-clip timelines, internal cuts, transitions, filters, captions, or independent audio editing.
- Animated crop/keyframes.
- Cloud storage, accounts, telemetry, or automatic uploads.
- Batch-applying one crop rectangle to every file in a directory.
- Mobile builds.
- A promise that metadata-only crop works in every player.

## 3. Functional requirements

### 3.1 Import and inspection

- **FR-001** The user can select or drop one local video or a directory. Directory discovery is non-recursive and includes supported video extensions only.
- **FR-002** The backend probes the primary video stream and reports container, video/audio codecs, coded dimensions, display rotation, sample aspect ratio, pixel format, bit depth, frame-rate rationals, duration, and color metadata when present.
- **FR-003** Direct WebView playback is attempted first. On playback failure the user can generate, or the app can automatically generate, a temporary H.264 proxy from the original.
- **FR-004** A newly opened video starts with the crop rectangle covering the complete displayed frame.
- **FR-005** When a directory is active, the user can switch videos with an in-app list, previous/next controls, or Page Up/Page Down. File names are sorted case-insensitively.

### 3.2 Crop interaction

- **FR-010** The crop rectangle has four edge handles, four corner handles, and a draggable interior.
- **FR-011** The area outside the crop rectangle is visibly dimmed.
- **FR-012** The rectangle cannot leave the visible video frame and cannot become smaller than 16 by 16 source pixels.
- **FR-013** Coordinates are stored in display-oriented source pixels as `{x, y, width, height}` and normalized values are derived only for responsive rendering.
- **FR-014** For chroma-subsampled output, coordinates are snapped to the required pixel modulus; the initial supported export modulus is 2.
- **FR-015** The user can reset to full frame and choose free, source, 1:1, 4:3, 16:9, or 9:16 aspect constraints.
- **FR-016** Pixel fields accept keyboard input and update the rectangle after validation.

### 3.3 Playback and scrub

- **FR-020** The UI provides play/pause, current time, duration, mute, a frame strip, a playhead scrubber, and start/end trim handles.
- **FR-021** Start is inclusive and end is exclusive. The selected range always contains at least one integer source frame.
- **FR-022** The crop overlay remains spatially stable during playback, seek, resize, and fullscreen layout changes.
- **FR-023** Trim dragging maps the dispatched pointer event's final absolute timeline position to the handle, with one-frame snapping at low speed and timeline-scale snapping at high speed. Coalesced samples may estimate velocity but cannot replace the final position. This prevents accumulated or one-event pointer/handle drift. Focused trim handles move by one frame with an arrow key or ten with Shift.
- **FR-024** An icon-only control toggles loop playback. The selected state is persisted and reused for subsequent videos and sessions.
- **FR-025** Playback and loop boundaries follow the selected time range. Space toggles playback even while a trim handle has focus; otherwise left/right keys seek by one frame or ten with Shift.
- **FR-026** The UI follows the current Windows light/dark app mode and selected accent color. Theme change notifications apply the mode immediately, and the accent is refreshed at startup, on theme change, and when the app regains focus.
- **FR-027** The trim selection border and handles use the Windows accent color with a computed readable foreground.
- **FR-028** The spatial crop inspector and time-trim footer can each be collapsed and restored with accessible icon-only controls.
- **FR-029** F11 enters or exits a video-only window fullscreen preview. Escape exits fullscreen.

### 3.4 Export

- **FR-030** When output and source extensions differ, the header shows a direct Copy and save action with no menu. When they match, one combined Save options control opens Copy and save or confirmed in-place Save; there is no separate export-configuration screen.
- **FR-031** Compatible mode outputs H.264 or H.265 MP4 and allows CRF, encoder preset, pixel format, audio handling/bitrate, frame-rate handling, metadata retention, and fast-start configuration.
- **FR-032** Lossless mode outputs FFV1 in Matroska. The video is decoded, cropped, and encoded losslessly while compatible audio/subtitle streams are copied.
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
- **FR-043** Compatible and lossless exports apply the selected ordinal frame range in FFmpeg. Packet-copied audio is encoded when necessary to align it with an exact time trim.
- **FR-044** Metadata-only stream copy cannot promise arbitrary frame boundaries and is disabled while a time trim is active.
- **FR-045** A successful-save notice closes after three seconds or when another UI control is used.
- **FR-046** Ctrl+S invokes Copy and save. Ctrl+Shift+S invokes confirmed in-place Save only when the current profile supports the source extension.

### 3.5 Common settings and localization

- **FR-050** An icon-only common-settings button is always available in the header.
- **FR-051** Settings persist locally and cover save profile, applicable encoder options, audio, frame rate, file metadata, and loop playback.
- **FR-052** Language mode is an exclusive choice between Windows language and manual selection. Manual selection supports Japanese and English; unsupported Windows languages fall back to English.
- **FR-053** Runtime validation messages follow the active UI language, and language controls are the final common-settings section.

## 4. Quality semantics

| Profile | Physical dimensions changed | Video re-encoded | Fidelity | Compatibility |
|---|---:|---:|---|---|
| Compatible MP4 | Yes | Yes | High, not mathematically lossless | High |
| Lossless FFV1/MKV | Yes | Yes | Lossless after source decode, provided pixel format and color metadata are preserved | Moderate/low |
| Metadata-only | Display-only | No | Original coded stream retained | Codec/player dependent |

The Compatible profile defaults to `libx264`, CRF 17, `medium`, `yuv420p`, source cadence, automatic audio, metadata retention, and fast start. This is a product default, not a guarantee of visual transparency for every source.

The Lossless profile must avoid an unconditional conversion to 8-bit `yuv420p`. The backend retains a compatible source pixel format and carries color primaries, transfer characteristics, matrix, and range where FFmpeg exposes them.

## 5. Architecture

```text
Tauri 2 desktop shell
  ├─ Svelte + TypeScript presentation layer
  │    ├─ Video viewport and crop overlay
  │    ├─ Playback/scrub and directory navigation
  │    ├─ Persistent settings and localization
  │    └─ Direct save actions and progress
  ├─ Typed Tauri IPC
  └─ Rust application/media service
       ├─ Probe and media descriptor mapping
       ├─ Crop validation and orientation mapping
       ├─ Preview proxy lifecycle
       ├─ Export profile/argument builder
       ├─ FFmpeg/ffprobe sidecar process manager
       ├─ Windows appearance/accent adapter
       └─ File Explorer reveal adapter
```

No localhost HTTP service or database is required. Tauri IPC carries small structured messages only; video bytes never pass through IPC.

### 5.1 Technology choices

- **Tauri 2**: small native shell, native dialogs, Rust process ownership.
- **Svelte 5 + Vite + TypeScript**: compact reactive crop UI.
- **Rust**: validation, process safety, progress parsing, filesystem operations.
- **FFmpeg/ffprobe**: broad media decoding and deterministic export.
- **Vitest + Testing Library**: frontend domain and component-interaction tests.
- **Playwright**: Chromium interaction, layout geometry, and screenshot-regression tests.
- **Cargo test**: backend domain and argument-builder tests.

### 5.2 Repository layout

```text
vidmetry/
  docs/SDD.md
  scripts/setup-ffmpeg.ps1
  src/
    lib/
      crop.ts
      export.ts
      i18n.ts
      media.ts
      settings.ts
      appearance.ts
    App.svelte
  src-tauri/
    binaries/
    src/
      export.rs
      ffmpeg.rs
      media.rs
      appearance.rs
      selection.rs
      lib.rs
  scripts/
    setup-ffmpeg.ps1
    test-integration.ps1
```

## 6. Domain model

```text
MediaDescriptor
  sourcePath: string
  durationSeconds: number
  frameCount: optional integer
  codedWidth/codedHeight: integer
  displayWidth/displayHeight: integer
  rotationDegrees: integer
  sampleAspectRatio: rational
  videoCodec/pixelFormat: string
  frameRate: rational
  hasAudio: boolean
  color: optional ColorDescriptor

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
  videoCodec/crf/preset/pixelFormat
  audioMode/audioBitrateKbps
  frameRateMode/constantFrameRate
  fastStart/preserveMetadata/copySubtitles

AppSettings
  languageMode/language
  loopPlayback
  export: ExportSettings
```

Crop rectangles use the displayed orientation. The backend owns the conversion to the filter coordinate system. All untrusted numbers are checked for finiteness, positivity, bounds, minimum size, and codec modulus.

## 7. IPC contract

| Command/event | Direction | Purpose |
|---|---|---|
| `inspect_selection(path)` | UI → Rust | Resolve a selected file or sorted directory playlist |
| `probe_video(path)` | UI → Rust | Return `MediaDescriptor` from ffprobe JSON |
| `create_preview(path)` | UI → Rust | Create/reuse local proxy and return its path |
| `create_timeline_strip(path, durationSeconds)` | UI → Rust | Create/reuse a 12-frame contact sheet for the trim bar |
| `start_export(request)` | UI → Rust | Validate request, start job, return job ID |
| `cancel_export(jobId)` | UI → Rust | Terminate matching child process |
| `reveal_in_explorer(path)` | UI → Rust | Open File Explorer with a completed output selected |
| `system_accent_color()` | UI → Rust | Return the current Windows DWM accent as a CSS RGB color |
| `export-progress` | Rust → UI | `{jobId, fraction, outTimeSeconds}` |
| `export-complete` | Rust → UI | Final output path |
| `export-error` | Rust → UI | User-safe message and diagnostic code |

Only the Rust layer constructs FFmpeg argument arrays. Paths and crop values are never interpolated into a shell command string.

## 8. User interface

The main window uses three regions:

1. A narrow header with the Vidmetry icon/wordmark, source details in the editor, icon-only Open/Folder/Settings actions, and one context-sensitive save control.
2. A flexible, neutrally colored video stage containing optional directory navigation, the video, and crop overlay.
3. A collapsible frame-strip footer with velocity-sensitive time handles, playback/scrub, persistent loop, and mute, plus a collapsible spatial inspector with coordinates, dimensions, aspect ratio, and Reset.

The first-run state is an accessible file/folder drop target. Export settings are edited only in the common-settings dialog and do not interrupt each save. Windows mode/accent changes are projected through CSS variables; fixed app-icon artwork is achromatic. Keyboard focus indicators are visible, icon-only actions have accessible labels, and errors appear inline.

## 9. Preview strategy

The application first exposes the selected file through Tauri's scoped asset protocol and asks the native WebView media element to play it. If decoding fails, `create_preview` produces an orientation-normalized, square-pixel, maximum-1280-pixel H.264 MP4 proxy with frequent keyframes. The proxy is for interaction only; final export always reads the original.

Proxy and timeline contact-sheet entries are stored under the operating-system cache directory and keyed by canonical path, file size, and last-write timestamp. Entries are reusable across sessions; bounded LRU cleanup is deferred.

## 10. Error handling and recovery

- Invalid or missing input: return a typed probe error without changing current UI state.
- Unsupported direct preview: offer/perform proxy creation.
- FFmpeg missing: display setup guidance and disable export.
- Existing copy destination: native dialog confirmation and backend overwrite permission are both required.
- In-place save: explicit application confirmation is required; encoding completes to a sibling temporary path before replacement.
- Disk full or permission failure: retain the original and remove only the known partial file.
- App closes during export: terminate owned child processes.
- Sidecar output is logged without exposing unrelated environment variables.

## 11. Security, privacy, and distribution

- No network requests occur during normal application use.
- The source is opened read-only during probe, preview, and encoding. Only confirmed in-place Save can replace it after successful encoding.
- File access is limited to user-selected paths and the app cache.
- FFmpeg is invoked as an allowlisted sidecar with structured arguments.
- Sidecar archives are verified against the publisher-provided SHA-256 checksum by the setup script.
- FFmpeg binaries are not committed to Git. Distribution must include the applicable FFmpeg license, build configuration, copyright notices, and corresponding-source obligations for the selected build.
- A pushed `vX.Y.Z` tag must match every application version file. The release workflow reruns verification, builds on Windows, creates a GitHub Release with generated notes, and attaches MSI and NSIS installers. Hyphenated versions are marked as prereleases.

## 12. Performance requirements

- Main-window interactions should remain under 16 ms per pointer update on typical 1080p media.
- Crop movement uses CSS geometry; it does not run FFmpeg per pointer event.
- Scrub seeks are throttled to at most 30 requests per second.
- Probe and process work never block the UI thread.
- Export can use all FFmpeg-managed worker threads but remains cancellable.

## 13. Test strategy

### 13.1 Frontend unit tests

- Screen-to-source coordinate mapping.
- Move and eight resize handles at every boundary.
- Minimum size, clamping, modulus snapping, and aspect locks.
- Time formatting and seek clamping.
- Export-profile eligibility.
- Settings normalization, persistence, OS/manual language resolution, output-extension and in-place eligibility.

### 13.2 Component and UI regression tests

- Testing Library covers launcher content, settings, save shortcuts, Space playback from trim focus, localization, trim-frame key steps, pane collapse, F11 state, Windows appearance projection, notice dismissal, and completed-output links.
- Playwright exercises the same critical flows in Chromium, including absolute trim-handle alignment, trim export ranges, theme/accent projection, collapsible panes, F11, notification expiry, and screenshot layout.
- Screenshot baselines cover launcher, settings, save-menu, successful-save, and Windows light-theme states.

### 13.3 Rust unit tests

- Crop validation and full-frame/no-op detection.
- Rotation-aware crop mapping for 0, 90, 180, and 270 degrees.
- FFmpeg argument generation for all three profiles.
- H.264/HEVC metadata-only eligibility.
- Progress-line parsing.
- Temporary output naming and destination validation.
- Directory filtering, non-recursive discovery, and stable sorting.
- Detailed encoder/audio/frame-rate argument generation and invalid-setting rejection.
- Exact `start_frame`/`end_frame` FFmpeg filters, audio alignment, and metadata-only rejection.
- Windows extended-path normalization and Explorer selection arguments.
- DWM ARGB-to-CSS accent conversion.

### 13.4 Integration tests

Generated fixtures exercise H.264 compatible output, configured HEVC 10-bit output, FFV1, metadata-only crop, a 60-frame time trim, and staged in-place replacement. Tests ffprobe dimensions/codecs/pixel formats/frame count, compare lossless frame hashes, and verify the original fixture hash remains unchanged.

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
- output cancellation and disk/permission errors

## 14. Acceptance criteria for 0.4.1

- **AC-001** A user can open a video, drag every crop handle, scrub, play, and reset without leaving the main window.
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
- **AC-013** Start/end handles provide velocity-sensitive mouse adjustment and keyboard frame steps, and the exported video contains the selected ordinal frames.
- **AC-014** The save notice expires after three seconds or another interaction; its link opens Explorer with the saved file selected.
- **AC-015** Windows light/dark app mode and accent color drive all app surfaces and the trim bar; fixed icon assets contain no brand accent color.
- **AC-016** Ctrl+S starts Copy and save, while Ctrl+Shift+S starts confirmed in-place Save when eligible.
- **AC-017** Start/end handles move by 1 or 10 frames from keyboard focus, Space still toggles playback, and a narrowed-range pointer drag keeps its handle aligned with the pointer.
- **AC-018** The crop inspector and time-trim footer collapse independently, and F11/Escape toggle a video-only window fullscreen preview.

## 15. Verification status

The 0.4.1 implementation satisfies AC-001 through AC-018 at automated or implementation-inspection level. Native picker interaction, live Windows personalization changes in the packaged WebView, and the wider codec/device matrix remain manual acceptance items. Exact commands, fixture results, tool versions, and produced installer hashes are recorded in `docs/VERIFICATION.md`.
