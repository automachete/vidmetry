import { describe, expect, it } from 'vitest';
import { formatFrameRate, formatTime, requiresCompatiblePreview, type MediaDescriptor } from './media';

describe('media formatting', () => {
  it('formats rational frame rates', () => {
    expect(formatFrameRate('30000/1001')).toBe('29.97 fps');
    expect(formatFrameRate('60/1')).toBe('60 fps');
  });

  it('formats media time', () => {
    expect(formatTime(65.4329)).toBe('1:05.432');
    expect(formatTime(3661.2)).toBe('1:01:01.200');
  });

  it('uses a compatible proxy for Matroska and FFV1 sources', () => {
    const media = {
      sourcePath: 'C:\\clips\\clip.mp4',
      videoCodec: 'h264',
    } as MediaDescriptor;

    expect(requiresCompatiblePreview(media)).toBe(false);
    expect(requiresCompatiblePreview({ ...media, sourcePath: 'C:\\clips\\clip.MKV' })).toBe(true);
    expect(requiresCompatiblePreview({ ...media, videoCodec: 'FFV1' })).toBe(true);
  });
});
