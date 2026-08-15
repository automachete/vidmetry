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
      if (command === 'plugin:dialog|open') {
        return args.options?.directory ? 'C:\\clips' : sourcePath;
      }
      if (command === 'plugin:dialog|save') return 'C:\\clips\\sample_cropped.mp4';
      if (command === 'inspect_selection') {
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
    });
    HTMLMediaElement.prototype.play = async function () {
      (window as any).__vidmetryPlayCount += 1;
    };
    HTMLMediaElement.prototype.pause = function () {};
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
  await expect(page.getByText('Local video cropper')).toHaveCount(0);
  await expect(page.getByText('Keep precisely the part of the frame you need.')).toHaveCount(0);
  await expect(page.locator('.app-shell')).toHaveScreenshot('launcher.png', { animations: 'disabled' });
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

  await page.getByRole('button', { name: /Show saved file in File Explorer:/ }).click();
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
