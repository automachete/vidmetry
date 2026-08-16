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

function mockSelection(paths = videoPaths, failingProbePath?: string): void {
  vi.mocked(invoke).mockImplementation(async (command, args) => {
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
      return mediaDescriptor(path) as never;
    }
    if (command === 'system_accent_color') return '#FF8C00' as never;
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
    const settingsButton = screen.getByRole('button', { name: 'Settings' });
    await waitFor(() => expect((settingsButton as HTMLButtonElement).disabled).toBe(false));
    await fireEvent.click(settingsButton);

    expect(screen.getByRole('heading', { name: 'Settings' })).toBeTruthy();
    expect(screen.getByText('Video export method')).toBeTruthy();
    expect(screen.queryByText('Export video')).toBeNull();
    const sections = container.querySelectorAll('.settings-scroll > .settings-section');
    expect(sections.item(sections.length - 1).querySelector('h3')?.textContent).toBe(
      'Display language',
    );
  });

  it('applies Japanese UI and persists loop playback', async () => {
    storeState.value = { ...defaultSettings, languageMode: 'manual', language: 'ja' };
    render(App);

    const settingsButton = await screen.findByRole('button', { name: '共通設定' });
    await waitFor(() => expect((settingsButton as HTMLButtonElement).disabled).toBe(false));
    await fireEvent.click(settingsButton);
    await fireEvent.click(screen.getByRole('checkbox', { name: 'ループ再生を有効化' }));
    await fireEvent.click(screen.getByRole('button', { name: '適用' }));

    await waitFor(() => expect(storeState.save).toHaveBeenCalledOnce());
    expect(storeState.value).toMatchObject({
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

  it('keeps the current playlist position when the next video cannot be probed', async () => {
    useEnglish();
    const paths = ['C:\\clips\\a.mp4', 'C:\\clips\\b.mp4'];
    vi.mocked(dialogOpen).mockResolvedValue('C:\\clips');
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
    useEnglish();
    vi.mocked(dialogOpen).mockResolvedValue('C:\\blocked');
    vi.mocked(invoke).mockRejectedValue({ code: 'folder_read_failed', detail: 'access denied' });
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
    await fireEvent.click(screen.getByRole('checkbox', { name: 'Enable loop playback' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Apply' }));

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
