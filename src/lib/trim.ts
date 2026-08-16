export interface TrimRange {
  /** Inclusive source-frame index. */
  startFrame: number;
  /** Exclusive source-frame index. */
  endFrame: number;
}

export type TrimHandle = 'start' | 'end';

export function parseFrameRate(value: string): number {
  const [numeratorText, denominatorText = '1'] = value.split('/');
  const numerator = Number(numeratorText);
  const denominator = Number(denominatorText);
  if (!Number.isFinite(numerator) || !Number.isFinite(denominator) || denominator <= 0) return 0;
  const rate = numerator / denominator;
  return Number.isFinite(rate) && rate > 0 ? rate : 0;
}

export function totalVideoFrames(
  durationSeconds: number,
  frameRate: string,
  reportedFrameCount: number | null = null,
): number {
  if (
    reportedFrameCount !== null &&
    Number.isSafeInteger(reportedFrameCount) &&
    reportedFrameCount > 0
  ) {
    return reportedFrameCount;
  }
  const duration = Number.isFinite(durationSeconds) ? Math.max(0, durationSeconds) : 0;
  const rate = parseFrameRate(frameRate);
  return Math.max(1, Math.round(duration * (rate || 30)));
}

export function fullTrimRange(totalFrames: number): TrimRange {
  return { startFrame: 0, endFrame: Math.max(1, Math.floor(totalFrames)) };
}

export function sanitizeTrimRange(range: TrimRange, totalFrames: number): TrimRange {
  const total = Math.max(1, Math.floor(totalFrames));
  const startFrame = Math.min(total - 1, Math.max(0, Math.round(range.startFrame)));
  const endFrame = Math.min(total, Math.max(startFrame + 1, Math.round(range.endFrame)));
  return { startFrame, endFrame };
}

export function updateTrimHandle(
  range: TrimRange,
  handle: TrimHandle,
  frame: number,
  totalFrames: number,
): TrimRange {
  const safe = sanitizeTrimRange(range, totalFrames);
  if (handle === 'start') {
    return { ...safe, startFrame: Math.min(safe.endFrame - 1, Math.max(0, Math.round(frame))) };
  }
  return {
    ...safe,
    endFrame: Math.max(safe.startFrame + 1, Math.min(Math.floor(totalFrames), Math.round(frame))),
  };
}

export function frameToSeconds(frame: number, totalFrames: number, durationSeconds: number): number {
  const total = Math.max(1, Math.floor(totalFrames));
  const duration = Number.isFinite(durationSeconds) ? Math.max(0, durationSeconds) : 0;
  return (Math.min(total, Math.max(0, frame)) / total) * duration;
}

export function secondsToFrame(
  seconds: number,
  totalFrames: number,
  durationSeconds: number,
): number {
  const total = Math.max(1, Math.floor(totalFrames));
  if (!Number.isFinite(durationSeconds) || durationSeconds <= 0) return 0;
  const safe = Math.min(durationSeconds, Math.max(0, seconds));
  return Math.min(total, Math.max(0, Math.round((safe / durationSeconds) * total)));
}

export function trimDuration(
  range: TrimRange,
  totalFrames: number,
  durationSeconds: number,
): number {
  const safe = sanitizeTrimRange(range, totalFrames);
  return frameToSeconds(safe.endFrame - safe.startFrame, totalFrames, durationSeconds);
}

export function isFullTrim(range: TrimRange, totalFrames: number): boolean {
  const safe = sanitizeTrimRange(range, totalFrames);
  return safe.startFrame === 0 && safe.endFrame === Math.max(1, Math.floor(totalFrames));
}

/**
 * Converts pointer velocity to timeline sensitivity. Slow movement always reaches
 * a sub-frame-per-pixel precision on long clips; quick movement spans the full
 * timeline at its natural frames-per-pixel rate.
 */
export function adaptiveFrameQuantum(
  totalFrames: number,
  renderedWidth: number,
  velocityPixelsPerMillisecond: number,
): number {
  const natural = Math.max(1, totalFrames) / Math.max(1, renderedWidth);
  const velocity = Number.isFinite(velocityPixelsPerMillisecond)
    ? Math.max(0, velocityPixelsPerMillisecond)
    : 0;
  if (velocity <= 0.12) return 1;
  if (velocity >= 1.2) return Math.max(1, Math.round(natural));
  const amount = (velocity - 0.12) / (1.2 - 0.12);
  return Math.max(1, Math.round(1 + (natural - 1) * amount));
}

/**
 * Maps the pointer's absolute timeline position to a frame. Slow motion snaps to
 * a single frame; quick motion coarsens the snap without allowing accumulated
 * relative-drag error to separate a trim-boundary handle from the pointer.
 */
export function pointerFrameFromTimeline(
  clientX: number,
  timelineLeft: number,
  renderedWidth: number,
  totalFrames: number,
  grabOffsetX: number,
  velocityPixelsPerMillisecond: number,
): number {
  const width = Math.max(1, renderedWidth);
  const total = Math.max(1, Math.floor(totalFrames));
  const rawFrame = ((clientX - grabOffsetX - timelineLeft) / width) * total;
  const quantum = adaptiveFrameQuantum(total, width, velocityPixelsPerMillisecond);
  return Math.min(total, Math.max(0, Math.round(rawFrame / quantum) * quantum));
}
