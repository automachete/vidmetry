import { expect, test, type Locator, type Page } from '@playwright/test';

const settings = {
  languageMode: 'manual',
  language: 'en',
  appearance: {
    themeMode: 'system',
    theme: 'dark',
    accentMode: 'system',
    accentColor: 'blue',
  },
  shortcuts: {
    openVideo: 'Ctrl+KeyO',
    openFolder: 'Ctrl+Shift+KeyO',
    openSettings: 'Ctrl+Comma',
    profileCompatible: 'Alt+Digit1',
    profileLossless: 'Alt+Digit2',
    profileMetadata: 'Alt+Digit3',
    copySave: 'Ctrl+KeyS',
    saveInPlace: 'Ctrl+Shift+KeyS',
    previousVideo: 'PageUp',
    nextVideo: 'PageDown',
    playPause: 'Space',
    seekBackward: 'ArrowLeft',
    seekForward: 'ArrowRight',
    seekBackwardLarge: 'Shift+ArrowLeft',
    seekForwardLarge: 'Shift+ArrowRight',
    toggleFullscreen: 'F11',
  },
  loopPlayback: true,
  explorerIntegration: true,
  folderPicker: {
    mode: 'standard',
    lastPath: null,
    viewMode: 1,
    iconSize: 96,
  },
  export: {
    profile: 'compatible',
    videoCodec: 'h264',
    encoder: 'automatic',
    crf: 17,
    preset: 'medium',
    pixelFormat: 'source',
    audioMode: 'auto',
    audioBitrateKbps: 192,
    frameRateMode: 'passthrough',
    constantFrameRate: 30,
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
    let storedSettings: any = structuredClone(persistedSettings);
    const query = new URLSearchParams(location.search);
    if (query.get('betaPicker') === '1') {
      storedSettings.folderPicker.mode = 'explorerBeta';
    }
    const sourceExtension = query.get('source') === 'mov' ? 'mov' : 'mp4';
    const requestedTheme = query.get('theme') === 'light' ? 'light' : 'dark';
    const simulatedMediaDuration = Number(query.get('mediaDuration'));
    const sourcePath = `C:\\clips\\sample.${sourceExtension}`;
    let directoryVideos = ['C:\\clips\\first.mp4', 'C:\\clips\\second.mp4'];

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
      if (command === 'plugin:window|set_theme') return null;
      if (command === 'plugin:window|set_fullscreen') {
        (window as any).__vidmetryFullscreen = Boolean(args.value);
        return null;
      }
      if (command === 'system_accent_color') return '#FF8C00';
      if (command === 'windows_ui_language') return 'en';
      if (command === 'supported_video_extensions') {
        return [
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
      }
      if (command === 'available_video_encoders') {
        return {
          h264: { nvidia: true, intel: false, amd: true },
          h265: { nvidia: true, intel: false, amd: false },
        };
      }
      if (command === 'startup_selection') return null;
      if (command === 'plugin:store|load') return 1;
      if (command === 'plugin:store|get') return [storedSettings, storedSettings !== undefined];
      if (command === 'plugin:store|set') {
        storedSettings = structuredClone(args.value);
        return null;
      }
      if (command === 'plugin:store|save') return null;
      if (command === 'plugin:log|log') return null;
      if (command === 'plugin:dialog|open') {
        return args.options?.directory ? 'C:\\clips' : sourcePath;
      }
      if (command === 'plugin:dialog|save') return 'C:\\clips\\sample_cropped.mp4';
      if (command === 'pick_video_folder') {
        return { path: 'C:\\clips', viewMode: 1, iconSize: 96 };
      }
      if (command === 'watch_directory') return null;
      if (command === 'inspect_selection') {
        if (
          args.path === 'C:\\clips' &&
          new URLSearchParams(location.search).get('selectionError') === 'folder'
        ) {
          throw { code: 'folder_read_failed', detail: 'access denied' };
        }
        const folder = args.path === 'C:\\clips';
        return {
          kind: folder ? 'directory' : 'file',
          rootPath: folder ? 'C:\\clips' : sourcePath,
          videoPaths: folder ? [...directoryVideos] : [sourcePath],
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
          frameRate: '30000/1001',
          videoCodec: 'h264',
          pixelFormat: 'yuv420p',
          bitDepth: 8,
          hasAudio: true,
          audioCodec: 'aac',
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
      __getStoredSettings: () => structuredClone(storedSettings),
      __setDirectoryVideos: (paths: string[]) => {
        directoryVideos = [...paths];
      },
    });
    const pausedState = new WeakMap<HTMLMediaElement, boolean>();
    Object.defineProperty(HTMLMediaElement.prototype, 'paused', {
      configurable: true,
      get() {
        return pausedState.get(this) ?? true;
      },
    });
    if (Number.isFinite(simulatedMediaDuration) && simulatedMediaDuration > 0) {
      Object.defineProperty(HTMLMediaElement.prototype, 'duration', {
        configurable: true,
        get: () => simulatedMediaDuration,
      });
    }
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
});

test('Windows desktop type ramp and scaled panes stay balanced', async ({ page }) => {
  const auditTextLayout = async (root: Locator) =>
    root.evaluate((scope) => {
      const visible = (element: HTMLElement) => {
        const style = getComputedStyle(element);
        return style.display !== 'none' && style.visibility !== 'hidden' && element.getClientRects().length > 0;
      };
      const candidates = Array.from(
        scope.querySelectorAll<HTMLElement>(
          '.button, .square-button, .pane-button, .text-button, .empty-description, .dialog-heading h2, .settings-nav button, .settings-section h3, .settings-section h4, .settings-field, .radio-row label, .check-list label, .profile-settings button, .palette-trigger, .shortcut-row, .metadata-warning, .inspector h2, .field, .output-size, .media-details div, .trim-readout > span, .trim-reset, .time',
        ),
      ).filter(visible);
      const clipped = candidates
        .filter(
          (element) =>
            element.clientWidth > 0 &&
            (element.scrollWidth > element.clientWidth + 1 || element.scrollHeight > element.clientHeight + 1),
        )
        .map((element) => `${element.tagName.toLowerCase()}.${element.className}: ${element.textContent?.trim()}`);
      const singleLine = Array.from(
        scope.querySelectorAll<HTMLElement>(
          '.button, .empty-description, .dialog-heading h2, .settings-nav button, .settings-section h3, .settings-section h4, .settings-field > span, .profile-settings strong, .inspector h2, .text-button, .field > span, .section-label, .trim-readout > span, .trim-reset, .time',
        ),
      )
        .filter(visible)
        .filter((element) => {
          const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
          const lineTops: number[] = [];
          while (walker.nextNode()) {
            if (!walker.currentNode.textContent?.trim()) continue;
            const range = document.createRange();
            range.selectNodeContents(walker.currentNode);
            lineTops.push(
              ...Array.from(range.getClientRects())
                .filter((rect) => rect.width > 0 && rect.height > 0)
                .map((rect) => Math.round(rect.top)),
            );
          }
          return new Set(lineTops).size > 1;
        })
        .map((element) => `${element.tagName.toLowerCase()}.${element.className}: ${element.textContent?.trim()}`);
      return { clipped, singleLine };
    });

  const expectContainedSettingsFocus = async (control: Locator) => {
    await control.focus();
    await expect(control).toBeFocused();
    const metrics = await control.evaluate((element) => {
      const page = element.closest('.settings-page')!.getBoundingClientRect();
      const focusOwner = element.closest('.unit-input') ?? element;
      const box = focusOwner.getBoundingClientRect();
      const style = getComputedStyle(focusOwner);
      return {
        leftClearance: box.left - page.left,
        rightClearance: page.right - box.right,
        focusVisible: style.outlineStyle !== 'none' || style.boxShadow !== 'none',
      };
    });
    expect(metrics.leftClearance).toBeGreaterThanOrEqual(4);
    expect(metrics.rightClearance).toBeGreaterThanOrEqual(4);
    expect(metrics.focusVisible).toBe(true);
  };

  await page.goto('/');

  const launcherMetrics = await page.evaluate(() => {
    const rect = (selector: string) => document.querySelector(selector)!.getBoundingClientRect();
    const style = (selector: string) => getComputedStyle(document.querySelector(selector)!);
    return {
      bodyFontFamily: style('body').fontFamily,
      bodyFontSize: style('body').fontSize,
      headerHeight: rect('.app-header').height,
      descriptionFontSize: style('.empty-description').fontSize,
      largeButtonHeight: rect('.button.large').height,
      settingsButtonSize: rect('.launcher-settings').width,
    };
  });
  expect(launcherMetrics.bodyFontFamily).toContain('Segoe UI Variable');
  expect(launcherMetrics.bodyFontSize).toBe('14px');
  expect(launcherMetrics.headerHeight).toBe(76);
  expect(launcherMetrics.descriptionFontSize).toBe('18px');
  expect(launcherMetrics.largeButtonHeight).toBe(48);
  expect(launcherMetrics.settingsButtonSize).toBe(40);
  expect(await auditTextLayout(page.locator('.app-shell'))).toEqual({ clipped: [], singleLine: [] });

  await page.getByRole('button', { name: 'Settings' }).click();
  const settingsDialog = page.getByRole('dialog', { name: 'Settings' });
  await expect(settingsDialog.getByText('Settings', { exact: true })).toHaveCount(1);
  await expect(settingsDialog.locator('.section-label')).toHaveCount(0);
  const dialogMetrics = await settingsDialog.evaluate((dialog) => {
    const style = (selector: string) => getComputedStyle(dialog.querySelector(selector)!);
    const rect = (selector: string) => dialog.querySelector(selector)!.getBoundingClientRect();
    return {
      width: dialog.getBoundingClientRect().width,
      height: dialog.getBoundingClientRect().height,
      titleFontSize: style('h2').fontSize,
      sectionFontSize: style('.settings-section h3').fontSize,
      navigationWidth: rect('.settings-nav').width,
      labelFontSize: style('.settings-field').fontSize,
      selectHeight: rect('.settings-field select').height,
      closeButtonSize: rect('.dialog-close').width,
      profileHeight: rect('.profile-settings button').height,
    };
  });
  expect(dialogMetrics.width).toBe(920);
  expect(dialogMetrics.height).toBe(760);
  expect(dialogMetrics.titleFontSize).toBe('20px');
  expect(dialogMetrics.sectionFontSize).toBe('20px');
  expect(dialogMetrics.navigationWidth).toBe(190);
  expect(dialogMetrics.labelFontSize).toBe('12px');
  expect(dialogMetrics.selectHeight).toBe(40);
  expect(dialogMetrics.closeButtonSize).toBe(40);
  expect(dialogMetrics.profileHeight).toBeGreaterThanOrEqual(96);

  const pairedControlOffsets = await settingsDialog.locator('.settings-grid').evaluateAll((grids) =>
    grids.flatMap((grid) => {
      const fields = Array.from(grid.children);
      const offsets: number[] = [];
      for (let index = 0; index + 1 < fields.length; index += 2) {
        const first = fields[index]?.querySelector('select, input, .unit-input');
        const second = fields[index + 1]?.querySelector('select, input, .unit-input');
        if (!first || !second) continue;
        offsets.push(
          Math.abs(first.getBoundingClientRect().top - second.getBoundingClientRect().top),
        );
      }
      return offsets;
    }),
  );
  expect(pairedControlOffsets.length).toBeGreaterThan(0);
  expect(Math.max(...pairedControlOffsets)).toBeLessThanOrEqual(0.5);

  for (const control of await settingsDialog.locator('.settings-page select, .settings-page input[type="number"]:not(:disabled)').all()) {
    await expectContainedSettingsFocus(control);
  }

  await settingsDialog.getByRole('button', { name: 'Appearance' }).click();
  await settingsDialog.locator('.appearance-group').first().getByRole('radio', { name: 'Customize' }).check();
  const themeChoice = settingsDialog.getByRole('combobox', { name: 'App mode' });
  await expectContainedSettingsFocus(themeChoice);
  await settingsDialog.getByRole('button', { name: 'Language' }).click();
  await settingsDialog.getByRole('radio', { name: 'Choose a language' }).check();
  const languageChoice = settingsDialog.getByRole('combobox', { name: 'Display language' });
  await expectContainedSettingsFocus(languageChoice);
  await settingsDialog.getByRole('button', { name: 'Export' }).click();

  await page.locator('.dialog-actions').getByRole('button', { name: 'Close' }).click();
  await page.getByRole('button', { name: 'Open video' }).click();
  const editorMetrics = await page.evaluate(() => {
    const rect = (selector: string) => document.querySelector(selector)!.getBoundingClientRect();
    return {
      inspectorWidth: rect('.inspector').width,
      transportHeight: rect('.transport').height,
      timelineHeight: rect('.trim-timeline').height,
      paneButtonSize: rect('.pane-button').width,
      cropHandleSize: rect('.crop-handle').width,
    };
  });
  expect(editorMetrics).toEqual({
    inspectorWidth: 320,
    transportHeight: 148,
    timelineHeight: 64,
    paneButtonSize: 32,
    cropHandleSize: 16,
  });

  await page.setViewportSize({ width: 960, height: 720 });
  await expect
    .poll(() => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth))
    .toBe(true);
  await expect(page.locator('.video-stage')).toBeVisible();
  await expect(page.locator('.transport')).toBeVisible();
  expect(await auditTextLayout(page.locator('.app-shell'))).toEqual({ clipped: [], singleLine: [] });

  await page.locator('.settings-button').click();
  const compactSettings = page.locator('.settings-dialog');
  await expect(compactSettings).toBeVisible();
  const fixedCompactSize = await compactSettings.evaluate((dialog) => {
    const box = dialog.getBoundingClientRect();
    return { width: box.width, height: box.height };
  });
  expect(fixedCompactSize).toEqual({ width: 912, height: 672 });
  await expect(compactSettings.locator('.settings-save-status')).toHaveCount(0);
  expect(await auditTextLayout(compactSettings)).toEqual({ clipped: [], singleLine: [] });

  for (const category of ['Playback', 'Appearance', 'Keyboard shortcuts', 'File Explorer', 'Language']) {
    await compactSettings.getByRole('button', { name: category }).click();
    if (category === 'Appearance') {
      await compactSettings.locator('.appearance-group').nth(1).getByRole('radio', { name: 'Customize' }).check();
      await compactSettings.getByRole('button', { name: 'Open color palette' }).click();
    }
    expect(
      await compactSettings.evaluate((dialog) => {
        const box = dialog.getBoundingClientRect();
        return { width: box.width, height: box.height };
      }),
    ).toEqual(fixedCompactSize);
    expect(await auditTextLayout(compactSettings)).toEqual({ clipped: [], singleLine: [] });
  }

  await compactSettings.locator('.settings-field.compact select').selectOption('ja');
  await expect(compactSettings.getByRole('heading', { name: '共通設定' })).toBeVisible();
  await expect(compactSettings.getByText('共通設定', { exact: true })).toHaveCount(1);
  await expectContainedSettingsFocus(compactSettings.locator('.settings-field.compact select'));
  expect(
    await compactSettings.evaluate((dialog) => {
      const box = dialog.getBoundingClientRect();
      return { width: box.width, height: box.height };
    }),
  ).toEqual(fixedCompactSize);
  expect(await auditTextLayout(compactSettings)).toEqual({ clipped: [], singleLine: [] });

  await compactSettings.locator('.dialog-actions .button.primary').click();
  await expect(page.getByRole('button', { name: '共通設定' })).toBeVisible();
  expect(await auditTextLayout(page.locator('.app-shell'))).toEqual({ clipped: [], singleLine: [] });
  expect(
    await page.evaluate(() => ({
      horizontalOverflow: document.documentElement.scrollWidth > window.innerWidth,
      verticalOverflow: document.documentElement.scrollHeight > window.innerHeight,
    })),
  ).toEqual({ horizontalOverflow: false, verticalOverflow: false });
});

