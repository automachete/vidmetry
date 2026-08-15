import { describe, expect, it } from 'vitest';
import { formatFrameRate, formatTime } from './media';

describe('media formatting', () => {
  it('formats rational frame rates', () => {
    expect(formatFrameRate('30000/1001')).toBe('29.97 fps');
    expect(formatFrameRate('60/1')).toBe('60 fps');
  });

  it('formats media time', () => {
    expect(formatTime(65.4329)).toBe('1:05.432');
    expect(formatTime(3661.2)).toBe('1:01:01.200');
  });
});

