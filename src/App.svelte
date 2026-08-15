<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { convertFileSrc, invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { open, save } from '@tauri-apps/plugin-dialog';

  import {
    aspectRatio,
    cropStyle,
    dragCrop,
    fitAspect,
    fullFrame,
    sanitizeRect,
    screenDeltaToSource,
    type AspectPreset,
    type CropBounds,
    type CropHandle,
    type CropRect,
  } from './lib/crop';
  import {
    canSaveInPlace,
    clampProgress,
    suggestOutput,
    type AudioMode,
    type EncoderPreset,
    type ExportCompleteEvent,
    type ExportErrorEvent,
    type ExportProfile,
    type ExportProgressEvent,
    type FrameRateMode,
    type PixelFormat,
    type VideoCodec,
  } from './lib/export';
  import { translate, type TranslationKey } from './lib/i18n';
  import { formatFrameRate, formatTime, type MediaDescriptor } from './lib/media';
  import {
    cloneSettings,
    defaultSettings,
    loadSettings,
    normalizeSettings,
    persistSettings,
    resolveLanguage,
    type AppSettings,
    type Language,
    type LanguageMode,
  } from './lib/settings';

  const handles: Array<{ value: CropHandle; ja: string; en: string }> = [
    { value: 'north-west', ja: '左上を調整', en: 'Resize from top left' },
    { value: 'north', ja: '上辺を調整', en: 'Resize top edge' },
    { value: 'north-east', ja: '右上を調整', en: 'Resize from top right' },
    { value: 'east', ja: '右辺を調整', en: 'Resize right edge' },
    { value: 'south-east', ja: '右下を調整', en: 'Resize from bottom right' },
    { value: 'south', ja: '下辺を調整', en: 'Resize bottom edge' },
    { value: 'south-west', ja: '左下を調整', en: 'Resize from bottom left' },
    { value: 'west', ja: '左辺を調整', en: 'Resize left edge' },
  ];

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

  interface FrameGeometry {
    left: number;
    top: number;
    width: number;
    height: number;
  }

  interface DragState {
    handle: CropHandle;
    pointerX: number;
    pointerY: number;
    start: CropRect;
    renderedWidth: number;
    renderedHeight: number;
  }

  interface SelectionDescriptor {
    kind: 'file' | 'directory';
    rootPath: string;
    videoPaths: string[];
  }

  let media: MediaDescriptor | null = null;
  let crop: CropRect = { x: 0, y: 0, width: 16, height: 16 };
  let aspect: AspectPreset = 'free';
  let videoSrc = '';
  let videoElement: HTMLVideoElement;
  let stageWidth = 0;
  let stageHeight = 0;
  let currentTime = 0;
  let isPlaying = false;
  let isMuted = false;
  let isLoading = false;
  let isPreparingProxy = false;
  let usingProxy = false;
  let errorMessage = '';
  let successMessage = '';
  let dragState: DragState | null = null;
  let seekFrame: number | null = null;
  let unlistenDragDrop: UnlistenFn | undefined;
  let unlistenExportEvents: UnlistenFn[] = [];
  let playlist: string[] = [];
  let playlistIndex = 0;
  let directoryPath: string | null = null;
  let showSaveMenu = false;
  let exportJobId: string | null = null;
  let exportProgress = 0;
  let exportOutTime = 0;
  let isStartingExport = false;
  let inPlaceExportPath: string | null = null;
  let showSettings = false;
  let settings: AppSettings = cloneSettings(defaultSettings);
  let settingsDraft: AppSettings = cloneSettings(defaultSettings);
  let systemLanguage = 'en-US';
  let language: Language = 'en';
  let text = (key: TranslationKey, values: Record<string, string | number> = {}) =>
    translate(language, key, values);

  $: bounds = media
    ? { width: media.displayWidth, height: media.displayHeight }
    : { width: 16, height: 16 };
  $: frameGeometry = containFrame(stageWidth, stageHeight, bounds);
  $: frameStyle = `left:${frameGeometry.left}px;top:${frameGeometry.top}px;width:${frameGeometry.width}px;height:${frameGeometry.height}px`;
  $: boxStyle = cropStyle(crop, bounds);
  $: activeRatio = aspectRatio(aspect, bounds);
  $: duration = Math.max(media?.durationSeconds ?? 0, videoElement?.duration || 0);
  $: language = resolveLanguage(showSettings ? settingsDraft : settings, systemLanguage);
  $: text = (key: TranslationKey, values: Record<string, string | number> = {}) =>
    translate(language, key, values);
  $: if (typeof document !== 'undefined') document.documentElement.lang = language;
  $: profileSupported =
    media !== null &&
    (settings.export.profile !== 'metadata' || media.metadataCropSupported);
  $: canOverwrite =
    media !== null &&
    profileSupported &&
    canSaveInPlace(media.sourcePath, settings.export.profile);
  $: profileLabel = profileName(settings.export.profile);

  onMount(() => {
    systemLanguage = navigator.language || 'en-US';
    settings = loadSettings();
    settingsDraft = cloneSettings(settings);

    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type !== 'drop' || event.payload.paths.length === 0) return;
        if (event.payload.paths.length === 1) {
          void loadSelection(event.payload.paths[0]);
        } else {
          playlist = [...event.payload.paths];
          directoryPath = null;
          playlistIndex = 0;
          void loadVideo(playlist[0], true);
        }
      })
      .then((unlisten) => {
        unlistenDragDrop = unlisten;
      })
      .catch(() => undefined);

    void Promise.all([
      listen<ExportProgressEvent>('export-progress', (event) => {
        if (event.payload.jobId !== exportJobId) return;
        exportProgress = clampProgress(event.payload.fraction);
        exportOutTime = event.payload.outTimeSeconds;
      }),
      listen<ExportCompleteEvent>('export-complete', (event) => {
        if (event.payload.jobId !== exportJobId) return;
        void handleExportComplete(event.payload);
      }),
      listen<ExportErrorEvent>('export-error', (event) => {
        if (event.payload.jobId !== exportJobId) return;
        const shouldRestore = inPlaceExportPath !== null;
        exportJobId = null;
        exportProgress = 0;
        inPlaceExportPath = null;
        if (shouldRestore) restoreSourcePreview();
        if (!event.payload.cancelled) {
          errorMessage = `${text('exportFailed')}${event.payload.message}`;
        }
      }),
    ]).then((unlisteners) => {
      unlistenExportEvents = unlisteners;
    });

    window.addEventListener('keydown', handleKeyboard);
    window.addEventListener('pointerdown', closeSaveMenuFromOutside);
  });

  onDestroy(() => {
    unlistenDragDrop?.();
    for (const unlisten of unlistenExportEvents) unlisten();
    window.removeEventListener('keydown', handleKeyboard);
    window.removeEventListener('pointerdown', closeSaveMenuFromOutside);
    endCropDrag();
    if (seekFrame !== null) cancelAnimationFrame(seekFrame);
  });

  function observeStage(node: HTMLElement) {
    const observer = new ResizeObserver(([entry]) => {
      if (!entry) return;
      stageWidth = entry.contentRect.width;
      stageHeight = entry.contentRect.height;
    });
    observer.observe(node);
    return { destroy: () => observer.disconnect() };
  }

  function containFrame(width: number, height: number, source: CropBounds): FrameGeometry {
    if (width <= 0 || height <= 0 || source.width <= 0 || source.height <= 0) {
      return { left: 0, top: 0, width: 0, height: 0 };
    }
    const sourceRatio = source.width / source.height;
    const stageRatio = width / height;
    if (stageRatio > sourceRatio) {
      const frameWidth = height * sourceRatio;
      return { left: (width - frameWidth) / 2, top: 0, width: frameWidth, height };
    }
    const frameHeight = width / sourceRatio;
    return { left: 0, top: (height - frameHeight) / 2, width, height: frameHeight };
  }

  async function chooseVideo() {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [
        {
          name: text('selectVideoFilter'),
          extensions: ['mp4', 'mov', 'mkv', 'webm', 'avi', 'm4v', 'wmv', 'mts', 'm2ts', 'mpg', 'mpeg'],
        },
      ],
    });
    if (typeof selected === 'string') await loadSelection(selected);
  }

  async function chooseDirectory() {
    const selected = await open({ multiple: false, directory: true });
    if (typeof selected === 'string') await loadSelection(selected);
  }

  async function loadSelection(path: string) {
    if (isLoading || isPreparingProxy || exportJobId) return;
    try {
      const selection = await invoke<SelectionDescriptor>('inspect_selection', { path });
      playlist = selection.videoPaths;
      directoryPath = selection.kind === 'directory' ? selection.rootPath : null;
      playlistIndex = 0;
      await loadVideo(playlist[0], true);
    } catch (error) {
      errorMessage = readableError(error);
    }
  }

  async function loadVideo(path: string, keepPlaylist = false) {
    if (isLoading || isPreparingProxy || exportJobId) return;
    isLoading = true;
    errorMessage = '';
    successMessage = '';
    usingProxy = false;
    currentTime = 0;
    isPlaying = false;
    showSaveMenu = false;
    videoElement?.pause();

    try {
      const descriptor = await invoke<MediaDescriptor>('probe_video', { path });
      media = descriptor;
      crop = fullFrame({ width: descriptor.displayWidth, height: descriptor.displayHeight });
      aspect = 'free';
      videoSrc = convertFileSrc(descriptor.sourcePath);
      if (!keepPlaylist) {
        playlist = [descriptor.sourcePath];
        playlistIndex = 0;
        directoryPath = null;
      } else {
        const found = playlist.findIndex(
          (item) => item.toLocaleLowerCase() === descriptor.sourcePath.toLocaleLowerCase(),
        );
        if (found >= 0) playlistIndex = found;
      }
      await tick();
      videoElement?.load();
    } catch (error) {
      errorMessage = readableError(error);
    } finally {
      isLoading = false;
    }
  }

  async function navigatePlaylist(offset: number) {
    if (playlist.length < 2 || isLoading || exportJobId) return;
    const next = Math.min(playlist.length - 1, Math.max(0, playlistIndex + offset));
    if (next === playlistIndex) return;
    playlistIndex = next;
    await loadVideo(playlist[next], true);
  }

  async function selectPlaylistVideo(event: Event) {
    const next = Number((event.currentTarget as HTMLSelectElement).value);
    if (!Number.isInteger(next) || !playlist[next] || next === playlistIndex) return;
    playlistIndex = next;
    await loadVideo(playlist[next], true);
  }

  async function handleVideoError() {
    if (!media || usingProxy || isPreparingProxy || isLoading || !videoSrc) return;
    isPreparingProxy = true;
    errorMessage = '';
    try {
      const proxyPath = await invoke<string>('create_preview', { path: media.sourcePath });
      usingProxy = true;
      videoSrc = convertFileSrc(proxyPath);
      await tick();
      videoElement?.load();
    } catch (error) {
      errorMessage = `${text('previewFailed')}${readableError(error)}`;
    } finally {
      isPreparingProxy = false;
    }
  }

  function handleLoadedMetadata() {
    if (media && media.durationSeconds <= 0 && Number.isFinite(videoElement.duration)) {
      media = { ...media, durationSeconds: videoElement.duration };
    }
  }

  async function togglePlayback() {
    if (!videoElement || !media) return;
    if (videoElement.paused) {
      try {
        await videoElement.play();
      } catch (error) {
        errorMessage = readableError(error);
      }
    } else {
      videoElement.pause();
    }
  }

  function scrubTo(event: Event) {
    const next = Number((event.currentTarget as HTMLInputElement).value);
    currentTime = next;
    if (seekFrame !== null) cancelAnimationFrame(seekFrame);
    seekFrame = requestAnimationFrame(() => {
      if (videoElement && Number.isFinite(next)) videoElement.currentTime = next;
      seekFrame = null;
    });
  }

  function toggleMute() {
    isMuted = !isMuted;
    if (videoElement) videoElement.muted = isMuted;
  }

  function toggleLoop() {
    settings = { ...settings, loopPlayback: !settings.loopPlayback };
    persistSettings(settings);
    if (videoElement) videoElement.loop = settings.loopPlayback;
  }

  function handleKeyboard(event: KeyboardEvent) {
    const target = event.target instanceof Element ? event.target : null;
    if (event.code === 'Escape') {
      showSaveMenu = false;
      if (!exportJobId) showSettings = false;
      return;
    }
    if (showSettings || !media) return;
    if (event.code === 'PageUp' || event.code === 'PageDown') {
      if (target?.matches('input, select, textarea')) return;
      event.preventDefault();
      void navigatePlaylist(event.code === 'PageDown' ? 1 : -1);
      return;
    }
    if (target?.matches('input, select, textarea, button')) return;
    if (event.code === 'Space') {
      event.preventDefault();
      void togglePlayback();
      return;
    }
    if (event.code === 'ArrowLeft' || event.code === 'ArrowRight') {
      event.preventDefault();
      const direction = event.code === 'ArrowRight' ? 1 : -1;
      const amount = event.shiftKey ? 10 : 1;
      const next = Math.min(duration, Math.max(0, currentTime + direction * amount));
      currentTime = next;
      if (videoElement) videoElement.currentTime = next;
    }
  }

  function closeSaveMenuFromOutside(event: PointerEvent) {
    if (!(event.target instanceof Element) || event.target.closest('.split-button')) return;
    showSaveMenu = false;
  }

  function beginCropDrag(event: PointerEvent, handle: CropHandle) {
    if (!media || event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    dragState = {
      handle,
      pointerX: event.clientX,
      pointerY: event.clientY,
      start: { ...crop },
      renderedWidth: frameGeometry.width,
      renderedHeight: frameGeometry.height,
    };
    window.addEventListener('pointermove', continueCropDrag);
    window.addEventListener('pointerup', endCropDrag, { once: true });
  }

  function continueCropDrag(event: PointerEvent) {
    if (!dragState || !media) return;
    const delta = screenDeltaToSource(
      event.clientX - dragState.pointerX,
      event.clientY - dragState.pointerY,
      dragState.renderedWidth,
      dragState.renderedHeight,
      bounds,
    );
    crop = dragCrop(dragState.start, dragState.handle, delta.x, delta.y, bounds, activeRatio);
  }

  function endCropDrag() {
    dragState = null;
    window.removeEventListener('pointermove', continueCropDrag);
    window.removeEventListener('pointerup', endCropDrag);
  }

  function setAspect(event: Event) {
    aspect = (event.currentTarget as HTMLSelectElement).value as AspectPreset;
    const ratio = aspectRatio(aspect, bounds);
    if (ratio) crop = fitAspect(crop, ratio, bounds);
  }

  function updateCropField(field: keyof CropRect, event: Event) {
    const value = Number((event.currentTarget as HTMLInputElement).value);
    if (!Number.isFinite(value)) return;
    crop = sanitizeRect({ ...crop, [field]: value }, bounds);
  }

  function resetCrop() {
    crop = fullFrame(bounds);
    aspect = 'free';
  }

  async function saveCopy() {
    showSaveMenu = false;
    if (!media || !profileSupported || exportJobId || isStartingExport) return;
    const suggestion = suggestOutput(media.sourcePath, settings.export.profile);
    const outputPath = await save({
      defaultPath: suggestion.path,
      filters: [
        {
          name: text('outputVideoFilter', { extension: suggestion.extension.toUpperCase() }),
          extensions: [suggestion.extension],
        },
      ],
    });
    if (outputPath) await startExport(outputPath, false);
  }

  async function saveInPlace() {
    showSaveMenu = false;
    if (!media || !canOverwrite || exportJobId || isStartingExport) return;
    if (!window.confirm(text('overwriteConfirm', { name: media.fileName }))) return;
    await startExport(media.sourcePath, true);
  }

  async function startExport(outputPath: string, inPlace: boolean) {
    if (!media) return;
    const sourcePath = media.sourcePath;
    isStartingExport = true;
    errorMessage = '';
    successMessage = '';
    try {
      videoElement?.pause();
      if (inPlace) {
        inPlaceExportPath = sourcePath;
        videoSrc = '';
        await tick();
        videoElement?.load();
      }
      exportProgress = 0;
      exportOutTime = 0;
      exportJobId = await invoke<string>('start_export', {
        request: {
          sourcePath,
          outputPath,
          crop,
          settings: settings.export,
          overwrite: true,
          inPlace,
        },
      });
    } catch (error) {
      if (inPlace) restoreSourcePreview();
      inPlaceExportPath = null;
      errorMessage = `${text('exportStartError')}${readableError(error)}`;
    } finally {
      isStartingExport = false;
    }
  }

  async function handleExportComplete(event: ExportCompleteEvent) {
    exportProgress = 1;
    exportJobId = null;
    const replacedSource = inPlaceExportPath !== null;
    inPlaceExportPath = null;
    successMessage = text('saved', { path: event.outputPath });
    if (replacedSource) await loadVideo(event.outputPath, true);
  }

  async function cancelExport() {
    if (!exportJobId) return;
    try {
      await invoke('cancel_export', { jobId: exportJobId });
    } catch (error) {
      errorMessage = readableError(error);
    }
  }

  function restoreSourcePreview() {
    if (!media) return;
    usingProxy = false;
    videoSrc = convertFileSrc(media.sourcePath);
    void tick().then(() => videoElement?.load());
  }

  function openSettingsDialog() {
    settingsDraft = cloneSettings(settings);
    showSaveMenu = false;
    showSettings = true;
  }

  function closeSettingsDialog() {
    settingsDraft = cloneSettings(settings);
    showSettings = false;
  }

  function applySettings() {
    settings = normalizeSettings(settingsDraft);
    persistSettings(settings);
    showSettings = false;
    if (videoElement) videoElement.loop = settings.loopPlayback;
  }

  function updateDraft<K extends keyof AppSettings>(key: K, value: AppSettings[K]) {
    settingsDraft = { ...settingsDraft, [key]: value };
  }

  function updateExportDraft<K extends keyof AppSettings['export']>(
    key: K,
    value: AppSettings['export'][K],
  ) {
    settingsDraft = { ...settingsDraft, export: { ...settingsDraft.export, [key]: value } };
  }

  function setProfile(profile: ExportProfile) {
    const next = { ...settingsDraft.export, profile };
    if (profile === 'compatible') {
      if (next.audioMode === 'flac' || next.audioMode === 'pcm') next.audioMode = 'auto';
      if (next.pixelFormat === 'source') next.pixelFormat = 'yuv420p';
    } else if (profile === 'lossless') {
      next.pixelFormat = 'source';
    }
    settingsDraft = { ...settingsDraft, export: next };
  }

  function profileName(profile: ExportProfile): string {
    return text(profile);
  }

  function fileName(path: string): string {
    const separator = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
    return separator >= 0 ? path.slice(separator + 1) : path;
  }

  function readableError(error: unknown): string {
    if (typeof error === 'string') return error;
    if (error instanceof Error) return error.message;
    return text('unknownError');
  }
</script>

<svelte:head>
  <title>{media ? `${media.fileName} — Vidmetry` : 'Vidmetry'}</title>
</svelte:head>

<div class="app-shell" class:has-media={media !== null}>
  <header class="app-header">
    <div class="brand" aria-label="Vidmetry">
      <span class="brand-mark" aria-hidden="true">V</span>
      <span>Vidmetry</span>
    </div>

    {#if media}
      <div class="source-summary" title={media.sourcePath}>
        <strong>{media.fileName}</strong>
        <span>{media.displayWidth} × {media.displayHeight} · {formatFrameRate(media.frameRate)} · {media.videoCodec.toUpperCase()}</span>
        <small>{text('activeProfile', { profile: profileLabel })}</small>
      </div>
    {:else}
      <div class="source-summary"><span>{text('localVideoCropper')}</span></div>
    {/if}

    <div class="header-actions">
      <button class="button secondary" type="button" onclick={chooseVideo} disabled={isLoading || isPreparingProxy || exportJobId !== null}>
        {media ? text('openAnother') : text('openVideo')}
      </button>
      <button class="square-button" type="button" aria-label={text('openFolder')} title={text('openFolder')} onclick={chooseDirectory} disabled={isLoading || isPreparingProxy || exportJobId !== null}>▣</button>
      <button class="square-button settings-button" type="button" aria-label={text('settings')} title={text('settings')} onclick={openSettingsDialog}>⚙</button>
      <div class="split-button">
        <button class="button primary export-main" type="button" disabled={!profileSupported || exportJobId !== null || isStartingExport} onclick={saveCopy} title={!profileSupported ? text('metadataUnavailable') : text('copySave')}>
          {text('export')}
        </button>
        <button class="button primary export-toggle" type="button" aria-label={text('exportMenu')} disabled={!profileSupported || exportJobId !== null || isStartingExport} onclick={() => (showSaveMenu = !showSaveMenu)}>▾</button>
        {#if showSaveMenu}
          <div class="save-menu">
            <button type="button" onclick={saveCopy}>
              <strong>{text('copySave')}</strong>
              <small>.{media ? suggestOutput(media.sourcePath, settings.export.profile).extension : ''}</small>
            </button>
            <button type="button" onclick={saveInPlace} disabled={!canOverwrite} title={!canOverwrite ? text('saveUnavailable') : text('saveHint')}>
              <strong>{text('save')}</strong>
              <small>{canOverwrite ? text('saveHint') : text('saveUnavailable')}</small>
            </button>
          </div>
        {/if}
      </div>
    </div>
  </header>

  {#if media}
    <main class="editor-grid">
      <section class="stage-panel" aria-label="Video preview">
        {#if playlist.length > 1}
          <div class="playlist-bar" title={directoryPath ?? ''}>
            <button type="button" aria-label={text('previousVideo')} title={`${text('previousVideo')} (Page Up)`} disabled={playlistIndex === 0 || isLoading || exportJobId !== null} onclick={() => navigatePlaylist(-1)}>‹</button>
            <label>
              <span>{text('chooseFromFolder')}</span>
              <select value={playlistIndex} onchange={selectPlaylistVideo} disabled={isLoading || exportJobId !== null}>
                {#each playlist as path, index}
                  <option value={index}>{index + 1}. {fileName(path)}</option>
                {/each}
              </select>
            </label>
            <span class="playlist-count">{text('folderPosition', { current: playlistIndex + 1, total: playlist.length })}</span>
            <button type="button" aria-label={text('nextVideo')} title={`${text('nextVideo')} (Page Down)`} disabled={playlistIndex === playlist.length - 1 || isLoading || exportJobId !== null} onclick={() => navigatePlaylist(1)}>›</button>
          </div>
        {/if}

        <div class="video-stage" use:observeStage>
          <div class="video-frame" style={frameStyle}>
            <video
              bind:this={videoElement}
              src={videoSrc}
              playsinline
              preload="metadata"
              loop={settings.loopPlayback}
              onerror={handleVideoError}
              onloadedmetadata={handleLoadedMetadata}
              ontimeupdate={() => (currentTime = videoElement.currentTime)}
              onplay={() => (isPlaying = true)}
              onpause={() => (isPlaying = false)}
              onended={() => (isPlaying = false)}
            ><track kind="captions" /></video>

            <div class="crop-layer">
              <div class="crop-box" class:is-dragging={dragState?.handle === 'move'} style={boxStyle} role="presentation" onpointerdown={(event) => beginCropDrag(event, 'move')}>
                <div class="thirds vertical one"></div><div class="thirds vertical two"></div>
                <div class="thirds horizontal one"></div><div class="thirds horizontal two"></div>
                {#each handles as handle}
                  <button class="crop-handle {handle.value}" type="button" aria-label={language === 'ja' ? handle.ja : handle.en} onpointerdown={(event) => beginCropDrag(event, handle.value)}></button>
                {/each}
              </div>
            </div>
          </div>

          {#if isLoading || isPreparingProxy}
            <div class="stage-status" role="status">
              <span class="spinner"></span>
              <strong>{isPreparingProxy ? text('preparingProxy') : text('analyzing')}</strong>
              {#if isPreparingProxy}<small>{text('sourceUnchanged')}</small>{/if}
            </div>
          {/if}
        </div>
        {#if usingProxy}<p class="proxy-note">{text('proxyPreview')}</p>{/if}
      </section>

      <aside class="inspector" aria-label={text('cropArea')}>
        <div class="inspector-heading">
          <div><span class="section-label">{text('frame')}</span><h2>{text('cropArea')}</h2></div>
          <button class="text-button" type="button" onclick={resetCrop}>{text('reset')}</button>
        </div>

        <label class="field full-width">
          <span>{text('aspectRatio')}</span>
          <select value={aspect} onchange={setAspect}>
            <option value="free">{text('free')}</option><option value="source">{text('sourceRatio')}</option>
            <option value="1:1">1 : 1</option><option value="4:3">4 : 3</option>
            <option value="16:9">16 : 9</option><option value="9:16">9 : 16</option>
          </select>
        </label>

        <div class="field-grid">
          <label class="field"><span>X</span><div class="number-input"><input type="number" min="0" step="2" value={crop.x} onchange={(event) => updateCropField('x', event)} /><em>px</em></div></label>
          <label class="field"><span>Y</span><div class="number-input"><input type="number" min="0" step="2" value={crop.y} onchange={(event) => updateCropField('y', event)} /><em>px</em></div></label>
          <label class="field"><span>{text('width')}</span><div class="number-input"><input type="number" min="16" step="2" value={crop.width} onchange={(event) => updateCropField('width', event)} /><em>px</em></div></label>
          <label class="field"><span>{text('height')}</span><div class="number-input"><input type="number" min="16" step="2" value={crop.height} onchange={(event) => updateCropField('height', event)} /><em>px</em></div></label>
        </div>

        <div class="output-size"><span>{text('outputFrame')}</span><strong>{crop.width} × {crop.height}</strong></div>
        <div class="divider"></div>
        <div class="media-details">
          <span class="section-label">{text('source')}</span>
          <dl>
            <div><dt>{text('codec')}</dt><dd>{media.videoCodec.toUpperCase()}</dd></div>
            <div><dt>{text('pixelFormat')}</dt><dd>{media.pixelFormat}{media.bitDepth ? ` · ${media.bitDepth}bit` : ''}</dd></div>
            <div><dt>{text('rotation')}</dt><dd>{media.rotationDegrees}°</dd></div>
            <div><dt>{text('audio')}</dt><dd>{media.audioCodec?.toUpperCase() ?? text('none')}</dd></div>
          </dl>
        </div>
      </aside>
    </main>

    <footer class="transport">
      <button class="icon-button" type="button" aria-label={isPlaying ? text('pause') : text('play')} onclick={togglePlayback}>{isPlaying ? 'Ⅱ' : '▶'}</button>
      <span class="time current">{formatTime(currentTime)}</span>
      <input class="scrubber" type="range" aria-label="Seek" min="0" max={duration || 0} step="0.001" value={currentTime} oninput={scrubTo} />
      <span class="time">{formatTime(duration)}</span>
      <button class="icon-button transport-option" class:active={settings.loopPlayback} type="button" aria-label={settings.loopPlayback ? text('disableLoop') : text('enableLoop')} title={settings.loopPlayback ? text('disableLoop') : text('enableLoop')} onclick={toggleLoop}>↻</button>
      <button class="icon-button transport-option" type="button" aria-label={isMuted ? text('unmute') : text('mute')} onclick={toggleMute}>{isMuted ? '×' : '♪'}</button>
    </footer>
  {:else}
    <main class="empty-state" use:observeStage>
      <div class="empty-visual" aria-hidden="true">
        <span class="corner top-left"></span><span class="corner top-right"></span>
        <span class="corner bottom-left"></span><span class="corner bottom-right"></span>
        <span class="play-symbol">▶</span>
      </div>
      <p class="eyebrow">{text('emptyEyebrow')}</p>
      <h1>{text('emptyTitle')}</h1>
      <p>{text('emptyDescription')}</p>
      <div class="empty-actions">
        <button class="button primary large" type="button" onclick={chooseVideo} disabled={isLoading}>{text('openVideo')}</button>
        <button class="button secondary large" type="button" onclick={chooseDirectory} disabled={isLoading}>{text('openFolder')}</button>
      </div>
      <span class="shortcut">{text('supportedFormats')}</span>
    </main>
  {/if}

  {#if exportJobId && media}
    <div class="inline-export" role="status">
      <div><strong>{text('exporting')}</strong><span>{Math.round(exportProgress * 100)}% · {formatTime(exportOutTime)} / {formatTime(media.durationSeconds)}</span></div>
      <div class="inline-progress"><span style={`width:${exportProgress * 100}%`}></span></div>
      <button type="button" onclick={cancelExport}>{text('cancel')}</button>
    </div>
  {/if}

  {#if showSettings}
    <div class="modal-backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && closeSettingsDialog()}>
      <div class="settings-dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title">
        <div class="dialog-heading">
          <div><span class="section-label">SETTINGS</span><h2 id="settings-title">{text('settingsTitle')}</h2><p>{text('settingsDescription')}</p></div>
          <button class="dialog-close" type="button" aria-label={text('close')} onclick={closeSettingsDialog}>×</button>
        </div>

        <div class="settings-scroll">
          <section class="settings-section">
            <h3>{text('language')}</h3>
            <div class="radio-row">
              <label><input type="radio" name="language-mode" checked={settingsDraft.languageMode === 'system'} onchange={() => updateDraft('languageMode', 'system' as LanguageMode)} />{text('languageSystem')}</label>
              <label><input type="radio" name="language-mode" checked={settingsDraft.languageMode === 'manual'} onchange={() => updateDraft('languageMode', 'manual' as LanguageMode)} />{text('languageManual')}</label>
            </div>
            <label class="settings-field compact"><span>{text('language')}</span><select value={settingsDraft.language} disabled={settingsDraft.languageMode !== 'manual'} onchange={(event) => updateDraft('language', (event.currentTarget as HTMLSelectElement).value as Language)}><option value="ja">{text('japanese')}</option><option value="en">{text('english')}</option></select></label>
          </section>

          <section class="settings-section">
            <h3>{text('saveMethod')}</h3>
            <div class="profile-settings">
              {#each ['compatible', 'lossless', 'metadata'] as profile}
                <button class:active={settingsDraft.export.profile === profile} type="button" onclick={() => setProfile(profile as ExportProfile)}>
                  <strong>{profileName(profile as ExportProfile)}</strong>
                  <small>{text(`${profile}Description` as TranslationKey)}</small>
                </button>
              {/each}
            </div>
          </section>

          {#if settingsDraft.export.profile !== 'metadata'}
            <section class="settings-section">
              <h3>{text('encodingSettings')}</h3>
              <div class="settings-grid">
                {#if settingsDraft.export.profile === 'compatible'}
                  <label class="settings-field"><span>{text('videoCodec')}</span><select value={settingsDraft.export.videoCodec} onchange={(event) => updateExportDraft('videoCodec', (event.currentTarget as HTMLSelectElement).value as VideoCodec)}><option value="h264">H.264</option><option value="h265">H.265 / HEVC</option></select></label>
                  <label class="settings-field"><span>{text('encoder')}</span><input value={text('softwareEncoder', { encoder: settingsDraft.export.videoCodec === 'h264' ? 'libx264' : 'libx265' })} disabled /></label>
                  <label class="settings-field"><span>{text('crf')}</span><input type="number" min="0" max="51" step="1" value={settingsDraft.export.crf} onchange={(event) => updateExportDraft('crf', Number((event.currentTarget as HTMLInputElement).value))} /><small>{text('crfHint')}</small></label>
                  <label class="settings-field"><span>{text('preset')}</span><select value={settingsDraft.export.preset} onchange={(event) => updateExportDraft('preset', (event.currentTarget as HTMLSelectElement).value as EncoderPreset)}>{#each presets as preset}<option value={preset}>{preset}</option>{/each}</select></label>
                {/if}
                <label class="settings-field"><span>{text('pixelFormatSetting')}</span><select value={settingsDraft.export.pixelFormat} onchange={(event) => updateExportDraft('pixelFormat', (event.currentTarget as HTMLSelectElement).value as PixelFormat)}>{#each pixelFormats as format}<option value={format}>{format === 'source' ? text('sourcePixelFormat') : format}</option>{/each}</select></label>
              </div>
            </section>

            <section class="settings-section">
              <h3>{text('audioSettings')}</h3>
              <div class="settings-grid">
                <label class="settings-field"><span>{text('audioMode')}</span><select value={settingsDraft.export.audioMode} onchange={(event) => updateExportDraft('audioMode', (event.currentTarget as HTMLSelectElement).value as AudioMode)}><option value="auto">{text('audioAuto')}</option><option value="copy">{text('audioCopy')}</option><option value="aac">{text('audioAac')}</option>{#if settingsDraft.export.profile === 'lossless'}<option value="flac">{text('audioFlac')}</option><option value="pcm">{text('audioPcm')}</option>{/if}<option value="none">{text('audioNone')}</option></select></label>
                <label class="settings-field"><span>{text('audioBitrate')}</span><div class="unit-input"><input type="number" min="32" max="1024" step="8" value={settingsDraft.export.audioBitrateKbps} disabled={settingsDraft.export.audioMode !== 'aac'} onchange={(event) => updateExportDraft('audioBitrateKbps', Number((event.currentTarget as HTMLInputElement).value))} /><em>kbps</em></div></label>
              </div>
            </section>

            <section class="settings-section">
              <h3>{text('timingSettings')}</h3>
              <div class="settings-grid">
                <label class="settings-field"><span>{text('frameRateMode')}</span><select value={settingsDraft.export.frameRateMode} onchange={(event) => updateExportDraft('frameRateMode', (event.currentTarget as HTMLSelectElement).value as FrameRateMode)}><option value="passthrough">{text('fpsPassthrough')}</option><option value="constant">{text('fpsConstant')}</option></select></label>
                <label class="settings-field"><span>{text('constantFps')}</span><div class="unit-input"><input type="number" min="1" max="240" step="0.001" value={settingsDraft.export.constantFrameRate} disabled={settingsDraft.export.frameRateMode !== 'constant'} onchange={(event) => updateExportDraft('constantFrameRate', Number((event.currentTarget as HTMLInputElement).value))} /><em>fps</em></div></label>
              </div>
            </section>

            <section class="settings-section">
              <h3>{text('fileSettings')}</h3>
              <div class="check-list">
                {#if settingsDraft.export.profile === 'compatible'}<label><input type="checkbox" checked={settingsDraft.export.fastStart} onchange={(event) => updateExportDraft('fastStart', (event.currentTarget as HTMLInputElement).checked)} />{text('fastStart')}</label>{/if}
                <label><input type="checkbox" checked={settingsDraft.export.preserveMetadata} onchange={(event) => updateExportDraft('preserveMetadata', (event.currentTarget as HTMLInputElement).checked)} />{text('preserveMetadata')}</label>
                {#if settingsDraft.export.profile === 'lossless'}<label><input type="checkbox" checked={settingsDraft.export.copySubtitles} onchange={(event) => updateExportDraft('copySubtitles', (event.currentTarget as HTMLInputElement).checked)} />{text('copySubtitles')}</label>{/if}
              </div>
            </section>
          {:else}
            <p class="metadata-warning">{text('metadataNote')}</p>
          {/if}

          <section class="settings-section">
            <h3>{text('play')}</h3>
            <div class="check-list"><label><input type="checkbox" checked={settingsDraft.loopPlayback} onchange={(event) => updateDraft('loopPlayback', (event.currentTarget as HTMLInputElement).checked)} />{text('enableLoop')}</label></div>
            <p class="settings-note">{text('loopRemember')}</p>
          </section>
        </div>

        <div class="dialog-actions"><button class="button secondary" type="button" onclick={closeSettingsDialog}>{text('close')}</button><button class="button primary" type="button" onclick={applySettings}>{text('apply')}</button></div>
      </div>
    </div>
  {/if}

  {#if errorMessage}<div class="error-banner" role="alert"><span>{errorMessage}</span><button type="button" aria-label={text('closeError')} onclick={() => (errorMessage = '')}>×</button></div>{/if}
  {#if successMessage}<div class="success-banner" role="status"><span>{successMessage}</span><button type="button" aria-label={text('closeNotice')} onclick={() => (successMessage = '')}>×</button></div>{/if}
</div>