test('structured backend errors use the selected UI language', async ({ page }) => {
  await page.goto('/?selectionError=folder');
  await page.getByRole('button', { name: 'Open folder' }).click();

  await expect(page.getByRole('alert')).toContainText(
    'The folder could not be read. (access denied)',
  );
});

test('ffprobe timing remains authoritative over the preview element duration', async ({ page }) => {
  await page.goto('/?mediaDuration=80');
  await page.getByRole('button', { name: 'Open video' }).click();

  await expect(page.locator('.time.total')).toHaveText('0:08.000');
});

test('directory navigation continues playback in regular and fullscreen preview', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Open folder' }).click();
  const video = page.locator('video');

  await page.getByRole('button', { name: 'Play', exact: true }).click();
  await expect.poll(() => page.evaluate(() => (window as any).__vidmetryPlayCount)).toBe(1);
  await page.getByRole('button', { name: 'Next video' }).click();
  await video.dispatchEvent('canplay');
  await expect.poll(() => page.evaluate(() => (window as any).__vidmetryPlayCount)).toBe(2);

  await page.keyboard.press('F11');
  await expect(page.locator('.app-shell')).toHaveClass(/video-fullscreen/);
  await page.keyboard.press('PageUp');
  await video.dispatchEvent('canplay');
  await expect.poll(() => page.evaluate(() => (window as any).__vidmetryPlayCount)).toBe(3);
});

