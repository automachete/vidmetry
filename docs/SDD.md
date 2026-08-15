# Vidmetry Software Design Description

| Field | Value |
|---|---|
| Document version | 1.0 |
| Product version | 0.1.0 |
| Status | Implemented and verified baseline |
| Primary platform | Windows 11 x64 |
| UI languages | Japanese, English-ready architecture |
| Last updated | 2026-08-15 |

## 1. Purpose

Vidmetry is a lightweight, single-purpose desktop GUI for cropping the spatial frame of a video. Its interaction model is the crop tool in a photo application: open one video, drag a rectangle, scrub or play to inspect the whole clip, and save a new file.

The application must not imply that physical video cropping can normally use stream copy. It presents three distinct export behaviors so the user can choose compatibility, decoded-pixel fidelity, or metadata-only stream preservation.

## 2. Goals and non-goals

### 2.1 Goals

- Open a local video without modifying it.
- Display a draggable and resizable crop rectangle over a playable preview.
- Seek with a scrubber and play or pause while the crop overlay remains active.
- Show the final integer crop coordinates and output frame dimensions.
- Support arbitrary FFmpeg-decodable input by creating a temporary preview proxy when direct playback fails.
- Export with visible progress and cancellation.
- Offer compatible MP4, mathematically lossless FFV1, and metadata-only lossless modes.
- Keep all media processing local.
- Remain responsive while probing, proxying, and exporting.

### 2.2 Non-goals for 0.1.0

- Temporal trimming, multi-clip timelines, transitions, filters, captions, or audio editing.
- Animated crop/keyframes.
- Cloud storage, accounts, telemetry, or automatic uploads.
- In-place source modification.
- Mobile builds.
- A promise that metadata-only crop works in every player.

## 3. Functional requirements

### 3.1 Import and inspection

- **FR-001** The user can select one local video with the native file picker.
- **FR-002** The backend probes the primary video stream and reports container, video/audio codecs, coded dimensions, display rotation, sample aspect ratio, pixel format, bit depth, frame-rate rationals, duration, and color metadata when present.
- **FR-003** Direct WebView playback is attempted first. On playback failure the user can generate, or the app can automatically generate, a temporary H.264 proxy from the original.
- **FR-004** A newly opened video starts with the crop rectangle covering the complete displayed frame.

### 3.2 Crop interaction

- **FR-010** The crop rectangle has four edge handles, four corner handles, and a draggable interior.
- **FR-011** The area outside the crop rectangle is visibly dimmed.
- **FR-012** The rectangle cannot leave the visible video frame and cannot become smaller than 16 by 16 source pixels.
- **FR-013** Coordinates are stored in display-oriented source pixels as `{x, y, width, height}` and normalized values are derived only for responsive rendering.
- **FR-014** For chroma-subsampled output, coordinates are snapped to the required pixel modulus; the initial supported export modulus is 2.
- **FR-015** The user can reset to full frame and choose free, source, 1:1, 4:3, 16:9, or 9:16 aspect constraints.
- **FR-016** Pixel fields accept keyboard input and update the rectangle after validation.

### 3.3 Playback and scrub

- **FR-020** The UI provides play/pause, current time, duration, mute, and a full-duration range scrubber.
- **FR-021** Dragging the scrubber seeks the preview and is throttled to protect the UI thread.
- **FR-022** The crop overlay remains spatially stable during playback, seek, resize, and fullscreen layout changes.
- **FR-023** Left and right keyboard arrows seek by one second; Shift plus arrow seeks by ten seconds; Space toggles playback.

### 3.4 Export

