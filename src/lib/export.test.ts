import { describe, expect, it } from 'vitest';
import { canSaveInPlace, clampProgress, outputExtension, suggestOutput } from './export';

describe('export UI helpers', () => {
  it('suggests a compatible output next to a Windows source', () => {
    expect(suggestOutput('C:\\clips\\phone.take.mov', 'compatible')).toEqual({
      path: 'C:\\clips\\phone.take_cropped.mp4',
      extension: 'mp4',
    });
  });

  it('uses Matroska for a lossless export', () => {
    expect(suggestOutput('/clips/source', 'lossless').path).toBe('/clips/source_cropped.mkv');
  });

  it('retains a supported source container for metadata crop', () => {
    expect(outputExtension('C:\\clips\\source.MOV', 'metadata')).toBe('mov');
    expect(suggestOutput('C:\\clips\\source.MOV', 'metadata').path).toBe(
      'C:\\clips\\source_display-crop.mov',
    );
  });

  it('allows in-place save only when the configured extension matches', () => {
    expect(canSaveInPlace('movie.mp4', 'compatible')).toBe(true);
    expect(canSaveInPlace('movie.mov', 'compatible')).toBe(false);
    expect(canSaveInPlace('movie.mkv', 'lossless')).toBe(true);
  });

  it('clamps malformed progress payloads', () => {
    expect(clampProgress(-0.2)).toBe(0);
    expect(clampProgress(0.42)).toBe(0.42);
    expect(clampProgress(2)).toBe(1);
    expect(clampProgress(Number.NaN)).toBe(0);
  });
});
