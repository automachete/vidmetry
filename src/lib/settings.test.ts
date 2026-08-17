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
  it('round-trips export, loop, Explorer, and language preferences through the durable store', async () => {
    const store = new MemoryStore();
    await persistSettings(
      {
        ...defaultSettings,
        languageMode: 'manual',
        language: 'en',
        loopPlayback: true,
        explorerIntegration: false,
        export: { ...defaultSettings.export, videoCodec: 'h265', encoder: 'nvidia', crf: 21 },
      },
      store,
    );
    await expect(loadSettings(store)).resolves.toMatchObject({
      languageMode: 'manual',
      language: 'en',
      loopPlayback: true,
      explorerIntegration: false,
      export: { videoCodec: 'h265', encoder: 'nvidia', crf: 21 },
    });
    expect(store.saveCount).toBe(1);
  });

  it('uses defaults when the durable store has no settings yet', async () => {
    await expect(loadSettings(new MemoryStore())).resolves.toEqual(defaultSettings);
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
