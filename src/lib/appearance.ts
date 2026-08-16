export type AppTheme = 'light' | 'dark';

const HEX_COLOR = /^#[0-9a-f]{6}$/i;

export function normalizeAccent(value: unknown): string {
  return typeof value === 'string' && HEX_COLOR.test(value) ? value.toUpperCase() : '#0078D4';
}

export function fallbackTheme(prefersDark: boolean): AppTheme {
  return prefersDark ? 'dark' : 'light';
}

export function accentTextColor(accent: string): '#000000' | '#FFFFFF' {
  const normalized = normalizeAccent(accent);
  const channels = [1, 3, 5].map((offset) => Number.parseInt(normalized.slice(offset, offset + 2), 16));
  const linear = channels.map((channel) => {
    const value = channel / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  const luminance = linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722;
  return luminance > 0.36 ? '#000000' : '#FFFFFF';
}

export function applySystemAppearance(
  theme: AppTheme,
  accent: string,
  root: HTMLElement = document.documentElement,
): void {
  const normalizedAccent = normalizeAccent(accent);
  root.dataset.theme = theme;
  root.style.setProperty('--accent', normalizedAccent);
  root.style.setProperty('--accent-contrast', accentTextColor(normalizedAccent));
}
