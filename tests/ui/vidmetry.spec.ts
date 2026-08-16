import { expect, test, type Page } from '@playwright/test';

const settings = {
  version: 1,
  languageMode: 'manual',
  language: 'en',
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

async function installTauriMock(page: Page): Promise<void> {
  await page.addInitScript((persistedSettings) => {
    const callbacks = new Map<number, (data: unknown) => unknown>();
    const listeners = new Map<string, number[]>();
    let nextCallback = 1;
    const invocations: Array<{ command: string; args: unknown }> = [];
    const sourceExtension = new URLSearchParams(location.search).get('source') === 'mov' ? 'mov' : 'mp4';
    const requestedTheme = new URLSearchParams(location.search).get('theme') === 'light' ? 'light' : 'dark';
    const sourcePath = `C:\\clips\\sample.${sourceExtension}`;

    const transformCallback = (callback?: (data: unknown) => unknown, once = false) => {
      const id = nextCallback++;
      callbacks.set(id, (data) => {
        if (once) callbacks.delete(id);
        return callback?.(data);
      });
      return id;
    };

    const runCallback = (id: number, data: unknown) => callbacks.get(id)?.(data);
    const invoke = async (command: string, args: Record<string, any> = {}) => {
      invocations.push({ command, args });
      if (command === 'plugin:event|listen') {
        const eventListeners = listeners.get(args.event) ?? [];
        eventListeners.push(args.handler);
        listeners.set(args.event, eventListeners);
        return args.handler;
      }
      if (command === 'plugin:event|unlisten') {
        const eventListeners = listeners.get(args.event) ?? [];
        listeners.set(
          args.event,
          eventListeners.filter((id) => id !== args.eventId),
        );
        return null;
      }
      if (command === 'plugin:window|theme') return requestedTheme;
      if (command === 'plugin:window|set_fullscreen') {
        (window as any).__vidmetryFullscreen = Boolean(args.value);
        return null;
      }
      if (command === 'system_accent_color') return '#FF8C00';
      if (command === 'plugin:dialog|open') {
        return args.options?.directory ? 'C:\\clips' : sourcePath;
      }
      if (command === 'plugin:dialog|save') return 'C:\\clips\\sample_cropped.mp4';
      if (command === 'inspect_selection') {
        if (new URLSearchParams(location.search).get('selectionError') === 'folder') {
          throw { code: 'folder_read_failed', detail: 'access denied' };
        }
        const folder = args.path === 'C:\\clips';
        return {
          kind: folder ? 'directory' : 'file',
          rootPath: folder ? 'C:\\clips' : sourcePath,
          videoPaths: folder ? ['C:\\clips\\first.mp4', 'C:\\clips\\second.mp4'] : [sourcePath],
        };
      }
      if (command === 'probe_video') {
        const path = String(args.path);
        return {
          sourcePath: path,
          fileName: path.slice(path.lastIndexOf('\\') + 1),
          durationSeconds: 8,
          frameCount: 240,
          codedWidth: 1920,
          codedHeight: 1080,
          displayWidth: 1920,
          displayHeight: 1080,
          rotationDegrees: 0,
          sampleAspectRatio: '1:1',
          frameRate: '30000/1001',
          videoCodec: 'h264',
          pixelFormat: 'yuv420p',
          bitDepth: 8,
          hasAudio: true,
          audioCodec: 'aac',
          color: { primaries: 'bt709', transfer: 'bt709', matrix: 'bt709', range: 'tv' },
          metadataCropSupported: true,
        };
      }
      if (command === 'create_preview') return 'C:\\cache\\preview.mp4';
      if (command === 'create_timeline_strip') return 'C:\\cache\\timeline.jpg';
      if (command === 'start_export') return 'job-1';
      if (command === 'reveal_in_explorer') return null;
      return null;
    };

    Object.defineProperty(window, '__TAURI_INTERNALS__', {
      configurable: true,
      value: {
        metadata: {
          currentWindow: { label: 'main' },
          currentWebview: { windowLabel: 'main', label: 'main' },
        },
        invoke,
        transformCallback,
        unregisterCallback: (id: number) => callbacks.delete(id),
        runCallback,
        callbacks,
        convertFileSrc: (path: string) => `http://asset.localhost/${encodeURIComponent(path)}`,
      },
    });
    Object.defineProperty(window, '__TAURI_EVENT_PLUGIN_INTERNALS__', {
      configurable: true,
      value: { unregisterListener: (_event: string, id: number) => callbacks.delete(id) },
    });
    Object.assign(window, {
      __vidmetryInvocations: invocations,
      __emitTauri: (event: string, payload: unknown) => {
        for (const id of listeners.get(event) ?? []) {
          runCallback(id, { event, id, payload });
        }
      },
      __vidmetryPlayCount: 0,
      __vidmetryFullscreen: false,
    });
    const pausedState = new WeakMap<HTMLMediaElement, boolean>();
    Object.defineProperty(HTMLMediaElement.prototype, 'paused', {
      configurable: true,
      get() {
        return pausedState.get(this) ?? true;
      },
    });
    HTMLMediaElement.prototype.play = async function () {
      (window as any).__vidmetryPlayCount += 1;
      pausedState.set(this, false);
      this.dispatchEvent(new Event('play'));
    };
    HTMLMediaElement.prototype.pause = function () {
      const wasPaused = pausedState.get(this) ?? true;
      pausedState.set(this, true);
      if (!wasPaused) this.dispatchEvent(new Event('pause'));
    };
    localStorage.setItem('vidmetry.settings.v1', JSON.stringify(persistedSettings));
  }, settings);
}

test.beforeEach(async ({ page }) => {
  await installTauriMock(page);
});

test('launcher is concise and keeps only the settings command in its header', async ({ page }) => {
  await page.goto('/');

  await expect(page.locator('header button')).toHaveCount(1);
  await expect(page.getByRole('button', { name: 'Settings' })).toBeVisible();
  await expect(page.getByText('Drop or select a video or folder')).toBeVisible();
  await expect(page.locator('.brand-icon')).toBeVisible();
  await expect(page.getByText('Local video cropper')).toHaveCount(0);
  await expect(page.getByText('Keep precisely the part of the frame you need.')).toHaveCount(0);
  await expect(page.locator('.app-shell')).toHaveScreenshot('launcher.png', { animations: 'disabled' });
});

test('structured backend errors use the selected UI language', async ({ page }) => {
  await page.goto('/?selectionError=folder');
  await page.getByRole('button', { name: 'Open folder' }).click();

  await expect(page.getByRole('alert')).toContainText(
    'The folder could not be read. (access denied)',
  );
});

test('settings place display language at the bottom', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Settings' }).click();

  const headings = await page.locator('.settings-scroll > .settings-section > h3').allTextContents();
  expect(headings.at(-1)).toBe('Display language');
  await expect(page.getByRole('dialog', { name: 'Settings' })).toHaveScreenshot('settings.png', {
    animations: 'disabled',
  });
});