- **FR-030** Export always writes a new file selected through a native save dialog.
- **FR-031** Compatible mode outputs H.264/AAC MP4, preserves source timestamps/frame cadence when possible, uses a constant-quality software encode, and never upscales.
- **FR-032** Lossless mode outputs FFV1 in Matroska. The video is decoded, cropped, and encoded losslessly while compatible audio/subtitle streams are copied.
- **FR-033** Metadata-only mode is available only for H.264 or HEVC. It copies encoded media while setting codec crop metadata. The UI warns that coded pixels and file size remain, and that some players ignore the crop.
- **FR-034** A physical crop filter is never combined with video stream copy.
- **FR-035** Rotation is handled explicitly so the exported rectangle matches the displayed orientation. Output rotation metadata is normalized.
- **FR-036** Audio is stream-copied when the output container supports it; compatible MP4 mode may fall back to AAC.
- **FR-037** FFmpeg progress is emitted to the UI at least twice per second when available.
- **FR-038** The user can cancel an export. A partial temporary output is not promoted to the selected final path.
- **FR-039** Successful export is written to a temporary sibling path and atomically renamed where the filesystem permits.

## 4. Quality semantics

| Profile | Physical dimensions changed | Video re-encoded | Fidelity | Compatibility |
|---|---:|---:|---|---|
| Compatible MP4 | Yes | Yes | High, not mathematically lossless | High |
| Lossless FFV1/MKV | Yes | Yes | Lossless after source decode, provided pixel format and color metadata are preserved | Moderate/low |
| Metadata-only | Display-only | No | Original coded stream retained | Codec/player dependent |

The Compatible profile defaults to `libx264`, CRF 17, `medium` preset. This is a product default, not a guarantee of visual transparency for every source.

The Lossless profile must avoid an unconditional conversion to 8-bit `yuv420p`. The backend retains a compatible source pixel format and carries color primaries, transfer characteristics, matrix, and range where FFmpeg exposes them.

## 5. Architecture

```text
Tauri 2 desktop shell
  ├─ Svelte + TypeScript presentation layer
  │    ├─ Video viewport and crop overlay
  │    ├─ Playback/scrub controls
  │    └─ Export dialog and progress
  ├─ Typed Tauri IPC
  └─ Rust application/media service
       ├─ Probe and media descriptor mapping
       ├─ Crop validation and orientation mapping
       ├─ Preview proxy lifecycle
       ├─ Export profile/argument builder
       └─ FFmpeg/ffprobe sidecar process manager
```

No localhost HTTP service or database is required. Tauri IPC carries small structured messages only; video bytes never pass through IPC.

### 5.1 Technology choices

- **Tauri 2**: small native shell, native dialogs, Rust process ownership.
- **Svelte 5 + Vite + TypeScript**: compact reactive crop UI.
- **Rust**: validation, process safety, progress parsing, filesystem operations.
- **FFmpeg/ffprobe**: broad media decoding and deterministic export.
- **Vitest**: frontend geometry/state tests.
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
      media.ts
    App.svelte
  src-tauri/
    binaries/
    src/
      export.rs
      ffmpeg.rs
      media.rs
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

ExportRequest
  sourcePath/outputPath: string
  crop: CropRect
  profile: compatible | lossless | metadata
  overwrite: false
