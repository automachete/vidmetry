import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import { open as dialogOpen, save as dialogSave } from '@tauri-apps/plugin-dialog';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import App from './App.svelte';
import { defaultSettings, persistSettings } from './lib/settings';

const eventState = vi.hoisted(() => ({
  handlers: new Map<string, (event: { payload: never }) => void>(),
}));

const windowState = vi.hoisted(() => ({
  theme: vi.fn().mockResolvedValue('dark'),
  onThemeChanged: vi.fn().mockResolvedValue(vi.fn()),
  setFullscreen: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (path: string) => `asset://${path}`,
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (name: string, handler: (event: { payload: never }) => void) => {
    eventState.handlers.set(name, handler);
    return vi.fn();
  }),
}));

vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: () => ({ onDragDropEvent: vi.fn().mockResolvedValue(vi.fn()) }),
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => windowState,
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

const videoPaths = ['C:\\clips\\a.mp4'];

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
    sampleAspectRatio: '1:1',
    frameRate: '30/1',
    videoCodec: 'h264',
    pixelFormat: 'yuv420p',
    bitDepth: 8,
    hasAudio: true,
    audioCodec: 'aac',
    color: { primaries: null, transfer: null, matrix: null, range: null },
    metadataCropSupported: true,
  };
}

function useEnglish(): void {
  persistSettings({ ...defaultSettings, languageMode: 'manual', language: 'en' });
}

function mockSelection(paths = videoPaths): void {
  vi.mocked(invoke).mockImplementation(async (command, args) => {
    if (command === 'inspect_selection') {
      return {
        kind: paths.length > 1 ? 'directory' : 'file',
        rootPath: paths.length > 1 ? 'C:\\clips' : paths[0],
        videoPaths: paths,
      } as never;
    }
    if (command === 'probe_video') {
      return mediaDescriptor(String((args as { path: string }).path)) as never;
    }
    if (command === 'system_accent_color') return '#FF8C00' as never;
    if (command === 'start_export') return 'job-1' as never;
    if (command === 'reveal_in_explorer') return undefined as never;
    throw new Error(`Unexpected command: ${command}`);
  });
}

describe('application shell', () => {
  beforeEach(() => {
    localStorage.clear();
    eventState.handlers.clear();
    vi.stubGlobal('ResizeObserver', ResizeObserverStub);
    vi.spyOn(HTMLMediaElement.prototype, 'load').mockImplementation(() => undefined);
    vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => undefined);
    vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue(undefined);
    vi.mocked(invoke).mockReset();
    vi.mocked(dialogOpen).mockReset();
    vi.mocked(dialogSave).mockReset();
    windowState.theme.mockReset().mockResolvedValue('dark');
    windowState.onThemeChanged.mockReset().mockResolvedValue(vi.fn());
    windowState.setFullscreen.mockReset().mockResolvedValue(undefined);
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

  it('opens common settings with the language section last', async () => {
    useEnglish();
    const { container } = render(App);
    await fireEvent.click(screen.getByRole('button', { name: 'Settings' }));

    expect(screen.getByRole('heading', { name: 'Settings' })).toBeTruthy();
    expect(screen.getByText('Video export method')).toBeTruthy();
    expect(screen.queryByText('Export video')).toBeNull();
    const sections = container.querySelectorAll('.settings-scroll > .settings-section');
    expect(sections.item(sections.length - 1).querySelector('h3')?.textContent).toBe(
      'Display language',
    );
  });

  it('applies Japanese UI and persists loop playback', async () => {
    persistSettings({ ...defaultSettings, languageMode: 'manual', language: 'ja' });
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

  it('navigates a folder and requires two clicks to save when extensions match', async () => {
    useEnglish();
    const paths = ['C:\\clips\\a.mp4', 'C:\\clips\\b.mp4'];
    vi.mocked(dialogOpen).mockResolvedValue('C:\\clips');
    vi.mocked(dialogSave).mockResolvedValue(null);
    mockSelection(paths);
    const { container } = render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open folder' }));
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

  it('keeps Space playback available while a trim handle has focus', async () => {
    useEnglish();
    vi.mocked(dialogOpen).mockResolvedValue(videoPaths[0]);
    mockSelection();
    render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open video' }));
    const startHandle = await screen.findByRole('slider', { name: 'Adjust start frame' });
    startHandle.focus();
    await fireEvent.keyDown(startHandle, { key: ' ', code: 'Space' });

    expect(HTMLMediaElement.prototype.play).toHaveBeenCalledOnce();
  });

  it('localizes backend validation messages in English mode', async () => {
    useEnglish();
    vi.mocked(dialogOpen).mockResolvedValue('C:\\blocked');
    vi.mocked(invoke).mockRejectedValue('フォルダーを読み取れません: access denied');
    render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open folder' }));
    expect((await screen.findByRole('alert')).textContent).toContain(
      'Could not read the folder: access denied',
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

  it('adjusts the time range by one frame and sends the exclusive range to export', async () => {
    useEnglish();
    vi.mocked(dialogOpen).mockResolvedValue(videoPaths[0]);
    vi.mocked(dialogSave).mockResolvedValue('C:\\clips\\a_cropped.mp4');
    mockSelection();
    render(App);

    await fireEvent.click(screen.getByRole('button', { name: 'Open video' }));
    await screen.findByText('a.mp4');
    const startHandle = screen.getByRole('slider', { name: 'Adjust start frame' });
    const endHandle = screen.getByRole('slider', { name: 'Adjust end frame' });
    expect(startHandle.getAttribute('aria-valuenow')).toBe('0');
    expect(endHandle.getAttribute('aria-valuenow')).toBe('120');

    await fireEvent.keyDown(startHandle, { key: 'ArrowRight', code: 'ArrowRight' });
    expect(startHandle.getAttribute('aria-valuenow')).toBe('1');
    await fireEvent.keyDown(startHandle, { key: 'ArrowRight', code: 'ArrowRight', shiftKey: true });
    expect(startHandle.getAttribute('aria-valuenow')).toBe('11');
    await fireEvent.keyDown(endHandle, { key: 'ArrowLeft', code: 'ArrowLeft', shiftKey: true });
    expect(endHandle.getAttribute('aria-valuenow')).toBe('110');
    await fireEvent.click(screen.getByRole('button', { name: 'Save options' }));
    await fireEvent.click(screen.getByRole('menuitem', { name: 'Save a copy' }));

    expect(invoke).toHaveBeenCalledWith(
      'start_export',
      expect.objectContaining({
        request: expect.objectContaining({ trim: { startFrame: 11, endFrame: 110 } }),
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
    expect(document.documentElement.style.getPropertyValue('--accent')).toBe('#FF8C00');
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

    await fireEvent.pointerDown(screen.getByRole('button', { name: 'Enable loop playback' }));
    expect(screen.queryByRole('status')).toBeNull();
  });
});