test('folder navigation, save options, success alignment, and Explorer selection work together', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Open folder' }).click();
  await expect(page.getByRole('option', { name: '2. second.mp4' })).toBeAttached();

  for (const navigationButton of await page.locator('.playlist-nav').all()) {
    const buttonBox = await navigationButton.boundingBox();
    const iconBox = await navigationButton.locator('svg').boundingBox();
    expect(buttonBox).not.toBeNull();
    expect(iconBox).not.toBeNull();
    expect(Math.abs(buttonBox!.x + buttonBox!.width / 2 - (iconBox!.x + iconBox!.width / 2))).toBeLessThanOrEqual(0.5);
    expect(Math.abs(buttonBox!.y + buttonBox!.height / 2 - (iconBox!.y + iconBox!.height / 2))).toBeLessThanOrEqual(0.5);
  }

  await page.getByRole('button', { name: 'Save options' }).click();
  await expect(page.getByRole('menu')).toBeVisible();
  await expect(page.getByRole('menuitem', { name: 'Save a copy' })).toBeVisible();
  await expect(page.getByRole('menuitem', { name: 'Save', exact: true })).toBeVisible();
  await expect(page.locator('.app-shell')).toHaveScreenshot('editor-save-menu.png', {
    animations: 'disabled',
  });
  await page.getByRole('menuitem', { name: 'Save a copy' }).click();
  await page.evaluate(() =>
    (window as any).__emitTauri('export-complete', {
      jobId: 'job-1',
      outputPath: 'C:\\clips\\sample_cropped.mp4',
    }),
  );

  const notice = page.getByRole('status');
  await expect(notice).toContainText('Saved:');
  const verticalCenters = await notice.locator(':scope > *').evaluateAll((elements) =>
    elements.map((element) => {
      const box = element.getBoundingClientRect();
      return box.y + box.height / 2;
    }),
  );
  expect(Math.max(...verticalCenters) - Math.min(...verticalCenters)).toBeLessThanOrEqual(1);
  await expect(page.locator('.app-shell')).toHaveScreenshot('editor-success.png', {
    animations: 'disabled',
  });

  await page.getByRole('link', { name: /Show saved file in File Explorer:/ }).click();
  const revealed = await page.evaluate(() =>
    (window as any).__vidmetryInvocations.some(
      (item: { command: string; args: { path?: string } }) =>
        item.command === 'reveal_in_explorer' && item.args.path === 'C:\\clips\\sample_cropped.mp4',
    ),
  );
  expect(revealed).toBe(true);
});

