import { describe, expect, it } from 'vitest';
import {
  defaultSettings,
  loadSettings,
  normalizeSettings,
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
  it('round-trips export, loop, and language preferences through the durable store', async () => {
    const store = new MemoryStore();
    await persistSettings(
      {
        ...defaultSettings,
        languageMode: 'manual',
        language: 'en',
        loopPlayback: true,
        export: { ...defaultSettings.export, videoCodec: 'h265', crf: 21 },
      },
      store,
    );
    await expect(loadSettings(store)).resolves.toMatchObject({
      languageMode: 'manual',
      language: 'en',
      loopPlayback: true,
      export: { videoCodec: 'h265', crf: 21 },
    });
    expect(store.saveCount).toBe(1);
  });

  it('uses defaults when the durable store has no settings yet', async () => {
    await expect(loadSettings(new MemoryStore())).resolves.toEqual(defaultSettings);
  });

  it('repairs invalid persisted values', () => {
    const normalized = normalizeSettings({
      languageMode: 'invalid',
      loopPlayback: 'yes',
      export: { crf: 200, profile: 'unknown', audioBitrateKbps: -1 },
    });
    expect(normalized).toEqual(defaultSettings);
  });

  it('resolves Japanese from the OS and supports a manual override', () => {
    expect(resolveLanguage(defaultSettings, 'ja-JP')).toBe('ja');
    expect(resolveLanguage(defaultSettings, 'de-DE')).toBe('en');
    expect(
      resolveLanguage({ ...defaultSettings, languageMode: 'manual', language: 'ja' }, 'en-US'),
    ).toBe('ja');
  });
});
