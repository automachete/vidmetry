import type {
  AudioMode,
  EncoderPreset,
  ExportProfile,
  ExportSettings,
  FrameRateMode,
  PixelFormat,
  VideoCodec,
} from './export';
import { load as loadStore } from '@tauri-apps/plugin-store';

export type Language = 'ja' | 'en';
export type LanguageMode = 'system' | 'manual';

export interface AppSettings {
  version: 1;
  languageMode: LanguageMode;
  language: Language;
  loopPlayback: boolean;
  export: ExportSettings;
}

export interface SettingsStore {
  get<T>(key: string): Promise<T | undefined>;
  set(key: string, value: unknown): Promise<void>;
  save(): Promise<void>;
}

const settingsFile = 'settings.json';
const settingsKey = 'settings';
let settingsStorePromise: Promise<SettingsStore> | undefined;

export const defaultSettings: AppSettings = {
  version: 1,
  languageMode: 'system',
  language: 'ja',
  loopPlayback: false,
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
    fastStart: true,
    preserveMetadata: true,
    copySubtitles: true,
  },
};

const profiles: ExportProfile[] = ['compatible', 'lossless', 'metadata'];
const videoCodecs: VideoCodec[] = ['h264', 'h265'];
const presets: EncoderPreset[] = [
  'ultrafast',
  'superfast',
  'veryfast',
  'faster',
  'fast',
  'medium',
  'slow',
  'slower',
  'veryslow',
];
const pixelFormats: PixelFormat[] = [
  'source',
  'yuv420p',
  'yuv420p10le',
  'yuv422p',
  'yuv422p10le',
  'yuv444p',
  'yuv444p10le',
];
const audioModes: AudioMode[] = ['auto', 'copy', 'aac', 'flac', 'pcm', 'none'];
const frameRateModes: FrameRateMode[] = ['passthrough', 'constant'];

export async function loadSettings(
  store?: SettingsStore,
): Promise<AppSettings> {
  const settingsStore = store ?? (await openSettingsStore());
  const stored = await settingsStore.get<unknown>(settingsKey);
  return stored === undefined ? cloneSettings(defaultSettings) : normalizeSettings(stored);
}

export async function persistSettings(
  settings: AppSettings,
  store?: SettingsStore,
): Promise<void> {
  const settingsStore = store ?? (await openSettingsStore());
  await settingsStore.set(settingsKey, normalizeSettings(settings));
  await settingsStore.save();
}

export function cloneSettings(settings: AppSettings): AppSettings {
  return { ...settings, export: { ...settings.export } };
}

export function resolveLanguage(settings: AppSettings, systemLanguage: string): Language {
  if (settings.languageMode === 'manual') return settings.language;
  return systemLanguage.toLowerCase().startsWith('ja') ? 'ja' : 'en';
}

export function normalizeSettings(value: unknown): AppSettings {
  const candidate = isRecord(value) ? value : {};
  const exportCandidate = isRecord(candidate.export) ? candidate.export : {};
  return {
    version: 1,
    languageMode: oneOf(candidate.languageMode, ['system', 'manual'], 'system'),
    language: oneOf(candidate.language, ['ja', 'en'], 'ja'),
    loopPlayback: booleanValue(candidate.loopPlayback, false),
    export: {
      profile: oneOf(exportCandidate.profile, profiles, 'compatible'),
      videoCodec: oneOf(exportCandidate.videoCodec, videoCodecs, 'h264'),
      crf: integerInRange(exportCandidate.crf, 0, 51, 17),
      preset: oneOf(exportCandidate.preset, presets, 'medium'),
      pixelFormat: oneOf(exportCandidate.pixelFormat, pixelFormats, 'yuv420p'),
      audioMode: oneOf(exportCandidate.audioMode, audioModes, 'auto'),
      audioBitrateKbps: integerInRange(exportCandidate.audioBitrateKbps, 32, 1024, 192),
      frameRateMode: oneOf(exportCandidate.frameRateMode, frameRateModes, 'passthrough'),
      constantFrameRate: numberInRange(exportCandidate.constantFrameRate, 1, 240, 30),
      fastStart: booleanValue(exportCandidate.fastStart, true),
      preserveMetadata: booleanValue(exportCandidate.preserveMetadata, true),
      copySubtitles: booleanValue(exportCandidate.copySubtitles, true),
    },
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function oneOf<T extends string>(value: unknown, choices: readonly T[], fallback: T): T {
  return typeof value === 'string' && choices.includes(value as T) ? (value as T) : fallback;
}

function integerInRange(value: unknown, minimum: number, maximum: number, fallback: number): number {
  return typeof value === 'number' && Number.isInteger(value) && value >= minimum && value <= maximum
    ? value
    : fallback;
}

function numberInRange(value: unknown, minimum: number, maximum: number, fallback: number): number {
  return typeof value === 'number' && Number.isFinite(value) && value >= minimum && value <= maximum
    ? value
    : fallback;
}

function booleanValue(value: unknown, fallback: boolean): boolean {
  return typeof value === 'boolean' ? value : fallback;
}

async function openSettingsStore(): Promise<SettingsStore> {
  settingsStorePromise ??= loadStore(settingsFile, { autoSave: false }).catch((error) => {
    settingsStorePromise = undefined;
    throw error;
  });
  return settingsStorePromise;
}