test('settings use one-level category navigation and expose encoder availability', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Settings' }).click();

  const settingsNavigation = page.getByRole('navigation', { name: 'Settings categories' });
  await expect(settingsNavigation.getByRole('button')).toHaveText([
    'Export',
    'Playback',
    'Appearance',
    'Keyboard shortcuts',
    'File Explorer',
    'Language',
  ]);
  await expect(page.locator('.settings-page > .settings-section > h3')).toHaveText('Export');
  const encoder = page.getByRole('combobox', { name: 'Encoder' });
  await expect(encoder.locator('option[value="automatic"]')).toHaveText('Automatic');
  await expect(encoder.locator('option[value="nvidia"]')).toHaveText('nvenc');
  await expect(encoder.locator('option[value="nvidia"]')).not.toHaveAttribute('disabled', '');
  await expect(encoder.locator('option[value="intel"]')).toHaveText('qsv');
  await expect(encoder.locator('option[value="intel"]')).toHaveAttribute('disabled', '');
  await expect(encoder.locator('option[value="amd"]')).toHaveText('amf');
  await expect(encoder.locator('option[value="amd"]')).not.toHaveAttribute('disabled', '');
  await expect(encoder.locator('option[value="software"]')).toHaveText('libx264');
  await encoder.selectOption('amd');
  await expect(encoder).toHaveValue('amd');
  await encoder.selectOption('automatic');
  const codec = page.getByRole('combobox', { name: 'Video codec' });
  await codec.selectOption('h265');
  await expect(encoder.locator('option[value="software"]')).toHaveText('libx265');
  await expect(encoder.locator('option[value="amd"]')).toHaveAttribute('disabled', '');
  await codec.selectOption('h264');
  await expect(page.getByRole('button', { name: 'Apply' })).toHaveCount(0);
  await settingsNavigation.getByRole('button', { name: 'File Explorer' }).click();
  const folderPicker = page.getByRole('combobox', { name: 'Folder picker' });
  await expect(folderPicker).toHaveValue('standard');
  await expect(folderPicker.locator('option[value="standard"]')).toHaveText('Windows standard');
  await expect(folderPicker.locator('option[value="explorerBeta"]')).toHaveText('Show video files');
  await expect(page.getByText('You can view supported videos in the folder (Beta).')).toHaveCount(0);
  await folderPicker.selectOption('explorerBeta');
  await expect(page.getByText('You can view supported videos in the folder (Beta).')).toBeVisible();
  await expect
    .poll(() => page.evaluate(() => (window as any).__getStoredSettings().folderPicker.mode))
    .toBe('explorerBeta');
  await settingsNavigation.getByRole('button', { name: 'Language' }).click();
  await expect(page.getByRole('combobox', { name: 'Display language' })).toBeVisible();
  await expect(page.locator('.dialog-actions').getByRole('button', { name: 'Close' })).toBeVisible();
});