test('different extensions use direct Save a copy and Space starts playback', async ({ page }) => {
  await page.goto('/?source=mov');
  await page.getByRole('button', { name: 'Open video' }).click();
  await expect(page.getByText('sample.mov')).toBeVisible();

  await expect(page.getByRole('button', { name: 'Save options' })).toHaveCount(0);
  const saveCopy = page.getByRole('button', { name: 'Save a copy' });
  await expect(saveCopy).toBeVisible();
  await expect(saveCopy).not.toHaveAttribute('aria-haspopup', 'menu');
  await page.keyboard.press('Space');
  await expect.poll(() => page.evaluate(() => (window as any).__vidmetryPlayCount)).toBe(1);
});

test('Space toggles playback from the focused playback-position handle', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Open video' }).click();

  const timeline = page.locator('.trim-timeline');
  const timelineBox = await timeline.boundingBox();
  expect(timelineBox).not.toBeNull();
  const pointerY = timelineBox!.y + timelineBox!.height / 2;
  await page.mouse.click(timelineBox!.x + timelineBox!.width * 0.4, pointerY);

  const playbackPositionHandle = page.locator('.timeline-playhead');
  const handleBox = await playbackPositionHandle.boundingBox();
  expect(handleBox).not.toBeNull();
  await page.mouse.click(handleBox!.x + handleBox!.width / 2, pointerY);

  const playbackScrubber = page.getByRole('slider', { name: 'Playback-position handle' });
  await expect(playbackScrubber).toBeFocused();
  const video = page.locator('video');
  await expect.poll(() => video.evaluate((element) => (element as HTMLVideoElement).paused)).toBe(true);
  await page.keyboard.press('Space');
  await expect.poll(() => page.evaluate(() => (window as any).__vidmetryPlayCount)).toBe(1);
  await expect.poll(() => video.evaluate((element) => (element as HTMLVideoElement).paused)).toBe(false);
  await expect(page.getByRole('button', { name: 'Pause' })).toBeVisible();
});

