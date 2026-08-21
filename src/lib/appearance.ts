export type AppTheme = 'light' | 'dark';
export const appThemes = ['light', 'dark'] as const;
export const appearanceModes = ['system', 'manual'] as const;
export const accentColorIds = ['blue', 'teal', 'green', 'gold', 'orange', 'red', 'magenta', 'purple'] as const;

export type AppearanceMode = (typeof appearanceModes)[number];
export type AccentColorId = (typeof accentColorIds)[number];

export interface AppearancePreferences {
  themeMode: AppearanceMode;
  theme: AppTheme;
  accentMode: AppearanceMode;
  accentColor: AccentColorId;
}

export const accentPalette: Record<AccentColorId, Record<AppTheme, string>> = {
  blue: { light: '#0078D4', dark: '#3595DE' },
  teal: { light: '#038387', dark: '#2AA0A4' },
  green: { light: '#107C10', dark: '#359B35' },
  gold: { light: '#937700', dark: '#C19C00' },
  orange: { light: '#BC4B09', dark: '#F7630C' },
  red: { light: '#D13438', dark: '#DC5E62' },
  magenta: { light: '#BF0077', dark: '#CE3293' },
  purple: { light: '#5C2E91', dark: '#9470BD' },
};

const HEX_COLOR = /^#[0-9a-f]{6}$/i;

export function normalizeAccent(value: unknown): string {
  return typeof value === 'string' && HEX_COLOR.test(value) ? value.toUpperCase() : '#0078D4';
}

export function fallbackTheme(prefersDark: boolean): AppTheme {
  return prefersDark ? 'dark' : 'light';
}

export function resolveAppearance(
  preferences: AppearancePreferences,
  systemTheme: AppTheme,
  systemAccent: string,
): { theme: AppTheme; accent: string } {
  const theme = preferences.themeMode === 'system' ? systemTheme : preferences.theme;
  const accent =
    preferences.accentMode === 'system'
      ? normalizeAccent(systemAccent)
      : accentPalette[preferences.accentColor][theme];
  return { theme, accent };
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
