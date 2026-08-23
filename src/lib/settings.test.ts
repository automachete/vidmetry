import { describe, expect, it } from 'vitest';
import {
  defaultSettings,
  loadSettings,
  parseSettings,
  persistSettings,
  resolveLanguage,
  type SettingsStore,
} from './settings';

class MemoryStore implements SettingsStore {
  values = new Map<string, unknown>();
  saveCount = 0;

  async get<T>(key: string) {
    return this.values.get(key) as T | undefined;
  }

  async set(key: string, value: unknown) {
    this.values.set(key, structuredClone(value));
  }

  async save() {
    this.saveCount += 1;
  }
}

describe('persistent settings', () => {
  it('round-trips export, appearance, shortcuts, folder-picker, loop, Explorer, and language preferences through the durable store', async () => {
    const store = new MemoryStore();
    await persistSettings(
      {
        ...defaultSettings,
        languageMode: 'manual',
        language: 'en',
        appearance: { ...defaultSettings.appearance, themeMode: 'manual', theme: 'light' },
        shortcuts: { ...defaultSettings.shortcuts, openVideo: 'Alt+KeyO' },
        loopPlayback: false,
        explorerIntegration: false,
        folderPicker: {
          mode: 'explorerBeta',
          lastPath: 'D:\\Videos',
          viewMode: 4,
          iconSize: 24,
        },
        export: { ...defaultSettings.export, videoCodec: 'h265', encoder: 'nvidia', crf: 21 },
      },
      store,
    );
    await expect(loadSettings(store)).resolves.toMatchObject({
      languageMode: 'manual',
      language: 'en',
      appearance: { themeMode: 'manual', theme: 'light' },
      shortcuts: { openVideo: 'Alt+KeyO' },
      loopPlayback: false,
      explorerIntegration: false,
      folderPicker: { mode: 'explorerBeta', lastPath: 'D:\\Videos', viewMode: 4, iconSize: 24 },
      export: { videoCodec: 'h265', encoder: 'nvidia', crf: 21 },
    });
    expect(store.saveCount).toBe(1);
  });

  it('uses defaults when the durable store has no settings yet', async () => {
    await expect(loadSettings(new MemoryStore())).resolves.toEqual(defaultSettings);
    expect(defaultSettings.loopPlayback).toBe(true);
    expect(defaultSettings.export.pixelFormat).toBe('source');
  });

  it('rejects invalid persisted values instead of silently repairing them', async () => {
    const store = new MemoryStore();
    store.values.set('settings', {
      languageMode: 'invalid',
      loopPlayback: 'yes',
      export: { crf: 200, profile: 'unknown', audioBitrateKbps: -1 },
    });
    await expect(loadSettings(store)).rejects.toThrow();
  });

  it('rejects obsolete and partial settings shapes', () => {
    expect(() => parseSettings({ ...defaultSettings, version: 1 })).toThrow();
    expect(() =>
      parseSettings({
        ...defaultSettings,
        export: { ...defaultSettings.export, fastStart: true },
      }),
    ).toThrow();
    const { explorerIntegration: _, ...withoutExplorerIntegration } = defaultSettings;
    expect(() => parseSettings(withoutExplorerIntegration)).toThrow();
    const { shortcuts: __, ...withoutShortcuts } = defaultSettings;
    expect(() => parseSettings(withoutShortcuts)).toThrow();
    const { folderPicker: ___, ...withoutFolderPicker } = defaultSettings;
    expect(() => parseSettings(withoutFolderPicker)).toThrow();
    expect(() =>
      parseSettings({
        ...defaultSettings,
        folderPicker: { mode: 'standard', lastPath: 'D:\\', viewMode: 9, iconSize: 96 },
      }),
    ).toThrow();
    expect(() => parseSettings({ languageMode: 'system' })).toThrow();
  });

  it('resolves Japanese from the OS and supports a manual override', () => {
    expect(resolveLanguage(defaultSettings, 'ja-JP')).toBe('ja');
    expect(resolveLanguage(defaultSettings, 'de-DE')).toBe('en');
    expect(
      resolveLanguage({ ...defaultSettings, languageMode: 'manual', language: 'ja' }, 'en-US'),
    ).toBe('ja');
  });
});
