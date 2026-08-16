export interface MediaDescriptor {
  sourcePath: string;
  fileName: string;
  durationSeconds: number;
  frameCount: number;
  codedWidth: number;
  codedHeight: number;
  displayWidth: number;
  displayHeight: number;
  rotationDegrees: number;
  frameRate: string;
  videoCodec: string;
  pixelFormat: string;
  bitDepth: number | null;
  hasAudio: boolean;
  audioCodec: string | null;
  metadataCropSupported: boolean;
}

export function formatFrameRate(value: string): string {
  const [numeratorText, denominatorText] = value.split('/');
  const numerator = Number(numeratorText);
  const denominator = Number(denominatorText ?? 1);
  if (!Number.isFinite(numerator) || !Number.isFinite(denominator) || denominator === 0) {
    return value;
  }
  const rate = numerator / denominator;
  return Number.isInteger(rate) ? `${rate} fps` : `${rate.toFixed(2)} fps`;
}

export function formatTime(seconds: number): string {
  const safe = Number.isFinite(seconds) ? Math.max(0, seconds) : 0;
  // Work from one integer value so decimal inputs such as 3661.2 do not
  // display as 1:01:01.199 due to binary floating-point representation.
  const totalMilliseconds = Math.floor(safe * 1000 + 1e-6);
  const hours = Math.floor(totalMilliseconds / 3_600_000);
  const minutes = Math.floor((totalMilliseconds % 3_600_000) / 60_000);
  const wholeSeconds = Math.floor((totalMilliseconds % 60_000) / 1000);
  const milliseconds = totalMilliseconds % 1000;
  const main = `${minutes.toString().padStart(hours > 0 ? 2 : 1, '0')}:${wholeSeconds.toString().padStart(2, '0')}`;
  return `${hours > 0 ? `${hours}:${main}` : main}.${milliseconds.toString().padStart(3, '0')}`;
}