```

Crop rectangles use the displayed orientation. The backend owns the conversion to the filter coordinate system. All untrusted numbers are checked for finiteness, positivity, bounds, minimum size, and codec modulus.

## 7. IPC contract

| Command/event | Direction | Purpose |
|---|---|---|
| `probe_video(path)` | UI → Rust | Return `MediaDescriptor` from ffprobe JSON |
| `create_preview(path)` | UI → Rust | Create/reuse local proxy and return its path |
| `start_export(request)` | UI → Rust | Validate request, start job, return job ID |
| `cancel_export(jobId)` | UI → Rust | Terminate matching child process |
| `export-progress` | Rust → UI | `{jobId, fraction, outTimeSeconds}` |
| `export-complete` | Rust → UI | Final output path |
| `export-error` | Rust → UI | User-safe message and diagnostic code |

Only the Rust layer constructs FFmpeg argument arrays. Paths and crop values are never interpolated into a shell command string.

## 8. User interface

The main window uses three regions:

1. A narrow header with product name, source summary, Open, and Export.
2. A flexible dark video stage containing the video and crop overlay.
3. A compact inspector/control footer with playback, scrubber, coordinates, dimensions, aspect ratio, and Reset.

The first-run state is an accessible drop target with a primary Open Video button. Keyboard focus indicators are visible. Controls use semantic buttons and labelled inputs. Errors appear inline and remain copyable.

## 9. Preview strategy

The application first exposes the selected file through Tauri's scoped asset protocol and asks the native WebView media element to play it. If decoding fails, `create_preview` produces an orientation-normalized, square-pixel, maximum-1280-pixel H.264 MP4 proxy with frequent keyframes. The proxy is for interaction only; final export always reads the original.

Proxy cache entries are stored under the operating-system cache directory and keyed by canonical path, file size, and last-write timestamp. Entries are reusable across sessions; bounded LRU cleanup is deferred.

## 10. Error handling and recovery

- Invalid or missing input: return a typed probe error without changing current UI state.
- Unsupported direct preview: offer/perform proxy creation.
- FFmpeg missing: display setup guidance and disable export.
- Export destination exists: save dialog confirmation is required; backend does not silently overwrite.
- Disk full or permission failure: retain the original and remove only the known partial file.
- App closes during export: terminate owned child processes.
- Sidecar output is logged without exposing unrelated environment variables.

## 11. Security, privacy, and distribution

- No network requests occur during normal application use.
- The source is opened read-only.
- File access is limited to user-selected paths and the app cache.
- FFmpeg is invoked as an allowlisted sidecar with structured arguments.
- Sidecar archives are verified against the publisher-provided SHA-256 checksum by the setup script.
- FFmpeg binaries are not committed to Git. Distribution must include the applicable FFmpeg license, build configuration, copyright notices, and corresponding-source obligations for the selected build.

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

### 13.2 Rust unit tests

- Crop validation and full-frame/no-op detection.
- Rotation-aware crop mapping for 0, 90, 180, and 270 degrees.
- FFmpeg argument generation for all three profiles.
- H.264/HEVC metadata-only eligibility.
- Progress-line parsing.
- Temporary output naming and destination validation.

### 13.3 Integration tests

Generated fixtures cover landscape H.264/AAC, rotated portrait, no-audio, and odd requested coordinates. Tests probe and export fixtures with the downloaded sidecars, then ffprobe the output dimensions/codecs and compare representative lossless pixels where practical.

### 13.4 Manual acceptance matrix

- 1080p H.264 MP4
- 4K HEVC 10-bit
- rotated phone MOV
- variable-frame-rate video
- MKV with multiple audio streams
- corrupt/unsupported file
- Unicode and spaces in paths
- output cancellation and disk/permission errors

## 14. Acceptance criteria for 0.1.0

- **AC-001** A user can open a video, drag every crop handle, scrub, play, and reset without leaving the main window.
- **AC-002** Pixel readouts match the crop shown and remain in bounds after resize.
- **AC-003** Compatible and FFV1 exports have the selected physical frame dimensions according to ffprobe.
- **AC-004** Metadata-only export never uses a video encoder and is disabled for non-H.264/HEVC input.
- **AC-005** Cancelling export leaves no final output and the UI can immediately start another export.
- **AC-006** Source files are byte-identical before and after every automated test.
- **AC-007** Frontend tests, Rust tests, type checks, lint checks, and production builds pass.

## 15. Planned increments

1. Repository, SDD, CI-ready conventions.
2. Tauri/Svelte shell and reproducible FFmpeg setup.
3. Probe, direct preview, proxy fallback, and crop interaction.
4. Export profiles, progress, cancellation, and safe finalization.
5. Automated integration fixtures, package build, and documentation polish.

## 16. Verification status

The 0.1.0 baseline satisfies AC-001 through AC-007 at automated or implementation-inspection level. Native file-picker interaction and the wider codec/device matrix remain manual acceptance items. Exact commands, fixture results, tool versions, and produced installer hashes are recorded in `docs/VERIFICATION.md`.
