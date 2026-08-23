import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import { open as dialogOpen, save as dialogSave } from '@tauri-apps/plugin-dialog';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import App from './App.svelte';
import { defaultSettings } from './lib/settings';

const eventState = vi.hoisted(() => ({
  handlers: new Map<string, (event: { payload: never }) => void>(),
  failOn: undefined as string | undefined,
}));

const storeState = vi.hoisted(() => ({
  value: undefined as unknown,
  save: vi.fn().mockResolvedValue(undefined),
}));

const dragDropState = vi.hoisted(() => ({
  handler: undefined as ((event: { payload: { type: string; paths: string[] } }) => void) | undefined,
}));

const windowState = vi.hoisted(() => ({
  theme: vi.fn().mockResolvedValue('dark'),
  onThemeChanged: vi.fn().mockResolvedValue(vi.fn()),
  setFullscreen: vi.fn().mockResolvedValue(undefined),
  setTheme: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (path: string) => `asset://${path}`,
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (name: string, handler: (event: { payload: never }) => void) => {
    if (eventState.failOn === name) throw new Error(`listener failed: ${name}`);
    eventState.handlers.set(name, handler);
    return vi.fn();
  }),
}));

vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: vi.fn(async (handler) => {
      dragDropState.handler = handler;
      return vi.fn();
    }),
  }),
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => windowState,
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
  save: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-log', () => ({
  warn: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@tauri-apps/plugin-store', () => ({
  load: vi.fn(async () => ({
    get: vi.fn(async () => storeState.value),
    set: vi.fn(async (_key: string, value: unknown) => {
      storeState.value = structuredClone(value);
    }),
    save: storeState.save,
  })),
}));

class ResizeObserverStub {
  observe() {}
  disconnect() {}
  unobserve() {}
}

const videoPaths = ['C:\\clips\\a.mp4'];
const supportedVideoExtensions = [
  '3gp',
  'avi',
  'flv',
  'm2ts',
  'm4v',
  'mkv',
  'mov',
  'mp4',
  'mpeg',
  'mpg',
  'mts',
  'ogv',
  'ts',
  'vob',
  'webm',
  'wmv',
];
let mockWindowsUiLanguage: 'ja' | 'en' = 'en';

function mediaDescriptor(sourcePath: string) {
  return {
    sourcePath,
    fileName: sourcePath.slice(sourcePath.lastIndexOf('\\') + 1),
    durationSeconds: 4,
    frameCount: 120,
    codedWidth: 1280,
    codedHeight: 720,
    displayWidth: 1280,
    displayHeight: 720,
    rotationDegrees: 0,
    frameRate: '30/1',
    videoCodec: 'h264',
    pixelFormat: 'yuv420p',
    bitDepth: 8,
    hasAudio: true,
    audioCodec: 'aac',
    metadataCropSupported: true,
  };
}

function useEnglish(): void {
  storeState.value = { ...defaultSettings, languageMode: 'manual', language: 'en' };
}

function useEnglishExplorerBeta(): void {
  storeState.value = {
    ...defaultSettings,
    languageMode: 'manual',
    language: 'en',
    folderPicker: { ...defaultSettings.folderPicker, mode: 'explorerBeta' },
  };
}

function mockSelection(
  paths = videoPaths,
  failingProbePath?: string,
  startupPath: string | null = null,
  descriptorForProbe?: (path: string, invocation: number) => ReturnType<typeof mediaDescriptor>,
): void {
  let probeInvocation = 0;
  vi.mocked(invoke).mockImplementation(async (command, args) => {
    if (command === 'available_video_encoders') {
      return {
        h264: { nvidia: true, intel: false, amd: true },
        h265: { nvidia: true, intel: false, amd: false },
      } as never;
    }
    if (command === 'supported_video_extensions') {
      return supportedVideoExtensions as never;
    }
    if (command === 'windows_ui_language') return mockWindowsUiLanguage as never;
    if (command === 'pick_video_folder') {
      return { path: 'C:\\clips', viewMode: 5, iconSize: 96 } as never;
    }
    if (command === 'inspect_selection') {
      return {
        kind: paths.length > 1 ? 'directory' : 'file',
        rootPath: paths.length > 1 ? 'C:\\clips' : paths[0],
        videoPaths: paths,
      } as never;
    }
    if (command === 'probe_video') {
      const path = String((args as { path: string }).path);
      if (path === failingProbePath) throw new Error('probe failed');
      probeInvocation += 1;
      return (descriptorForProbe?.(path, probeInvocation) ?? mediaDescriptor(path)) as never;
    }
    if (command === 'system_accent_color') return '#FF8C00' as never;
    if (command === 'startup_selection') return startupPath as never;
    if (command === 'watch_directory') return undefined as never;
    if (command === 'start_export') return 'job-1' as never;
    if (command === 'reveal_in_explorer') return undefined as never;
    throw new Error(`Unexpected command: ${command}`);
  });
}

