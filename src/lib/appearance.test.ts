import { describe, expect, it } from 'vitest';

import { accentTextColor, applySystemAppearance, fallbackTheme, normalizeAccent } from './appearance';

describe('Windows appearance projection', () => {
  it('normalizes valid colors and falls back safely', () => {
    expect(normalizeAccent('#1a2b3c')).toBe('#1A2B3C');
    expect(normalizeAccent('green')).toBe('#0078D4');
  });

  it('chooses readable foreground colors for accent controls', () => {
    expect(accentTextColor('#F5C400')).toBe('#000000');
    expect(accentTextColor('#004E8C')).toBe('#FFFFFF');
  });

  it('applies the OS mode and accent to the document root', () => {
    const root = document.createElement('div');
    applySystemAppearance('light', '#ff8c00', root);
    expect(root.dataset.theme).toBe('light');
    expect(root.style.getPropertyValue('--accent')).toBe('#FF8C00');
    expect(root.style.getPropertyValue('--accent-contrast')).toBe('#000000');
    expect(fallbackTheme(true)).toBe('dark');
  });
});