test('trim-boundary handles use exact frame steps and export the selected range', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Open video' }).click();

  const start = page.getByRole('slider', { name: 'Start trim-boundary handle' });
  const end = page.getByRole('slider', { name: 'End trim-boundary handle' });
  await expect(start).toHaveAttribute('aria-valuenow', '0');
  await expect(end).toHaveAttribute('aria-valuenow', '240');
  await start.click();
  await expect(start).toBeFocused();
  await expect(start).toHaveClass(/selected/);
  expect(await start.evaluate((element) => getComputedStyle(element).boxShadow)).not.toBe('none');
  await page.keyboard.press('ArrowRight');
  await expect(start).toHaveAttribute('aria-valuenow', '1');
  await page.keyboard.press('Shift+ArrowRight');
  await expect(start).toHaveAttribute('aria-valuenow', '11');
  await end.click();
  await expect(end).toBeFocused();
  await expect(end).toHaveClass(/selected/);
  await page.keyboard.press('ArrowLeft');
  await expect(end).toHaveAttribute('aria-valuenow', '239');
  await page.keyboard.press('Shift+ArrowLeft');
  await expect(end).toHaveAttribute('aria-valuenow', '229');

  await page.getByRole('button', { name: 'Save options' }).click();
  await page.getByRole('menuitem', { name: 'Save a copy' }).click();
  const trim = await page.evaluate(() => {
    const invocation = (window as any).__vidmetryInvocations.find(
      (item: { command: string }) => item.command === 'start_export',
    );
    return invocation.args.request.trim;
  });
  expect(trim).toEqual({ startFrame: 11, endFrame: 229 });
});

test('playback-scrubber clicks and trim-boundary handles stay under the pointer after the selection is halved', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Open video' }).click();

  const timeline = page.locator('.trim-timeline');
  const start = page.getByRole('slider', { name: 'Start trim-boundary handle' });
  const end = page.getByRole('slider', { name: 'End trim-boundary handle' });
  await start.click();
  for (let index = 0; index < 6; index += 1) await page.keyboard.press('Shift+ArrowRight');
  await end.click();
  for (let index = 0; index < 6; index += 1) await page.keyboard.press('Shift+ArrowLeft');
  await expect(start).toHaveAttribute('aria-valuenow', '60');
  await expect(end).toHaveAttribute('aria-valuenow', '180');

  const timelineBox = await timeline.boundingBox();
  expect(timelineBox).not.toBeNull();
  const pointerY = timelineBox!.y + timelineBox!.height / 2;
  const seekPointerX = timelineBox!.x + timelineBox!.width * 0.4;
  await page.mouse.click(seekPointerX, pointerY);
  const playheadBox = await page.locator('.timeline-playhead').boundingBox();
  expect(playheadBox).not.toBeNull();
  const halfFrameWidth = timelineBox!.width / 240 / 2;
  expect(Math.abs(playheadBox!.x + playheadBox!.width / 2 - seekPointerX)).toBeLessThanOrEqual(
    halfFrameWidth + 1,
  );

  const handleBox = await start.boundingBox();
  expect(handleBox).not.toBeNull();
  const pointerX = timelineBox!.x + timelineBox!.width * 0.35;
  await page.mouse.move(handleBox!.x + 2, pointerY);
  await page.mouse.down();
  await page.mouse.move(pointerX, pointerY);
  await page.mouse.up();

  const movedBox = await start.boundingBox();
  expect(movedBox).not.toBeNull();
  expect(Math.abs(movedBox!.x + movedBox!.width / 2 - pointerX)).toBeLessThanOrEqual(
    halfFrameWidth + 1,
  );

  const endBox = await end.boundingBox();
  expect(endBox).not.toBeNull();
  const finalPointerX = timelineBox!.x + timelineBox!.width * 0.65;
  await page.mouse.move(endBox!.x + endBox!.width / 2, pointerY);
  await page.mouse.down();
  await page.evaluate(
    ({ finalX, y }) => {
      const staleSample = new PointerEvent('pointermove', {
        bubbles: true,
        clientX: finalX - 30,
        clientY: y,
        pointerId: 1,
        pointerType: 'mouse',
      });
      const finalEvent = new PointerEvent('pointermove', {
        bubbles: true,
        clientX: finalX,
        clientY: y,
        pointerId: 1,
        pointerType: 'mouse',
      });
      Object.defineProperty(finalEvent, 'getCoalescedEvents', { value: () => [staleSample] });
      window.dispatchEvent(finalEvent);
      window.dispatchEvent(
        new PointerEvent('pointerup', {
          bubbles: true,
          clientX: finalX,
          clientY: y,
          pointerId: 1,
          pointerType: 'mouse',
        }),
      );
    },
    { finalX: finalPointerX, y: pointerY },
  );
  await page.mouse.up();
  const finalEndBox = await end.boundingBox();
  expect(finalEndBox).not.toBeNull();
  expect(Math.abs(finalEndBox!.x + finalEndBox!.width / 2 - finalPointerX)).toBeLessThanOrEqual(
    halfFrameWidth + 1,
  );
});

