import type { AppErrorPayload } from './app-error';

export type ExportProfile = 'compatible' | 'lossless' | 'metadata';
export type VideoCodec = 'h264' | 'h265';
export type EncoderPreset =
  | 'ultrafast'
  | 'superfast'
  | 'veryfast'
  | 'faster'
  | 'fast'
  | 'medium'
  | 'slow'
  | 'slower'
  | 'veryslow';
export type PixelFormat =
  | 'source'
  | 'yuv420p'
  | 'yuv420p10le'
  | 'yuv422p'
  | 'yuv422p10le'
  | 'yuv444p'
  | 'yuv444p10le';
export type AudioMode = 'auto' | 'copy' | 'aac' | 'flac' | 'pcm' | 'none';
export type FrameRateMode = 'passthrough' | 'constant';

export interface ExportSettings {
  profile: ExportProfile;
  videoCodec: VideoCodec;
  crf: number;
  preset: EncoderPreset;
  pixelFormat: PixelFormat;
  audioMode: AudioMode;
  audioBitrateKbps: number;
  frameRateMode: FrameRateMode;
  constantFrameRate: number;
  fastStart: boolean;
  preserveMetadata: boolean;
  copySubtitles: boolean;
}

export interface ExportProgressEvent {
  jobId: string;
  fraction: number;
  outTimeSeconds: number;
}

export interface ExportCompleteEvent {
  jobId: string;
  outputPath: string;
}

export interface ExportErrorEvent {
  jobId: string;
  error: AppErrorPayload;
  cancelled: boolean;
}

export interface OutputSuggestion {
  path: string;
  extension: string;
}

const metadataContainers = new Set(['mp4', 'm4v', 'mov', 'mkv']);

export function outputExtension(sourcePath: string, profile: ExportProfile): string {
  if (profile === 'compatible') return 'mp4';
  if (profile === 'lossless') return 'mkv';
  const source = extensionOf(sourcePath);
  return metadataContainers.has(source) ? source : 'mp4';
}

export function suggestOutput(sourcePath: string, profile: ExportProfile): OutputSuggestion {
  const separatorIndex = Math.max(sourcePath.lastIndexOf('/'), sourcePath.lastIndexOf('\\'));
  const directory = separatorIndex >= 0 ? sourcePath.slice(0, separatorIndex + 1) : '';
  const fileName = separatorIndex >= 0 ? sourcePath.slice(separatorIndex + 1) : sourcePath;
  const dotIndex = fileName.lastIndexOf('.');
  const stem = dotIndex > 0 ? fileName.slice(0, dotIndex) : fileName || 'video';
  const extension = outputExtension(sourcePath, profile);
  const suffix = profile === 'metadata' ? 'display-crop' : 'cropped';
  return { path: `${directory}${stem}_${suffix}.${extension}`, extension };
}

export function canSaveInPlace(sourcePath: string, profile: ExportProfile): boolean {
  return extensionOf(sourcePath) === outputExtension(sourcePath, profile);
}

export function extensionOf(path: string): string {
  const separatorIndex = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
  const fileName = separatorIndex >= 0 ? path.slice(separatorIndex + 1) : path;
  const dotIndex = fileName.lastIndexOf('.');
  return dotIndex > 0 ? fileName.slice(dotIndex + 1).toLowerCase() : '';
}

export function clampProgress(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.min(1, Math.max(0, value));
}
