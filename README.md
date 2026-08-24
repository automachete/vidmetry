# Vidmetry

English | [日本語](README.ja.md)

<p align="center">
<img src="src-tauri/icons/128x128.png" width="80" height="80" alt="Vidmetry">
</p>

Vidmetry is a lightweight Windows app for cropping and saving a video's frame and playback range while checking the result in a preview.

You can crop videos as easily as with a standard smartphone app, without having to navigate the complex settings of heavyweight video-editing software.

It supports frame cropping that standard Windows apps cannot handle, and lets you save cropped videos with priority given to playback compatibility, file size, or image quality.

## Installation

| File | Description |
|---|---|
| [Vidmetry_x64-setup.exe](https://github.com/automachete/vidmetry/releases/latest/download/Vidmetry_x64-setup.exe) | Windows 11 NSIS installer |

## Usage

1. Drop a video file or video folder that you want to edit onto Vidmetry, or select it.
2. Move the frame in the preview to set the crop dimensions.
3. Adjust the start and end positions in the frame strip at the bottom of the screen.
4. Check the content by playing or scrolling through it, then choose "Save a copy" or "Save."

## What you can do with Vidmetry

- Set crop coordinates and dimensions in pixels, or lock the aspect ratio
- Adjust the start and end positions of a video frame by frame
- Change loop playback and playback start behavior, and check the content in full-screen view before saving
- Easily switch between videos in a folder for playback and editing

## Quality and saving methods

Vidmetry offers three saving methods, depending on whether you prioritize playback compatibility, file size, or image quality.

MP4 for everyday sharing, lossless saving that preserves pixels, and metadata-only saving that keeps the original compressed video each have the following advantages and disadvantages.

| Saving method | Advantages | Disadvantages |
|---|---|---|
| Compatible MP4 | Works well with many devices and players, with detailed control over the balance between quality, file size, and compatibility. | Because the video is re-encoded, some settings may cause quality loss from additional compression. |
| Lossless FFV1 / MKV | Saves the cropped frame and playback range without degrading the decoded pixels. | Produces large files and can be played by only a limited range of software. |
| Metadata only | Keeps the original compressed video for supported H.264/HEVC files without re-encoding. | Players that do not interpret crop metadata will not apply the crop, and this method cannot be combined with playback-range trimming. |

## Main shortcuts

| Key | Action |
|---|---|
| `Space` | Play / pause |
| `←` | Seek back 1 frame |
| `→` | Seek forward 1 frame |
| `Shift` + `←` | Seek back 10 frames |
| `Shift` + `→` | Seek forward 10 frames |
| `Page Up` | Previous video |
| `Page Down` | Next video |
| `Ctrl` + `S` | Save a copy |
| `Ctrl` + `Shift` + `S` | Save over the source video |
| `F11` | Toggle fullscreen preview |
| `Esc` | Exit fullscreen preview |

## License and development

Vidmetry itself is available under the [MIT License](LICENSE). See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for the licenses and corresponding source of bundled third-party software such as FFmpeg/ffprobe.

See [CONTRIBUTING.md](CONTRIBUTING.md) for development-environment setup and contribution information, and [docs/SDD.md](docs/SDD.md) for the detailed design.