test('settings save appearance and recorded shortcuts immediately', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Open video' })).toHaveAttribute(
    'title',
    'Open video (Ctrl+O)',
  );
  await expect(page.getByRole('button', { name: 'Open folder' })).toHaveAttribute(
    'title',
    'Open folder (Ctrl+Shift+O)',
  );
  await page.getByRole('button', { name: 'Settings' }).click();
  const dialog = page.getByRole('dialog', { name: 'Settings' });
  await expect(dialog.getByText('Configure export, playback, and File Explorer integration.')).toHaveCount(0);
  await expect(dialog.getByText('The loop playback state is remembered automatically.')).toHaveCount(0);
  await expect(dialog.getByText(/Vidmetry always appears in Open with/)).toHaveCount(0);

  await dialog.getByRole('button', { name: 'Appearance' }).click();
  const customize = dialog.getByRole('radio', { name: 'Customize' });
  await customize.nth(0).check();
  await dialog.getByRole('combobox', { name: 'App mode' }).selectOption('light');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await customize.nth(1).check();
  await dialog.getByRole('button', { name: 'Open color palette' }).click();
  await dialog.getByRole('option', { name: 'Purple' }).click();
  await expect
    .poll(() => page.locator('html').evaluate((element) => element.style.getPropertyValue('--accent')))
    .toBe('#5C2E91');
  await expect
    .poll(() =>
      page.evaluate(() => (window as any).__getStoredSettings().appearance),
    )
    .toEqual({ themeMode: 'manual', theme: 'light', accentMode: 'manual', accentColor: 'purple' });

  await dialog.getByRole('button', { name: 'Keyboard shortcuts' }).click();
  expect(
    await dialog
      .getByRole('button', { name: /^Change shortcut:/ })
      .evaluateAll((buttons) => buttons.map((button) => button.getAttribute('aria-label'))),
  ).toEqual([
    'Change shortcut: Play / pause',
    'Change shortcut: Seek back 1 frame',
    'Change shortcut: Seek forward 1 frame',
    'Change shortcut: Seek back 10 frames',
    'Change shortcut: Seek forward 10 frames',
    'Change shortcut: Save a copy',
    'Change shortcut: Save over the source video',
    'Change shortcut: Previous video',
    'Change shortcut: Next video',
    'Change shortcut: Open video',
    'Change shortcut: Open folder',
    'Change shortcut: Switch to Compatible MP4',
    'Change shortcut: Switch to Lossless FFV1 / MKV',
    'Change shortcut: Switch to Metadata only',
    'Change shortcut: Toggle fullscreen preview',
    'Change shortcut: Open Settings',
  ]);
  await dialog.getByRole('button', { name: 'Change shortcut: Open video' }).click();
  await page.keyboard.press('Control+p');
  await expect(dialog.getByRole('button', { name: 'Change shortcut: Open video' }).locator('kbd')).toHaveText(
    'Ctrl+P',
  );
  await dialog.getByRole('button', { name: 'Change shortcut: Play / pause' }).click();
  await page.keyboard.press('k');
  await expect(dialog.getByRole('button', { name: 'Change shortcut: Play / pause' }).locator('kbd')).toHaveText(
    'K',
  );
  await dialog.getByRole('button', { name: 'Change shortcut: Save a copy' }).click();
  await page.keyboard.press('Control+k');
  await dialog
    .getByRole('button', { name: 'Change shortcut: Save over the source video' })
    .click();
  await page.keyboard.press('Control+Shift+k');
  await dialog.getByRole('button', { name: 'Change shortcut: Previous video' }).click();
  await page.keyboard.press('Control+PageUp');
  await dialog.getByRole('button', { name: 'Change shortcut: Next video' }).click();
  await page.keyboard.press('Control+PageDown');
  await dialog.getByRole('button', { name: 'Change shortcut: Switch to Compatible MP4' }).click();
  await page.keyboard.press('Alt+4');
  await dialog.getByRole('button', { name: 'Change shortcut: Toggle fullscreen preview' }).click();
  await page.keyboard.press('F10');
  await dialog.getByRole('button', { name: 'Change shortcut: Open Settings' }).click();
  await page.keyboard.press('Control+Period');
  await dialog.getByRole('button', { name: 'Change shortcut: Open folder' }).click();
  await page.keyboard.press('Control+p');
  await expect(dialog.getByRole('alert')).toContainText('Already used by “Open video”.');
  await page.keyboard.press('Escape');
  await dialog.getByRole('button', { name: 'Change shortcut: Open folder' }).click();
  await page.keyboard.press('Control+Shift+p');
  await dialog.getByRole('button', { name: 'Export' }).click();
  await expect(dialog.locator('.profile-settings button').nth(0)).toHaveAttribute(
    'title',
    'Compatible MP4 (Alt+4)',
  );
  await dialog.locator('.dialog-actions').getByRole('button', { name: 'Close' }).click();

  await expect(page.getByRole('button', { name: 'Open video' })).toHaveAttribute(
    'title',
    'Open video (Ctrl+P)',
  );
  await expect(page.getByRole('button', { name: 'Open folder' })).toHaveAttribute(
    'title',
    'Open folder (Ctrl+Shift+P)',
  );
  await expect(page.getByRole('button', { name: 'Settings' })).toHaveAttribute(
    'title',
    'Settings (Ctrl+.)',
  );

  await page.keyboard.press('Control+p');
  await expect(page.getByText('sample.mp4', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Open another video' })).toHaveAttribute(
    'title',
    'Open another video (Ctrl+P)',
  );
  await page.getByRole('button', { name: 'Open folder' }).click();
  await expect(page.getByRole('option', { name: '2. second.mp4' })).toBeAttached();
  await expect(page.getByRole('button', { name: 'Previous video' })).toHaveAttribute(
    'title',
    'Previous video (Ctrl+Page Up)',
  );
  await expect(page.getByRole('button', { name: 'Next video' })).toHaveAttribute(
    'title',
    'Next video (Ctrl+Page Down)',
  );
  await expect(page.locator('.video-stage')).toHaveAttribute(
    'title',
    'Fullscreen preview (F10)',
  );
  await expect(page.getByRole('button', { name: 'Play', exact: true })).toHaveAttribute(
    'title',
    'Play (K)',
  );
  await expect(page.locator('.source-summary small')).toHaveAttribute(
    'title',
    'Compatible MP4: Alt+4 · Lossless FFV1 / MKV: Alt+2 · Metadata only: Alt+3',
  );
  await expect(page.getByRole('button', { name: 'Save options' })).toHaveAttribute(
    'title',
    'Save options (Ctrl+K / Ctrl+Shift+K)',
  );
  await page.getByRole('button', { name: 'Save options' }).click();
  await expect(page.getByRole('menuitem', { name: 'Save a copy' })).toHaveAttribute(
    'title',
    'Save a copy (Ctrl+K)',
  );
  await expect(page.getByRole('menuitem', { name: 'Save', exact: true })).toHaveAttribute(
    'title',
    'Save (Ctrl+Shift+K)',
  );
  await page.getByRole('button', { name: 'Save options' }).click();
  await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
  await page.keyboard.press('k');
  await expect.poll(() => page.evaluate(() => (window as any).__vidmetryPlayCount)).toBe(1);
  await page.keyboard.press('Alt+2');
  await expect(page.getByText('Export: Lossless FFV1 / MKV')).toBeVisible();
  await page.keyboard.press('Alt+3');
  await expect(page.getByText('Export: Metadata only')).toBeVisible();
});

