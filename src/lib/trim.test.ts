import { describe, expect, it } from 'vitest';

import {
  adaptiveFrameQuantum,
  frameToSeconds,
  fullTrimRange,
  isFullTrim,
  parseFrameRate,
  pointerFrameFromTimeline,
  sanitizeTrimRange,
  secondsToFrame,
  totalVideoFrames,
  trimDuration,
  updateTrimHandle,
} from './trim';

describe('frame-accurate time trimming', () => {
  it('uses a reported frame count and otherwise derives one from a rational frame rate', () => {
    expect(parseFrameRate('30000/1001')).toBeCloseTo(29.97002997);
    expect(totalVideoFrames(10, '30000/1001', 301)).toBe(301);
    expect(totalVideoFrames(10, '30/1')).toBe(300);
  });

  it('keeps an exclusive end frame and at least one selected frame', () => {
    expect(sanitizeTrimRange({ startFrame: -4, endFrame: 400 }, 300)).toEqual({
      startFrame: 0,
      endFrame: 300,
    });
    expect(updateTrimHandle({ startFrame: 20, endFrame: 40 }, 'start', 40, 300)).toEqual({
      startFrame: 39,
      endFrame: 40,
    });
    expect(updateTrimHandle({ startFrame: 20, endFrame: 40 }, 'end', 20, 300)).toEqual({
      startFrame: 20,
      endFrame: 21,
    });
  });

  it('maps frames to preview time without accumulating floating point error', () => {
    expect(frameToSeconds(30, 120, 4)).toBe(1);
    expect(secondsToFrame(1, 120, 4)).toBe(30);
    expect(trimDuration({ startFrame: 30, endFrame: 90 }, 120, 4)).toBe(2);
    expect(isFullTrim(fullTrimRange(120), 120)).toBe(true);
  });

  it('uses frame-level snapping for slow drags and timeline-scale snapping for fast drags', () => {
    expect(adaptiveFrameQuantum(18_000, 900, 0.05)).toBe(1);
    expect(adaptiveFrameQuantum(18_000, 900, 2)).toBe(20);
    expect(adaptiveFrameQuantum(300, 900, 0.05)).toBe(1);
  });

  it('maps from the absolute pointer position without accumulating handle drift', () => {
    expect(pointerFrameFromTimeline(475, 100, 800, 800, 5, 0.05)).toBe(370);
    expect(pointerFrameFromTimeline(475, 100, 800, 16_000, 5, 2)).toBe(7_400);
    expect(pointerFrameFromTimeline(-100, 100, 800, 800, 0, 0.05)).toBe(0);
  });
});
