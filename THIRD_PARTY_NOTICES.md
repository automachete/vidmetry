# Third-party notices

Vidmetry distributes FFmpeg and ffprobe as separate command-line sidecars. They remain separate programs and are covered by their own license.

## FFmpeg 9.0.1 essentials build

- Distributor release: <https://github.com/GyanD/codexffmpeg/releases/tag/9.0.1>
- Binary archive: `ffmpeg-9.0.1-essentials_build.zip`
- Archive SHA-256: `fec81ae03971d9dd4be3ebe02e263bd2ec1d789483f931bdba5f5715e65da2e9`
- FFmpeg source revision: <https://github.com/FFmpeg/FFmpeg/commit/bf1b838f2a>
- License: GNU General Public License version 3
- FFmpeg legal information: <https://ffmpeg.org/legal.html>

`scripts/setup-ffmpeg.ps1` verifies the pinned archive and every extracted file used by Vidmetry. Each installer includes:

- `FFmpeg/FFMPEG_LICENSE.txt`: the GPLv3 license shipped in the pinned build;
- `FFmpeg/FFMPEG_BUILD_INFO.txt`: the distributor's exact version, source revision, build configuration, enabled components, and component source links.

Vidmetry's MIT license does not replace or modify the terms that apply to the bundled FFmpeg programs.
