export interface CropRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface CropBounds {
  width: number;
  height: number;
}

export type CropHandle =
  | 'move'
  | 'north'
  | 'north-east'
  | 'east'
  | 'south-east'
  | 'south'
  | 'south-west'
  | 'west'
  | 'north-west';

export type AspectPreset = 'free' | 'source' | '1:1' | '4:3' | '16:9' | '9:16';

export const MIN_CROP_SIZE = 16;

const clamp = (value: number, minimum: number, maximum: number) =>
  Math.min(Math.max(value, minimum), maximum);

const snapDown = (value: number, modulus: number) =>
  modulus <= 1 ? Math.floor(value) : Math.floor(value / modulus) * modulus;

const snapNearest = (value: number, modulus: number) =>
  modulus <= 1 ? Math.round(value) : Math.round(value / modulus) * modulus;

export function fullFrame(bounds: CropBounds, modulus = 2): CropRect {
  return {
    x: 0,
    y: 0,
    width: Math.max(modulus, snapDown(bounds.width, modulus)),
    height: Math.max(modulus, snapDown(bounds.height, modulus)),
  };
}

export function aspectRatio(preset: AspectPreset, bounds: CropBounds): number | null {
  if (preset === 'free') return null;
  if (preset === 'source') return bounds.width / bounds.height;
  const [width, height] = preset.split(':').map(Number);
  return width / height;
}

export function sanitizeRect(
  rect: CropRect,
  bounds: CropBounds,
  modulus = 2,
  minimumSize = MIN_CROP_SIZE,
): CropRect {
  const maxWidth = Math.max(modulus, snapDown(bounds.width, modulus));
  const maxHeight = Math.max(modulus, snapDown(bounds.height, modulus));
  const minWidth = Math.min(maxWidth, Math.max(modulus, snapDown(minimumSize, modulus)));
  const minHeight = Math.min(maxHeight, Math.max(modulus, snapDown(minimumSize, modulus)));
  const width = clamp(snapNearest(rect.width, modulus), minWidth, maxWidth);
  const height = clamp(snapNearest(rect.height, modulus), minHeight, maxHeight);
  const x = clamp(snapNearest(rect.x, modulus), 0, maxWidth - width);
  const y = clamp(snapNearest(rect.y, modulus), 0, maxHeight - height);
  return { x, y, width, height };
}

export function fitAspect(
  rect: CropRect,
  ratio: number,
  bounds: CropBounds,
  modulus = 2,
): CropRect {
  if (!Number.isFinite(ratio) || ratio <= 0) return sanitizeRect(rect, bounds, modulus);
  let width = rect.width;
  let height = width / ratio;
  if (height > rect.height) {
    height = rect.height;
    width = height * ratio;
  }
  if (width > bounds.width) {
    width = bounds.width;
    height = width / ratio;
  }
  if (height > bounds.height) {
    height = bounds.height;
    width = height * ratio;
  }
  return sanitizeRect(
    {
      x: rect.x + (rect.width - width) / 2,
      y: rect.y + (rect.height - height) / 2,
      width,
      height,
    },
    bounds,
    modulus,
  );
}

export function dragCrop(
  start: CropRect,
  handle: CropHandle,
  deltaX: number,
  deltaY: number,
  bounds: CropBounds,
  ratio: number | null,
  modulus = 2,
): CropRect {
  if (handle === 'move') {
    return sanitizeRect(
      { ...start, x: start.x + deltaX, y: start.y + deltaY },
      bounds,
      modulus,
    );
  }

  const movesNorth = handle.includes('north');
  const movesSouth = handle.includes('south');
  const movesWest = handle.includes('west');
  const movesEast = handle.includes('east');
  let left = start.x + (movesWest ? deltaX : 0);
  let right = start.x + start.width + (movesEast ? deltaX : 0);
  let top = start.y + (movesNorth ? deltaY : 0);
  let bottom = start.y + start.height + (movesSouth ? deltaY : 0);

  if (movesWest) left = Math.min(left, right - MIN_CROP_SIZE);
  if (movesEast) right = Math.max(right, left + MIN_CROP_SIZE);
  if (movesNorth) top = Math.min(top, bottom - MIN_CROP_SIZE);
  if (movesSouth) bottom = Math.max(bottom, top + MIN_CROP_SIZE);

  let candidate: CropRect = {
    x: left,
    y: top,
    width: right - left,
    height: bottom - top,
  };

  if (ratio && Number.isFinite(ratio) && ratio > 0) {
    const horizontal = movesWest || movesEast;
    const vertical = movesNorth || movesSouth;
    const widthChange = Math.abs(candidate.width - start.width) / Math.max(1, start.width);
    const heightChange = Math.abs(candidate.height - start.height) / Math.max(1, start.height);
    const widthDriven = horizontal && (!vertical || widthChange >= heightChange);
    let width = widthDriven ? candidate.width : candidate.height * ratio;
    let height = width / ratio;

    const maximumWidth = horizontal
      ? movesWest
        ? start.x + start.width
        : bounds.width - start.x
      : bounds.width;
    const maximumHeight = vertical
      ? movesNorth
        ? start.y + start.height
        : bounds.height - start.y
      : bounds.height;
    const scale = Math.min(1, maximumWidth / width, maximumHeight / height);
    width *= scale;
    height *= scale;

    const centerX = start.x + start.width / 2;
    const centerY = start.y + start.height / 2;
    const x = movesWest ? start.x + start.width - width : movesEast ? start.x : centerX - width / 2;
    const y = movesNorth
      ? start.y + start.height - height
      : movesSouth
        ? start.y
        : centerY - height / 2;
    candidate = { x, y, width, height };
  }

  return sanitizeRect(candidate, bounds, modulus);
}

export function screenDeltaToSource(
  deltaX: number,
  deltaY: number,
  renderedWidth: number,
  renderedHeight: number,
  bounds: CropBounds,
) {
  return {
    x: renderedWidth > 0 ? (deltaX / renderedWidth) * bounds.width : 0,
    y: renderedHeight > 0 ? (deltaY / renderedHeight) * bounds.height : 0,
  };
}

export function cropStyle(rect: CropRect, bounds: CropBounds): string {
  return [
    `left:${(rect.x / bounds.width) * 100}%`,
    `top:${(rect.y / bounds.height) * 100}%`,
    `width:${(rect.width / bounds.width) * 100}%`,
    `height:${(rect.height / bounds.height) * 100}%`,
  ].join(';');
}