test('Windows light mode and accent color drive the whole editor and trim bar', async ({ page }) => {
  await page.goto('/?theme=light');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await expect
    .poll(() => page.locator('html').evaluate((element) => getComputedStyle(element).getPropertyValue('--accent').trim()))
    .toBe('#FF8C00');
  await page.getByRole('button', { name: 'Open video' }).click();
  const trimColor = await page.locator('.trim-selection').evaluate((element) => getComputedStyle(element).borderTopColor);
  expect(trimColor).toBe('rgb(255, 140, 0)');
  await expect(page.locator('.app-shell')).toHaveScreenshot('editor-light-theme.png', {
    animations: 'disabled',
  });
});

test('save shortcuts, collapsible panes, and F11 preview are wired', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Open video' }).click();

  await page.getByRole('button', { name: 'Close crop details' }).click();
  await expect(page.getByRole('button', { name: 'Open crop details' })).toBeVisible();
  await page.getByRole('button', { name: 'Close playback and trimming' }).click();
  await expect(page.getByRole('button', { name: 'Open playback and trimming' })).toBeVisible();

  await page.evaluate(() => window.dispatchEvent(new KeyboardEvent('keydown', { code: 'F11', key: 'F11', bubbles: true })))
  await expect(page.locator('.app-shell')).toHaveClass(/video-fullscreen/);
  expect(await page.evaluate(() => (window as any).__vidmetryFullscreen)).toBe(true);
  await page.evaluate(() => window.dispatchEvent(new KeyboardEvent('keydown', { code: 'F11', key: 'F11', bubbles: true })))
  await expect(page.locator('.app-shell')).not.toHaveClass(/video-fullscreen/);

  await page.keyboard.press('Control+s');
  const copied = await page.evaluate(() =>
    (window as any).__vidmetryInvocations.some(
      (item: { command: string; args: { request?: { inPlace?: boolean } } }) =>
        item.command === 'start_export' && item.args.request?.inPlace === false,
    ),
  );
  expect(copied).toBe(true);
});

test('Ctrl+Shift+S confirms and starts in-place save', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Open video' }).click();
  page.once('dialog', (dialog) => dialog.accept());
  await page.keyboard.press('Control+Shift+s');
  await expect
    .poll(() =>
      page.evaluate(() =>
        (window as any).__vidmetryInvocations.some(
          (item: { command: string; args: { request?: { inPlace?: boolean } } }) =>
            item.command === 'start_export' && item.args.request?.inPlace === true,
        ),
      ),
    )
    .toBe(true);
});

test('save notice closes automatically after three seconds', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Open video' }).click();
  await page.getByRole('button', { name: 'Save options' }).click();
  await page.getByRole('menuitem', { name: 'Save a copy' }).click();
  await page.evaluate(() =>
    (window as any).__emitTauri('export-complete', {
      jobId: 'job-1',
      outputPath: 'C:\\clips\\sample_cropped.mp4',
    }),
  );
  await expect(page.getByRole('status')).toBeVisible();
  await page.waitForTimeout(3100);
  await expect(page.getByRole('status')).toHaveCount(0);
});