test('folder navigation, save options, success alignment, and Explorer selection work together', async ({
  page,
}) => {
  await page.goto('/?betaPicker=1');
  await page.getByRole('button', { name: 'Open folder' }).click();
  await expect(page.getByRole('option', { name: '2. second.mp4' })).toBeAttached();
  expect(
    await page.evaluate(() => {
      const invocations = (window as any).__vidmetryInvocations as Array<{
        command: string;
        args: { options?: unknown; path?: string };
      }>;
      return {
        picker: invocations.find((item) => item.command === 'pick_video_folder'),
        inspection: invocations.find(
          (item) => item.command === 'inspect_selection' && item.args.path === 'C:\\clips',
        ),
      };
    }),
  ).toEqual({
    picker: {
      command: 'pick_video_folder',
      args: {
        title: 'Select folder',
        selectFolderLabel: 'Select folder',
        cancelLabel: 'Cancel',
        initialDirectory: null,
        initialView: { viewMode: 1, iconSize: 96 },
        viewLabels: {
          view: 'View',
          extraLargeIcons: 'Extra large icons',
          largeIcons: 'Large icons',
          mediumIcons: 'Medium icons',
          smallIcons: 'Small icons',
          list: 'List',
          details: 'Details',
          tiles: 'Tiles',
          content: 'Content',
        },
      },
    },
    inspection: { command: 'inspect_selection', args: { path: 'C:\\clips' } },
  });

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

  await page.getByRole('link', { name: /Show saved file in File Explorer:/ }).click();
  const revealed = await page.evaluate(() =>
    (window as any).__vidmetryInvocations.some(
      (item: { command: string; args: { path?: string } }) =>
        item.command === 'reveal_in_explorer' && item.args.path === 'C:\\clips\\sample_cropped.mp4',
    ),
  );
  expect(revealed).toBe(true);
});

