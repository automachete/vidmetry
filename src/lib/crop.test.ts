import { describe, expect, it } from 'vitest';
import {
  aspectRatio,
  cropStyle,
  dragCrop,
  fitAspect,
  fullFrame,
  sanitizeRect,
  screenDeltaToSource,
} from './crop';

const bounds = { width: 1920, height: 1080 };

describe('crop geometry', () => {
  it('creates a full-frame crop', () => {
    expect(fullFrame(bounds)).toEqual({ x: 0, y: 0, width: 1920, height: 1080 });
  });

  it('snaps and clamps rectangles to even source pixels', () => {
    expect(sanitizeRect({ x: 191, y: -3, width: 1731, height: 1001 }, bounds)).toEqual({
      x: 188,
      y: 0,
      width: 1732,
      height: 1002,
    });
  });

  it('moves without leaving the frame', () => {
    const start = { x: 100, y: 100, width: 800, height: 600 };
    expect(dragCrop(start, 'move', 2000, 2000, bounds, null)).toEqual({
      x: 1120,
      y: 480,
      width: 800,
      height: 600,
    });
  });

  it('resizes from the north-west spatial-crop handle', () => {
    const start = { x: 200, y: 100, width: 800, height: 600 };
    expect(dragCrop(start, 'north-west', -100, -50, bounds, null)).toEqual({
      x: 100,
      y: 50,
      width: 900,
      height: 650,
    });
  });

  it('maintains a square constraint', () => {
    const start = { x: 200, y: 100, width: 600, height: 600 };
    const result = dragCrop(start, 'south-east', 200, 20, bounds, 1);
    expect(result.width).toBe(result.height);
    expect(result.x).toBe(200);
    expect(result.y).toBe(100);
  });

  it('fits a selected ratio inside the current rectangle', () => {
    const result = fitAspect(fullFrame(bounds), 1, bounds);
    expect(result).toEqual({ x: 420, y: 0, width: 1080, height: 1080 });
    expect(aspectRatio('16:9', bounds)).toBeCloseTo(16 / 9);
  });

  it('maps screen movement and styles back to the source frame', () => {
    expect(screenDeltaToSource(100, 50, 960, 540, bounds)).toEqual({ x: 200, y: 100 });
    expect(cropStyle({ x: 960, y: 540, width: 960, height: 540 }, bounds)).toBe(
      'left:50%;top:50%;width:50%;height:50%',
    );
  });
});
