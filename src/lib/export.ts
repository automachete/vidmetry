export type ExportProfile = 'compatible' | 'lossless' | 'metadata';

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
  message: string;
  cancelled: boolean;
}

export interface OutputSuggestion {
  path: string;
  extension: string;
  filterName: string;
}

export function suggestOutput(sourcePath: string, profile: ExportProfile): OutputSuggestion {
  const separatorIndex = Math.max(sourcePath.lastIndexOf('/'), sourcePath.lastIndexOf('\\'));
  const directory = separatorIndex >= 0 ? sourcePath.slice(0, separatorIndex + 1) : '';
  const fileName = separatorIndex >= 0 ? sourcePath.slice(separatorIndex + 1) : sourcePath;
  const dotIndex = fileName.lastIndexOf('.');
  const stem = dotIndex > 0 ? fileName.slice(0, dotIndex) : fileName || 'video';
  const extension = profile === 'lossless' ? 'mkv' : 'mp4';
  const suffix = profile === 'metadata' ? 'display-crop' : 'cropped';
  return {
    path: `${directory}${stem}_${suffix}.${extension}`,
    extension,
    filterName: profile === 'lossless' ? 'Matroska動画' : 'MP4動画',
  };
}

export function clampProgress(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.min(1, Math.max(0, value));
}