test('an open folder refreshes after Explorer additions and completed copy saves', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Open folder' }).click();
  await expect(page.getByRole('option', { name: '2. second.mp4' })).toBeAttached();

  await page.evaluate(() => {
    (window as any).__setDirectoryVideos([
      'C:\\clips\\first.mp4',
      'C:\\clips\\second.mp4',
      'C:\\clips\\third.mp4',
    ]);
    (window as any).__emitTauri('directory-changed', { rootPath: 'c:\\CLIPS' });
  });
  await expect(page.getByRole('option', { name: '3. third.mp4' })).toBeAttached();

  await page.getByRole('button', { name: 'Save options' }).click();
  await page.getByRole('menuitem', { name: 'Save a copy' }).click();
  await expect(page.getByRole('button', { name: 'Cancel' })).toBeVisible();
  await page.evaluate(() => {
    (window as any).__setDirectoryVideos([
      'C:\\clips\\first.mp4',
      'C:\\clips\\sample_cropped.mp4',
      'C:\\clips\\second.mp4',
      'C:\\clips\\third.mp4',
    ]);
    (window as any).__emitTauri('export-complete', {
      jobId: 'job-1',
      outputPath: 'C:\\clips\\sample_cropped.mp4',
    });
  });
  await expect(page.getByRole('option', { name: '2. sample_cropped.mp4' })).toBeAttached();
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
  const startPosition = await start.evaluate((element) => getComputedStyle(element).left);
  const endPosition = await end.evaluate((element) => getComputedStyle(element).left);

  await page.getByRole('button', { name: 'Save options' }).click();
  await page.getByRole('menuitem', { name: 'Save a copy' }).click();
  await expect(start).toBeDisabled();
  await expect(end).toBeDisabled();
  await expect(start).toHaveAttribute('aria-valuenow', '11');
  await expect(end).toHaveAttribute('aria-valuenow', '229');
  expect(await start.evaluate((element) => getComputedStyle(element).left)).toBe(startPosition);
  expect(await end.evaluate((element) => getComputedStyle(element).left)).toBe(endPosition);
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
