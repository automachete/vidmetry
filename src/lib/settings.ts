import { load as loadStore } from '@tauri-apps/plugin-store';
import { z } from 'zod';

import {
  audioModes,
  encoderPresets,
  exportProfiles,
  frameRateModes,
  pixelFormats,
  videoCodecs,
  type ExportSettings,
} from './export';

const exportSettingsSchema: z.ZodType<ExportSettings> = z.strictObject({
  profile: z.enum(exportProfiles),
  videoCodec: z.enum(videoCodecs),
  crf: z.number().int().min(0).max(51),
  preset: z.enum(encoderPresets),
  pixelFormat: z.enum(pixelFormats),
  audioMode: z.enum(audioModes),
  audioBitrateKbps: z.number().int().min(32).max(1024),
  frameRateMode: z.enum(frameRateModes),
  constantFrameRate: z.number().finite().min(1).max(240),
  preserveMetadata: z.boolean(),
  copySubtitles: z.boolean(),
});

export const appSettingsSchema = z.strictObject({
  languageMode: z.enum(['system', 'manual']),
  language: z.enum(['ja', 'en']),
  loopPlayback: z.boolean(),
  explorerIntegration: z.boolean(),
  export: exportSettingsSchema,
});

export type AppSettings = z.infer<typeof appSettingsSchema>;
export type Language = AppSettings['language'];
export type LanguageMode = AppSettings['languageMode'];

export interface SettingsStore {
  get<T>(key: string): Promise<T | undefined>;
  set(key: string, value: unknown): Promise<void>;
  save(): Promise<void>;
}

const settingsFile = 'settings.json';
const settingsKey = 'settings';
let settingsStorePromise: Promise<SettingsStore> | undefined;

export const defaultSettings: AppSettings = appSettingsSchema.parse({
  languageMode: 'system',
  language: 'ja',
  loopPlayback: false,
  explorerIntegration: true,
  export: {
    profile: 'compatible',
    videoCodec: 'h264',
    crf: 17,
    preset: 'medium',
    pixelFormat: 'yuv420p',
    audioMode: 'auto',
    audioBitrateKbps: 192,
    frameRateMode: 'passthrough',
    constantFrameRate: 30,
    preserveMetadata: true,
    copySubtitles: true,
  },
});

export async function loadSettings(store?: SettingsStore): Promise<AppSettings> {
  const settingsStore = store ?? (await openSettingsStore());
  const stored = await settingsStore.get<unknown>(settingsKey);
  return stored === undefined ? cloneSettings(defaultSettings) : parseSettings(stored);
}

export async function persistSettings(
  settings: AppSettings,
  store?: SettingsStore,
): Promise<void> {
  const settingsStore = store ?? (await openSettingsStore());
  await settingsStore.set(settingsKey, parseSettings(settings));
  await settingsStore.save();
}

export function parseSettings(value: unknown): AppSettings {
  return appSettingsSchema.parse(value);
}

export function cloneSettings(settings: AppSettings): AppSettings {
  return parseSettings(settings);
}

export function resolveLanguage(settings: AppSettings, systemLanguage: string): Language {
  if (settings.languageMode === 'manual') return settings.language;
  return systemLanguage.toLowerCase().startsWith('ja') ? 'ja' : 'en';
}

async function openSettingsStore(): Promise<SettingsStore> {
  settingsStorePromise ??= loadStore(settingsFile, { autoSave: false }).catch((error) => {
    settingsStorePromise = undefined;
    throw error;
  });
  return settingsStorePromise;
}
