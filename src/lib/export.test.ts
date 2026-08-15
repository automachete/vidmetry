import { describe, expect, it } from 'vitest';
import { clampProgress, suggestOutput } from './export';

describe('export UI helpers', () => {
  it('suggests a compatible output next to a Windows source', () => {
    expect(suggestOutput('C:\\clips\\phone.take.mov', 'compatible')).toEqual({
      path: 'C:\\clips\\phone.take_cropped.mp4',
      extension: 'mp4',
      filterName: 'MP4動画',
    });
  });

  it('uses Matroska for a lossless export', () => {
    expect(suggestOutput('/clips/source', 'lossless').path).toBe('/clips/source_cropped.mkv');
  });

  it('clamps malformed progress payloads', () => {
    expect(clampProgress(-0.2)).toBe(0);
    expect(clampProgress(0.42)).toBe(0.42);
    expect(clampProgress(2)).toBe(1);
    expect(clampProgress(Number.NaN)).toBe(0);
  });
});
