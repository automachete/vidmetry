import { describe, expect, it } from 'vitest';

import {
  defaultShortcuts,
  findShortcutConflict,
  formatShortcutChord,
  isShortcutChord,
  reservedShortcutChords,
  shortcutChordFromEvent,
  shortcutMatchesEvent,
} from './shortcuts';

describe('custom keyboard shortcuts', () => {
  it('uses conventional defaults for opening, settings, and profile selection', () => {
    expect(defaultShortcuts).toEqual({
      openVideo: 'Ctrl+KeyO',
      openFolder: 'Ctrl+Shift+KeyO',
      openSettings: 'Ctrl+Comma',
      profileCompatible: 'Alt+Digit1',
      profileLossless: 'Alt+Digit2',
      profileMetadata: 'Alt+Digit3',
    });
    expect(formatShortcutChord(defaultShortcuts.openSettings)).toBe('Ctrl+,');
    expect(formatShortcutChord(defaultShortcuts.profileLossless)).toBe('Alt+2');
  });

  it('captures and matches canonical modifier order', () => {
    const event = { altKey: false, code: 'KeyO', ctrlKey: true, metaKey: false, shiftKey: true };
    expect(shortcutChordFromEvent(event)).toBe('Ctrl+Shift+KeyO');
    expect(shortcutMatchesEvent(defaultShortcuts.openFolder, event)).toBe(true);
    expect(isShortcutChord('Shift+Ctrl+KeyO')).toBe(false);
    expect(isShortcutChord('Ctrl+Ctrl+KeyO')).toBe(false);
    expect(isShortcutChord('Ctrl+LaunchMail')).toBe(false);
  });

  it('rejects Windows-key chords and identifies duplicate or reserved assignments', () => {
    expect(
      shortcutChordFromEvent({ altKey: false, code: 'KeyO', ctrlKey: false, metaKey: true, shiftKey: false }),
    ).toBeNull();
    expect(findShortcutConflict('openVideo', defaultShortcuts.openFolder, defaultShortcuts)).toBe(
      'openFolder',
    );
    expect(reservedShortcutChords.has('Ctrl+KeyS')).toBe(true);
  });
});
