import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import { open as dialogOpen, save as dialogSave } from '@tauri-apps/plugin-dialog';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import App from './App.svelte';
import { defaultSettings, persistSettings } from './lib/settings';

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (path: string) => `asset://${path}`,
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(vi.fn()),
}));

vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: () => ({ onDragDropEvent: vi.fn().mockResolvedValue(vi.fn()) }),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
  save: vi.fn(),
}));

class ResizeObserverStub {
  observe() {}
  disconnect() {}
  unobserve() {}
}

describe('application shell', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.stubGlobal('ResizeObserver', ResizeObserverStub);
    vi.spyOn(HTMLMediaElement.prototype, 'load').mockImplementation(() => undefined);
    vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => undefined);
    vi.mocked(invoke).mockReset();
    vi.mocked(dialogOpen).mockReset();
    vi.mocked(dialogSave).mockReset();
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it('opens common settings without an intermediate export screen', async () => {
    render(App);
    await fireEvent.click(screen.getByRole('button', { name: 'Settings' }));

    expect(screen.getByRole('heading', { name: 'Settings' })).toBeTruthy();
    expect(screen.getByText('Video export method')).toBeTruthy();
    expect(screen.queryByText('Export video')).toBeNull();
  });

  it('applies Japanese UI and persists loop playback', async () => {
    persistSettings({
      ...defaultSettings,
      languageMode: 'manual',
      language: 'ja',
    });
    render(App);

    await fireEvent.click(screen.getByRole('button', { name: '共通設定' }));
    await fireEvent.click(screen.getByRole('checkbox', { name: 'ループ再生を有効化' }));
    await fireEvent.click(screen.getByRole('button', { name: '適用' }));

    expect(JSON.parse(localStorage.getItem('vidmetry.settings.v1') ?? '{}')).toMatchObject({
      languageMode: 'manual',
      language: 'ja',
      loopPlayback: true,
    });
  });

  it('loads a directory, navigates with Page Down, and opens save directly', async () => {
    const paths = ['C:\\clips\\a.mp4', 'C:\\clips\\b.mp4'];
    vi.mocked(dialogOpen).mockResolvedValue('C:\\clips');
    vi.mocked(dialogSave).mockResolvedValue(null);
    vi.mocked(invoke).mockImplementation(async (command, args) => {
      if (command === 'inspect_selection') {
        return { kind: 'directory', rootPath: 'C:\\clips', videoPaths: paths } as never;
      }
      if (command === 'probe_video') {
        const sourcePath = String((args as { path: string }).path);
        return {
          sourcePath,
          fileName: sourcePath.endsWith('b.mp4') ? 'b.mp4' : 'a.mp4',
          durationSeconds: 4,
          codedWidth: 1280,
          codedHeight: 720,
          displayWidth: 1280,
          displayHeight: 720,
          rotationDegrees: 0,
          sampleAspectRatio: '1:1',
          frameRate: '30/1',
          videoCodec: 'h264',
          pixelFormat: 'yuv420p',
          bitDepth: 8,
          hasAudio: true,
          audioCodec: 'aac',
          color: { primaries: null, transfer: null, matrix: null, range: null },
          metadataCropSupported: true,
        } as never;
      }
      throw new Error(`Unexpected command: ${command}`);
    });
    render(App);

    await fireEvent.click(screen.getAllByRole('button', { name: 'Open folder' }).at(-1)!);
    expect(await screen.findByRole('option', { name: '2. b.mp4' })).toBeTruthy();
    await waitFor(() =>
      expect((screen.getByRole('button', { name: 'Next video' }) as HTMLButtonElement).disabled).toBe(
        false,
      ),
    );

    await fireEvent.keyDown(document.body, { key: 'PageDown', code: 'PageDown' });
    await waitFor(() => expect(screen.getByText('b.mp4')).toBeTruthy());
    await fireEvent.click(screen.getByRole('button', { name: 'Export' }));

    expect(dialogSave).toHaveBeenCalledOnce();
    expect(screen.queryByRole('dialog', { name: /export/i })).toBeNull();
  });
});