describe('application shell', () => {
  beforeEach(() => {
    storeState.value = undefined;
    dragDropState.handler = undefined;
    storeState.save.mockReset().mockResolvedValue(undefined);
    eventState.handlers.clear();
    eventState.failOn = undefined;
    mockWindowsUiLanguage = 'en';
    vi.stubGlobal('ResizeObserver', ResizeObserverStub);
    vi.spyOn(HTMLMediaElement.prototype, 'load').mockImplementation(() => undefined);
    vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => undefined);
    vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue(undefined);
    vi.mocked(invoke).mockReset();
    vi.mocked(dialogOpen).mockReset();
    vi.mocked(dialogOpen).mockImplementation(async (options) =>
      options?.directory ? 'C:\\clips' : null,
    );
    vi.mocked(dialogSave).mockReset();
    windowState.theme.mockReset().mockResolvedValue('dark');
    windowState.onThemeChanged.mockReset().mockResolvedValue(vi.fn());
    windowState.setFullscreen.mockReset().mockResolvedValue(undefined);
    windowState.setTheme.mockReset().mockResolvedValue(undefined);
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it('keeps only Settings in the launcher header and uses the concise prompt', () => {
    useEnglish();
    const { container } = render(App);

    const header = screen.getByRole('banner');
    expect(within(header).getAllByRole('button')).toHaveLength(1);
    expect(within(header).getByRole('button', { name: 'Settings' })).toBeTruthy();
    expect(screen.getByText('Drop or select a video or folder')).toBeTruthy();
    expect(screen.queryByText('Local video cropper')).toBeNull();
    expect(screen.queryByText('Keep precisely the part of the frame you need.')).toBeNull();
    expect(screen.queryByText(/WebM and more/)).toBeNull();
    expect(container.querySelector('.brand-icon')).toBeTruthy();
  });

  it('opens a file or folder passed by File Explorer at startup', async () => {
    useEnglish();
    mockSelection(['C:\\clips\\a.mp4', 'C:\\clips\\b.mp4'], undefined, 'C:\\clips');
    render(App);

    expect(await screen.findByRole('option', { name: '2. b.mp4' })).toBeTruthy();
  });

  it('uses single-level settings navigation and saves export changes immediately', async () => {
    useEnglish();
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'available_video_encoders') {
        return {
          h264: { nvidia: true, intel: false, amd: true },
          h265: { nvidia: true, intel: false, amd: false },
        } as never;
      }
      if (command === 'system_accent_color') return '#FF8C00' as never;
      if (command === 'startup_selection') return null as never;
      return undefined as never;
    });
    const { container } = render(App);
    const settingsButton = screen.getByRole('button', { name: 'Settings' });
    await waitFor(() => expect((settingsButton as HTMLButtonElement).disabled).toBe(false));
    await fireEvent.click(settingsButton);

    const settingsDialog = screen.getByRole('dialog', { name: 'Settings' });
    expect(within(settingsDialog).getByRole('heading', { name: 'Settings' })).toBeTruthy();
    expect(within(settingsDialog).getAllByText('Settings')).toHaveLength(1);
    expect(container.querySelector('.settings-dialog .section-label')).toBeNull();
    const settingsNavigation = within(settingsDialog).getByRole('navigation', {
      name: 'Settings categories',
    });
    expect(within(settingsNavigation).getAllByRole('button').map((button) => button.textContent)).toEqual([
      'Export',
      'Playback',
      'Appearance',
      'Keyboard shortcuts',
      'File Explorer',
      'Language',
    ]);
    expect(screen.getByText('Compatible MP4')).toBeTruthy();
    expect(screen.queryByText('Export video')).toBeNull();
    expect(screen.queryByText('Optimize for web playback (faststart)')).toBeNull();
    const encoder = screen.getByRole('combobox', { name: 'Encoder' });
    expect(within(encoder).getByRole('option', { name: 'Automatic' }).textContent).toBe(
      'Automatic',
    );
    expect(
      (within(encoder).getByRole('option', { name: 'nvenc' }) as HTMLOptionElement).disabled,
    ).toBe(false);
    expect(
      (within(encoder).getByRole('option', { name: 'qsv' }) as HTMLOptionElement).disabled,
    ).toBe(true);
    expect(
      (within(encoder).getByRole('option', { name: 'amf' }) as HTMLOptionElement).disabled,
    ).toBe(false);
    expect(within(encoder).getByRole('option', { name: 'libx264' })).toBeTruthy();
    await fireEvent.change(encoder, { target: { value: 'amd' } });
    await waitFor(() => expect(storeState.value).toMatchObject({ export: { encoder: 'amd' } }));
    expect(container.querySelector('.settings-save-status')).toBeNull();
    await fireEvent.click(within(settingsNavigation).getByRole('button', { name: 'Language' }));
    expect(screen.getByRole('combobox', { name: 'Display language' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Apply' })).toBeNull();
  });

  it('toggles File Explorer integration and persists it with common settings', async () => {
    useEnglish();
    render(App);

    const settingsButton = screen.getByRole('button', { name: 'Settings' });
    await waitFor(() => expect((settingsButton as HTMLButtonElement).disabled).toBe(false));
    await fireEvent.click(settingsButton);
    await fireEvent.click(screen.getByRole('button', { name: 'File Explorer' }));
    const folderPicker = screen.getByRole('combobox', { name: 'Folder picker' });
    expect((folderPicker as HTMLSelectElement).value).toBe('standard');
    expect(within(folderPicker).getByRole('option', { name: 'Windows standard' })).toBeTruthy();
    expect(within(folderPicker).getByRole('option', { name: 'Show video files' })).toBeTruthy();
    expect(
      screen.queryByText('You can view supported videos in the folder (Beta).'),
    ).toBeNull();
    await fireEvent.change(folderPicker, { target: { value: 'explorerBeta' } });
    await waitFor(() =>
      expect(storeState.value).toMatchObject({ folderPicker: { mode: 'explorerBeta' } }),
    );
    expect(screen.getByText('You can view supported videos in the folder (Beta).')).toBeTruthy();
    await fireEvent.click(
      screen.getByRole('checkbox', { name: 'Show Open with Vidmetry for folders' }),
    );

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('set_explorer_integration', { enabled: false }),
    );
    await waitFor(() => expect(storeState.value).toMatchObject({ explorerIntegration: false }));
  });

  it('applies custom mode and Fluent accent choices as soon as they change', async () => {
    useEnglish();
    render(App);

    const settingsButton = screen.getByRole('button', { name: 'Settings' });
    await waitFor(() => expect((settingsButton as HTMLButtonElement).disabled).toBe(false));
    await fireEvent.click(settingsButton);
    await fireEvent.click(screen.getByRole('button', { name: 'Appearance' }));

    const appearanceGroups = document.querySelectorAll<HTMLElement>('.appearance-group');
    await fireEvent.click(within(appearanceGroups[0]).getByRole('radio', { name: 'Customize' }));
    await fireEvent.change(screen.getByRole('combobox', { name: 'App mode' }), {
      target: { value: 'light' },
    });
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe('light'));
    await waitFor(() => expect(windowState.setTheme).toHaveBeenLastCalledWith('light'));

    await fireEvent.click(within(appearanceGroups[1]).getByRole('radio', { name: 'Customize' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Open color palette' }));
    await fireEvent.click(screen.getByRole('option', { name: 'Purple' }));
    await waitFor(() =>
      expect(storeState.value).toMatchObject({
        appearance: { themeMode: 'manual', theme: 'light', accentMode: 'manual', accentColor: 'purple' },
      }),
    );
    expect(document.documentElement.style.getPropertyValue('--accent')).toBe('#5C2E91');
  });

  it('records a shortcut, rejects a duplicate, and uses the saved shortcut on the launcher', async () => {
    useEnglish();
    vi.mocked(dialogOpen).mockResolvedValue(null);
    render(App);

    const settingsButton = screen.getByRole('button', { name: 'Settings' });
    await waitFor(() => expect((settingsButton as HTMLButtonElement).disabled).toBe(false));
    await fireEvent.click(settingsButton);
    await fireEvent.click(screen.getByRole('button', { name: 'Keyboard shortcuts' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Change shortcut: Open video' }));
    await fireEvent.keyDown(window, { code: 'KeyP', key: 'p', ctrlKey: true });
    await waitFor(() =>
      expect(storeState.value).toMatchObject({ shortcuts: { openVideo: 'Ctrl+KeyP' } }),
    );
    expect(
      screen.getByRole('button', { name: 'Change shortcut: Save a copy' }),
    ).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Change shortcut: Play / pause' }));
    await fireEvent.keyDown(window, { code: 'KeyK', key: 'k' });
    await waitFor(() =>
      expect(storeState.value).toMatchObject({ shortcuts: { playPause: 'KeyK' } }),
    );

    await fireEvent.click(screen.getByRole('button', { name: 'Change shortcut: Open folder' }));
    await fireEvent.keyDown(window, { code: 'KeyP', key: 'p', ctrlKey: true });
    expect(screen.getByRole('alert').textContent).toContain('Already used by “Open video”.');

    await fireEvent.keyDown(window, { code: 'Escape', key: 'Escape' });
    await fireEvent.click(
      within(screen.getByRole('dialog', { name: 'Settings' })).getAllByRole('button', {
        name: 'Close',
      })[1],
    );
    expect(screen.getByRole('button', { name: 'Open video' }).getAttribute('title')).toBe(
      'Open video (Ctrl+P)',
    );
    mockSelection();
    vi.mocked(dialogOpen).mockResolvedValue(videoPaths[0]);
    await fireEvent.keyDown(window, { code: 'KeyP', key: 'p', ctrlKey: true });
    await screen.findByText('a.mp4');
    expect(screen.getByRole('button', { name: 'Play' }).getAttribute('title')).toBe('Play (K)');
    await fireEvent.keyDown(document.body, { code: 'KeyK', key: 'k' });
    await waitFor(() => expect(HTMLMediaElement.prototype.play).toHaveBeenCalledOnce());
  });

  it('opens Settings and folders with conventional shortcuts, uses the Windows dialog language, and switches export profiles', async () => {
    useEnglishExplorerBeta();
    mockWindowsUiLanguage = 'ja';
    vi.mocked(dialogOpen).mockResolvedValue(videoPaths[0]);
    mockSelection();
    render(App);

    const settingsButton = screen.getByRole('button', { name: 'Settings' });
    await waitFor(() => expect((settingsButton as HTMLButtonElement).disabled).toBe(false));
    await fireEvent.keyDown(window, { code: 'Comma', key: ',', ctrlKey: true });
    const settingsDialog = screen.getByRole('dialog', { name: 'Settings' });
    expect(settingsDialog).toBeTruthy();
    await fireEvent.click(within(settingsDialog).getAllByRole('button', { name: 'Close' })[1]);

    await fireEvent.keyDown(window, { code: 'KeyO', key: 'o', ctrlKey: true });
    expect(await screen.findByText('a.mp4')).toBeTruthy();
    await fireEvent.keyDown(window, { code: 'Digit2', key: '2', altKey: true });
    await waitFor(() => expect(screen.getByText('Export: Lossless FFV1 / MKV')).toBeTruthy());
    await fireEvent.keyDown(window, { code: 'Digit3', key: '3', altKey: true });
    await waitFor(() => expect(screen.getByText('Export: Metadata only')).toBeTruthy());

    vi.mocked(dialogOpen).mockClear().mockResolvedValue(null);
    await fireEvent.keyDown(window, { code: 'KeyO', key: 'o', ctrlKey: true, shiftKey: true });
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('pick_video_folder', {
        title: 'フォルダーの選択',
        selectFolderLabel: 'フォルダーの選択',
        cancelLabel: 'キャンセル',
        initialDirectory: 'C:\\clips',
        initialView: { viewMode: 1, iconSize: 96 },
        viewLabels: {
          view: '表示',
          extraLargeIcons: '特大アイコン',
          largeIcons: '大アイコン',
          mediumIcons: '中アイコン',
          smallIcons: '小アイコン',
          list: '一覧',
          details: '詳細',
          tiles: '並べて表示',
          content: 'コンテンツ',
        },
      }),
    );
  });

  it('uses the standard Windows folder picker by default and remembers its selection', async () => {
    useEnglish();
    mockSelection();
    vi.mocked(dialogOpen).mockResolvedValue('D:\\Standard');
    render(App);

    const openFolder = screen.getByRole('button', { name: 'Open folder' });
    await waitFor(() => expect((openFolder as HTMLButtonElement).disabled).toBe(false));
    await fireEvent.click(openFolder);

    expect(dialogOpen).toHaveBeenCalledWith({
      multiple: false,
      directory: true,
      defaultPath: undefined,
    });
    expect(invoke).not.toHaveBeenCalledWith('pick_video_folder', expect.anything());
    await waitFor(() =>
      expect(storeState.value).toMatchObject({
        folderPicker: { mode: 'standard', lastPath: 'D:\\Standard' },
      }),
    );
  });

  it('restores and updates the last confirmed folder and Explorer view across app sessions', async () => {
    storeState.value = {
      ...defaultSettings,
      languageMode: 'manual',
      language: 'en',
      folderPicker: {
        mode: 'explorerBeta',
        lastPath: 'D:\\Remembered',
        viewMode: 4,
        iconSize: 24,
      },
    };
    mockSelection();
    vi.mocked(invoke).mockImplementation(async (command, args) => {
      if (command === 'available_video_encoders') {
        return {
          h264: { nvidia: true, intel: false, amd: true },
          h265: { nvidia: true, intel: false, amd: false },
        } as never;
      }
      if (command === 'system_accent_color') return '#FF8C00' as never;
      if (command === 'startup_selection') return null as never;
      if (command === 'windows_ui_language') return 'en' as never;
      if (command === 'pick_video_folder') {
        expect(args).toMatchObject({
          initialDirectory: 'D:\\Remembered',
          initialView: { viewMode: 4, iconSize: 24 },
        });
        return { path: 'E:\\Selected', viewMode: 1, iconSize: 144 } as never;
      }
      if (command === 'inspect_selection') {
        return {
          kind: 'directory',
          rootPath: 'E:\\Selected',
          videoPaths: ['E:\\Selected\\a.mp4'],
        } as never;
      }
      if (command === 'probe_video') return mediaDescriptor('E:\\Selected\\a.mp4') as never;
      if (command === 'watch_directory') return undefined as never;
      throw new Error(`Unexpected command: ${command}`);
    });
    render(App);

    const settingsButton = screen.getByRole('button', { name: 'Settings' });
    await waitFor(() => expect((settingsButton as HTMLButtonElement).disabled).toBe(false));
    const openFolder = screen.getByRole('button', { name: 'Open folder' });
    await fireEvent.click(openFolder);

    await waitFor(() =>
      expect(storeState.value).toMatchObject({
        folderPicker: {
          mode: 'explorerBeta',
          lastPath: 'E:\\Selected',
          viewMode: 1,
          iconSize: 144,
        },
      }),
    );
    expect(storeState.save).toHaveBeenCalled();
  });

  it('keeps settings open when File Explorer integration cannot be changed', async () => {
    useEnglish();
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'startup_selection') return null as never;
      if (command === 'system_accent_color') return '#FF8C00' as never;
      if (command === 'set_explorer_integration') {
        throw { code: 'explorer_integration_update_failed', detail: 'access denied' };
      }
      return undefined as never;
    });
    render(App);

    const settingsButton = screen.getByRole('button', { name: 'Settings' });
    await waitFor(() => expect((settingsButton as HTMLButtonElement).disabled).toBe(false));
    await fireEvent.click(settingsButton);
    await fireEvent.click(screen.getByRole('button', { name: 'File Explorer' }));
    await fireEvent.click(
      screen.getByRole('checkbox', { name: 'Show Open with Vidmetry for folders' }),
    );

    expect((await screen.findByRole('alert')).textContent).toContain(
      'File Explorer integration could not be changed. (access denied)',
    );
    expect(screen.getByRole('dialog', { name: 'Settings' })).toBeTruthy();
    expect(storeState.save).not.toHaveBeenCalled();
  });

  it('applies Japanese UI and persists disabling loop playback', async () => {
    storeState.value = { ...defaultSettings, languageMode: 'manual', language: 'ja' };
    render(App);

    const settingsButton = await screen.findByRole('button', { name: '共通設定' });
    await waitFor(() => expect((settingsButton as HTMLButtonElement).disabled).toBe(false));
    await fireEvent.click(settingsButton);
    await fireEvent.click(screen.getByRole('button', { name: '再生' }));
    await fireEvent.click(screen.getByRole('checkbox', { name: 'ループ再生を有効化' }));

    await waitFor(() => expect(storeState.save).toHaveBeenCalledOnce());
    expect(storeState.value).toMatchObject({
      languageMode: 'manual',
      language: 'ja',
      loopPlayback: false,
    });
  });

  it('shows the Japanese Beta folder picker description only while it is selected', async () => {
    storeState.value = { ...defaultSettings, languageMode: 'manual', language: 'ja' };
    render(App);

    const settingsButton = await screen.findByRole('button', { name: '共通設定' });
    await waitFor(() => expect((settingsButton as HTMLButtonElement).disabled).toBe(false));
    await fireEvent.click(settingsButton);
    await fireEvent.click(screen.getByRole('button', { name: 'エクスプローラー' }));

    const folderPicker = screen.getByRole('combobox', { name: 'フォルダー選択方式' });
    expect(within(folderPicker).getByRole('option', { name: 'Windows標準' })).toBeTruthy();
    expect(within(folderPicker).getByRole('option', { name: '動画ファイルを表示' })).toBeTruthy();
    expect(screen.queryByText('フォルダー内の対応動画を確認できます（Beta）')).toBeNull();

    await fireEvent.change(folderPicker, { target: { value: 'explorerBeta' } });
    expect(screen.getByText('フォルダー内の対応動画を確認できます（Beta）')).toBeTruthy();
  });

  it('navigates a folder and requires two clicks to save when extensions match', async () => {
    useEnglish();
    const paths = ['C:\\clips\\a.mp4', 'C:\\clips\\b.mp4'];
    vi.mocked(dialogSave).mockResolvedValue(null);
    mockSelection(paths);
    const { container } = render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open folder' }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('inspect_selection', { path: 'C:\\clips' }),
    );
    expect(await screen.findByRole('option', { name: '2. b.mp4' })).toBeTruthy();
    await waitFor(() =>
      expect((screen.getByRole('button', { name: 'Next video' }) as HTMLButtonElement).disabled).toBe(
        false,
      ),
    );
    expect(container.querySelectorAll('.playlist-nav svg')).toHaveLength(2);

    await fireEvent.keyDown(document.body, { key: 'PageDown', code: 'PageDown' });
    await waitFor(() => expect(screen.getByText('b.mp4')).toBeTruthy());
    await fireEvent.click(screen.getByRole('button', { name: 'Save options' }));
    expect(dialogSave).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Save a copy' }));

    expect(dialogSave).toHaveBeenCalledOnce();
  });

  it('keeps the current playlist position when the next video cannot be probed', async () => {
    useEnglish();
    const paths = ['C:\\clips\\a.mp4', 'C:\\clips\\b.mp4'];
    mockSelection(paths, paths[1]);
    render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open folder' }));
    await screen.findByText('a.mp4');
    const nextButton = screen.getByRole('button', { name: 'Next video' });
    await waitFor(() => expect((nextButton as HTMLButtonElement).disabled).toBe(false));
    await fireEvent.click(nextButton);

    expect((await screen.findByRole('alert')).textContent).toContain('probe failed');
    expect(screen.getByText('a.mp4')).toBeTruthy();
    expect((screen.getByRole('combobox', { name: 'Videos in folder' }) as HTMLSelectElement).value).toBe(
      '0',
    );
  });

  it('continues playback after moving between videos in a selected directory', async () => {
    useEnglish();
    const paths = ['C:\\clips\\a.mp4', 'C:\\clips\\b.mp4'];
    mockSelection(paths);
    render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open folder' }));
    await screen.findByText('a.mp4');
    const video = document.querySelector('video') as HTMLVideoElement;
    await fireEvent.play(video);
    await fireEvent.click(screen.getByRole('button', { name: 'Next video' }));
    await screen.findByText('b.mp4');
    await fireEvent.canPlay(video);

    expect(HTMLMediaElement.prototype.play).toHaveBeenCalledOnce();
  });

  it('validates every path in a multi-file drop through the backend selection boundary', async () => {
    useEnglish();
    const paths = ['C:\\clips\\a.mp4', 'C:\\clips\\b.mp4'];
    mockSelection(paths);
    render(App);
    await waitFor(() => expect(dragDropState.handler).toBeTypeOf('function'));

    dragDropState.handler?.({ payload: { type: 'drop', paths } });

    await screen.findByText('a.mp4');
    expect(invoke).toHaveBeenCalledWith('inspect_selection', { path: paths[0] });
    expect(invoke).toHaveBeenCalledWith('inspect_selection', { path: paths[1] });
  });

  it('shows a direct non-expandable Save a copy button when extensions differ', async () => {
    useEnglish();
    vi.mocked(dialogOpen).mockResolvedValue('C:\\clips\\source.mov');
    vi.mocked(dialogSave).mockResolvedValue(null);
    mockSelection(['C:\\clips\\source.mov']);
    render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open video' }));
    expect(await screen.findByText('source.mov')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Save options' })).toBeNull();
    const saveCopy = screen.getByRole('button', { name: 'Save a copy' });
    expect(saveCopy.getAttribute('aria-haspopup')).toBeNull();
    await fireEvent.click(saveCopy);
    expect(dialogSave).toHaveBeenCalledOnce();
  });

  it('toggles playback with Space outside form controls', async () => {
    useEnglish();
    vi.mocked(dialogOpen).mockResolvedValue(videoPaths[0]);
    mockSelection();
    render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open video' }));
    await screen.findByText('a.mp4');
    await fireEvent.keyDown(document.body, { key: ' ', code: 'Space' });

    expect(HTMLMediaElement.prototype.play).toHaveBeenCalledOnce();
  });

  it('toggles playback when the playback-position scrubber has focus', async () => {
    useEnglish();
    vi.mocked(dialogOpen).mockResolvedValue(videoPaths[0]);
    mockSelection();
    render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open video' }));
    const playbackScrubber = await screen.findByRole('slider', { name: 'Playback-position handle' });
    await fireEvent.pointerDown(playbackScrubber, { button: 0, clientX: 0, clientY: 0 });
    await fireEvent.pointerUp(window, { button: 0, clientX: 0, clientY: 0 });
    expect(document.activeElement).toBe(playbackScrubber);
    await fireEvent.keyDown(playbackScrubber, { key: ' ', code: 'Space' });

    expect(HTMLMediaElement.prototype.play).toHaveBeenCalledOnce();
  });

  it('localizes structured backend errors in English mode', async () => {
    useEnglishExplorerBeta();
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'windows_ui_language') return 'en' as never;
      if (command === 'pick_video_folder') {
        return { path: 'C:\\blocked', viewMode: 5, iconSize: 96 } as never;
      }
      throw { code: 'folder_read_failed', detail: 'access denied' };
    });
    render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open folder' }));
    expect((await screen.findByRole('alert')).textContent).toContain(
      'The folder could not be read. (access denied)',
    );
  });

  it('localizes structured asynchronous export failures', async () => {
    useEnglish();
    vi.mocked(dialogOpen).mockResolvedValue(videoPaths[0]);
    vi.mocked(dialogSave).mockResolvedValue('C:\\clips\\a_cropped.mp4');
    mockSelection();
    render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open video' }));
    await screen.findByText('a.mp4');
    await fireEvent.click(screen.getByRole('button', { name: 'Save options' }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Save a copy' }));
    await waitFor(() => expect(eventState.handlers.has('export-error')).toBe(true));
    eventState.handlers.get('export-error')?.({
      payload: {
        jobId: 'job-1',
        error: { code: 'export_process_failed', detail: 'exit code 1' },
        cancelled: false,
      } as never,
    });

    expect((await screen.findByRole('alert')).textContent).toContain(
      'Export failed. The export process did not complete. (exit code 1)',
    );
  });

  it('disables saving and reports an error when export event registration is incomplete', async () => {
    useEnglish();
    eventState.failOn = 'export-complete';
    vi.mocked(dialogOpen).mockResolvedValue(videoPaths[0]);
    mockSelection();
    render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open video' }));
    await screen.findByText('a.mp4');

    expect((await screen.findByRole('alert')).textContent).toContain(
      'Saving is unavailable because export notifications could not be initialized.',
    );
    expect((screen.getByRole('button', { name: 'Save options' }) as HTMLButtonElement).disabled).toBe(
      true,
    );
  });

  it('keeps settings open and reports a durable-store save failure', async () => {
    useEnglish();
    storeState.save.mockRejectedValueOnce(new Error('disk unavailable'));
    render(App);

    const settingsButton = screen.getByRole('button', { name: 'Settings' });
    await waitFor(() => expect((settingsButton as HTMLButtonElement).disabled).toBe(false));
    await fireEvent.click(settingsButton);
    await fireEvent.click(screen.getByRole('button', { name: 'Playback' }));
    await fireEvent.click(screen.getByRole('checkbox', { name: 'Enable loop playback' }));

    expect((await screen.findByRole('alert')).textContent).toContain(
      'Could not save settings. (disk unavailable)',
    );
    expect(screen.getByRole('dialog', { name: 'Settings' })).toBeTruthy();
  });

  it('reports a rejected media play request instead of leaving an unhandled promise', async () => {
    useEnglish();
    vi.mocked(dialogOpen).mockResolvedValue(videoPaths[0]);
    mockSelection();
    vi.mocked(HTMLMediaElement.prototype.play).mockRejectedValueOnce(new Error('decoder busy'));
    render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open video' }));
    await screen.findByText('a.mp4');
    await fireEvent.keyDown(document.body, { key: ' ', code: 'Space' });

    expect((await screen.findByRole('alert')).textContent).toContain(
      'Could not play the video. (decoder busy)',
    );
  });

  it('links the completed output to Explorer file selection', async () => {
    useEnglish();
    vi.mocked(dialogOpen).mockResolvedValue(videoPaths[0]);
    vi.mocked(dialogSave).mockResolvedValue('C:\\clips\\a_cropped.mp4');
    mockSelection();
    render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open video' }));
    await screen.findByText('a.mp4');
    await fireEvent.click(screen.getByRole('button', { name: 'Save options' }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Save a copy' }));
    await waitFor(() => expect(eventState.handlers.has('export-complete')).toBe(true));
    eventState.handlers.get('export-complete')?.({
      payload: { jobId: 'job-1', outputPath: 'C:\\clips\\a_cropped.mp4' } as never,
    });

    const savedLink = await screen.findByRole('link', {
      name: 'Show saved file in File Explorer: C:\\clips\\a_cropped.mp4',
    });
    expect(screen.getByRole('status').textContent).toContain('Saved:');
    await fireEvent.click(savedLink);
    expect(invoke).toHaveBeenCalledWith('reveal_in_explorer', {
      path: 'C:\\clips\\a_cropped.mp4',
    });
  });

  it('refreshes an open folder after Explorer additions and completed copy saves', async () => {
    useEnglish();
    const paths = ['C:\\clips\\a.mp4', 'C:\\clips\\b.mp4'];
    vi.mocked(dialogSave).mockResolvedValue('C:\\clips\\d-copy.mp4');
    mockSelection(paths);
    render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open folder' }));
    await screen.findByRole('option', { name: '2. b.mp4' });
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('watch_directory', { path: 'C:\\clips' }),
    );

    paths.push('C:\\clips\\c-external.mp4');
    await waitFor(() => expect(eventState.handlers.has('directory-changed')).toBe(true));
    eventState.handlers.get('directory-changed')?.({
      payload: { rootPath: 'c:\\CLIPS' } as never,
    });
    expect(await screen.findByRole('option', { name: '3. c-external.mp4' })).toBeTruthy();

    await fireEvent.click(screen.getByRole('button', { name: 'Save options' }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Save a copy' }));
    await screen.findByRole('button', { name: 'Cancel' });
    paths.push('C:\\clips\\d-copy.mp4');
    eventState.handlers.get('export-complete')?.({
      payload: { jobId: 'job-1', outputPath: 'C:\\clips\\d-copy.mp4' } as never,
    });

    expect(await screen.findByRole('option', { name: '4. d-copy.mp4' })).toBeTruthy();
  });

  it('reloads an overwritten source with fresh media geometry and timing', async () => {
    useEnglish();
    vi.mocked(dialogOpen).mockResolvedValue(videoPaths[0]);
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    mockSelection(videoPaths, undefined, null, (path, invocation) =>
      invocation === 1
        ? mediaDescriptor(path)
        : {
            ...mediaDescriptor(path),
            durationSeconds: 2,
            frameCount: 60,
            codedWidth: 640,
            codedHeight: 360,
            displayWidth: 640,
            displayHeight: 360,
          },
    );
    render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open video' }));
    await screen.findByText('a.mp4');
    const video = document.querySelector('video') as HTMLVideoElement;
    const initialSource = video.getAttribute('src');
    expect(initialSource).toContain('vidmetryRevision=1');

    await fireEvent.click(screen.getByRole('button', { name: 'Save options' }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Save' }));
    await waitFor(() => expect(eventState.handlers.has('export-complete')).toBe(true));
    eventState.handlers.get('export-complete')?.({
      payload: { jobId: 'job-1', outputPath: videoPaths[0] } as never,
    });

    expect((await screen.findAllByText(/640 × 360/)).length).toBeGreaterThan(0);
    expect(
      screen.getByRole('slider', { name: 'End trim-boundary handle' }).getAttribute('aria-valuenow'),
    ).toBe('60');
    await waitFor(() => expect(video.getAttribute('src')).toContain('vidmetryRevision=2'));
    expect(video.getAttribute('src')).not.toBe(initialSource);
  });

  it('adjusts the time range by one frame and sends the exclusive range to export', async () => {
    useEnglish();
    vi.mocked(dialogOpen).mockResolvedValue(videoPaths[0]);
    vi.mocked(dialogSave).mockResolvedValue('C:\\clips\\a_cropped.mp4');
    mockSelection();
    render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open video' }));
    await screen.findByText('a.mp4');
    const startHandle = screen.getByRole('slider', { name: 'Start trim-boundary handle' });
    const endHandle = screen.getByRole('slider', { name: 'End trim-boundary handle' });
    expect(startHandle.getAttribute('aria-valuenow')).toBe('0');
    expect(endHandle.getAttribute('aria-valuenow')).toBe('120');

    await fireEvent.pointerDown(startHandle, { button: 0, clientX: 0, clientY: 0 });
    await fireEvent.pointerUp(window, { button: 0, clientX: 0, clientY: 0 });
    expect(document.activeElement).toBe(startHandle);
    await fireEvent.keyDown(startHandle, { key: 'ArrowRight', code: 'ArrowRight' });
    expect(startHandle.getAttribute('aria-valuenow')).toBe('1');
    await fireEvent.keyDown(startHandle, { key: 'ArrowRight', code: 'ArrowRight', shiftKey: true });
    expect(startHandle.getAttribute('aria-valuenow')).toBe('11');
    await fireEvent.pointerDown(endHandle, { button: 0, clientX: 0, clientY: 0 });
    await fireEvent.pointerUp(window, { button: 0, clientX: 0, clientY: 0 });
    expect(document.activeElement).toBe(endHandle);
    await fireEvent.keyDown(endHandle, { key: 'ArrowLeft', code: 'ArrowLeft' });
    expect(endHandle.getAttribute('aria-valuenow')).toBe('119');
    await fireEvent.keyDown(endHandle, { key: 'ArrowLeft', code: 'ArrowLeft', shiftKey: true });
    expect(endHandle.getAttribute('aria-valuenow')).toBe('109');
    await fireEvent.click(screen.getByRole('button', { name: 'Save options' }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Save a copy' }));

    expect((startHandle as HTMLButtonElement).disabled).toBe(true);
    expect((endHandle as HTMLButtonElement).disabled).toBe(true);
    expect(startHandle.getAttribute('aria-valuenow')).toBe('11');
    expect(endHandle.getAttribute('aria-valuenow')).toBe('109');
    expect(invoke).toHaveBeenCalledWith(
      'start_export',
      expect.objectContaining({
        request: expect.objectContaining({ trim: { startFrame: 11, endFrame: 109 } }),
      }),
    );
  });

  it('uses Ctrl+S for a copy and Ctrl+Shift+S for in-place save', async () => {
    useEnglish();
    vi.mocked(dialogOpen).mockResolvedValue(videoPaths[0]);
    vi.mocked(dialogSave).mockResolvedValue(null);
    mockSelection();
    render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open video' }));
    await screen.findByText('a.mp4');
    await fireEvent.keyDown(document.body, { key: 's', code: 'KeyS', ctrlKey: true });
    expect(dialogSave).toHaveBeenCalledOnce();

    vi.spyOn(window, 'confirm').mockReturnValue(true);
    await fireEvent.keyDown(document.body, {
      key: 'S',
      code: 'KeyS',
      ctrlKey: true,
      shiftKey: true,
    });
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        'start_export',
        expect.objectContaining({ request: expect.objectContaining({ inPlace: true }) }),
      ),
    );
  });

  it('closes both editor panes and toggles fullscreen preview with F11', async () => {
    useEnglish();
    vi.mocked(dialogOpen).mockResolvedValue(videoPaths[0]);
    mockSelection();
    const { container } = render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open video' }));
    await screen.findByText('a.mp4');
    await fireEvent.click(screen.getByRole('button', { name: 'Close crop details' }));
    expect(screen.getByRole('button', { name: 'Open crop details' })).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Close playback and trimming' }));
    expect(screen.getByRole('button', { name: 'Open playback and trimming' })).toBeTruthy();

    await fireEvent.keyDown(document.body, { key: 'F11', code: 'F11' });
    await waitFor(() => expect(windowState.setFullscreen).toHaveBeenCalledWith(true));
    expect(container.querySelector('.app-shell')?.classList.contains('video-fullscreen')).toBe(true);
    await fireEvent.keyDown(document.body, { key: 'F11', code: 'F11' });
    await waitFor(() => expect(windowState.setFullscreen).toHaveBeenLastCalledWith(false));
  });

  it('projects the Windows mode and accent to the document root', async () => {
    useEnglish();
    mockSelection();
    render(App);

    await waitFor(() => expect(document.documentElement.dataset.theme).toBe('dark'));
    await waitFor(() =>
      expect(document.documentElement.style.getPropertyValue('--accent')).toBe('#FF8C00'),
    );
  });

  it('dismisses the save notice when another control is used', async () => {
    useEnglish();
    vi.mocked(dialogOpen).mockResolvedValue(videoPaths[0]);
    vi.mocked(dialogSave).mockResolvedValue('C:\\clips\\a_cropped.mp4');
    mockSelection();
    render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open video' }));
    await screen.findByText('a.mp4');
    await fireEvent.click(screen.getByRole('button', { name: 'Save options' }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Save a copy' }));
    await waitFor(() => expect(eventState.handlers.has('export-complete')).toBe(true));
    eventState.handlers.get('export-complete')?.({
      payload: { jobId: 'job-1', outputPath: 'C:\\clips\\a_cropped.mp4' } as never,
    });
    expect(await screen.findByRole('status')).toBeTruthy();

    await fireEvent.pointerDown(screen.getByRole('button', { name: 'Disable loop playback' }));
    expect(screen.queryByRole('status')).toBeNull();
  });
});
