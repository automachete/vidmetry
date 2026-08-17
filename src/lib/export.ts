import type { AppErrorPayload } from './app-error';
import type { CropRect } from './crop';
import type { TrimRange } from './trim';

export const exportProfiles = ['compatible', 'lossless', 'metadata'] as const;
export const videoCodecs = ['h264', 'h265'] as const;
export const encoderPresets = [
  'ultrafast',
  'superfast',
  'veryfast',
  'faster',
  'fast',
  'medium',
  'slow',
  'slower',
  'veryslow',
] as const;
export const pixelFormats = [
  'source',
  'yuv420p',
  'yuv420p10le',
  'yuv422p',
  'yuv422p10le',
  'yuv444p',
  'yuv444p10le',
] as const;
export const audioModes = ['auto', 'copy', 'aac', 'flac', 'pcm', 'none'] as const;
export const frameRateModes = ['passthrough', 'constant'] as const;

export type ExportProfile = (typeof exportProfiles)[number];
export type VideoCodec = (typeof videoCodecs)[number];
export type EncoderPreset = (typeof encoderPresets)[number];
export type PixelFormat = (typeof pixelFormats)[number];
export type AudioMode = (typeof audioModes)[number];
export type FrameRateMode = (typeof frameRateModes)[number];

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
  preserveMetadata: boolean;
  copySubtitles: boolean;
}

export interface ExportRequest {
  sourcePath: string;
  outputPath: string;
  crop: CropRect;
  trim: TrimRange;
  settings: ExportSettings;
  overwrite: boolean;
  inPlace: boolean;
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
