# Vidmetry

Vidmetry is a focused desktop application for spatially cropping video. It combines an image-editor-style crop box with playback and scrubbing, then exports through FFmpeg using an explicit quality profile.

The initial target is Windows 11. The architecture keeps the UI and media engine portable to macOS and Linux.

## Status

Version `0.1.0` is under active implementation. See [docs/SDD.md](docs/SDD.md) for the requirements, architecture, and acceptance criteria.

## Product principles

- One task per window: open, frame, preview, export.
- The source file is never modified.
- Crop coordinates are exact and visible in pixels.
- Export behavior is honest about compatibility and quality.
- Media stays on the local computer.

