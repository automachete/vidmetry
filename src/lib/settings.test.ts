import { describe, expect, it } from 'vitest';
import {
  defaultSettings,
  loadSettings,
  normalizeSettings,
  persistSettings,
  resolveLanguage,
} from './settings';

class MemoryStorage {
  value: string | null = null;
  getItem() {
    return this.value;
  }
  setItem(_key: string, value: string) {
    this.value = value;
  }
}

describe('persistent settings', () => {
  it('round-trips export, loop, and language preferences', () => {
    const storage = new MemoryStorage();
    persistSettings(
      {
        ...defaultSettings,
        languageMode: 'manual',
        language: 'en',
        loopPlayback: true,
        export: { ...defaultSettings.export, videoCodec: 'h265', crf: 21 },
      },
      storage,
    );
    expect(loadSettings(storage)).toMatchObject({
      languageMode: 'manual',
      language: 'en',
      loopPlayback: true,
      export: { videoCodec: 'h265', crf: 21 },
    });
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
