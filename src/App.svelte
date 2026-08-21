<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { convertFileSrc, invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { open, save } from '@tauri-apps/plugin-dialog';
  import { warn as logWarning } from '@tauri-apps/plugin-log';
  import appIconUrl from '../assets/app-icon.svg';

  import {
    accentColorIds,
    accentPalette,
    applySystemAppearance,
    fallbackTheme,
    normalizeAccent,
    resolveAppearance,
    type AccentColorId,
    type AppearanceMode,
    type AppTheme,
  } from './lib/appearance';
  import { isAppErrorPayload } from './lib/app-error';

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
    encoderPresets,
    exportProfiles,
    pixelFormats,
    suggestOutput,
    type AudioMode,
    type EncoderPreset,
    type ExportCompleteEvent,
    type ExportErrorEvent,
    type ExportProfile,
    type ExportProgressEvent,
    type ExportRequest,
    type FrameRateMode,
    type PixelFormat,
    type VideoCodec,
    type VideoEncoder,
    type VideoEncoderAvailability,
  } from './lib/export';
  import { localizeAppError, translate, type TranslationKey } from './lib/i18n';
  import { formatFrameRate, formatTime, type MediaDescriptor } from './lib/media';
  import {
    cloneSettings,
    defaultSettings,
    loadSettings,
    parseSettings,
    persistSettings,
    resolveLanguage,
    type AppSettings,
    type Language,
    type LanguageMode,
  } from './lib/settings';
  import {
    defaultShortcuts,
    findShortcutConflict,
    formatShortcutChord,
    reservedShortcutChords,
    shortcutChordFromEvent,
    shortcutActionIds,
    shortcutMatchesEvent,
    type ShortcutActionId,
  } from './lib/shortcuts';
  import {
    frameToSeconds,
    fullTrimRange,
    isFullTrim,
    pointerFrameFromTimeline,
    sanitizeTrimRange,
    secondsToFrame,
    totalVideoFrames,
    trimDuration,
    updateTrimHandle,
    type TrimHandle,
    type TrimRange,
  } from './lib/trim';

  const handles: Array<{ value: CropHandle; label: TranslationKey }> = [
    { value: 'north-west', label: 'cropHandleNorthWest' },
    { value: 'north', label: 'cropHandleNorth' },
    { value: 'north-east', label: 'cropHandleNorthEast' },
    { value: 'east', label: 'cropHandleEast' },
    { value: 'south-east', label: 'cropHandleSouthEast' },
    { value: 'south', label: 'cropHandleSouth' },
    { value: 'south-west', label: 'cropHandleSouthWest' },
    { value: 'west', label: 'cropHandleWest' },
  ];

  const settingsCategories = [
    { value: 'export', label: 'settingsExport' },
    { value: 'playback', label: 'settingsPlayback' },
    { value: 'appearance', label: 'settingsAppearance' },
    { value: 'shortcuts', label: 'settingsShortcuts' },
    { value: 'explorer', label: 'settingsExplorer' },
    { value: 'language', label: 'settingsLanguage' },
  ] as const satisfies ReadonlyArray<{ value: string; label: TranslationKey }>;
  type SettingsCategory = (typeof settingsCategories)[number]['value'];

  const shortcutLabels: Record<ShortcutActionId, TranslationKey> = {
    openVideo: 'shortcutOpenVideo',
    openFolder: 'shortcutOpenFolder',
    openSettings: 'shortcutOpenSettings',
    profileCompatible: 'shortcutCompatible',
    profileLossless: 'shortcutLossless',
    profileMetadata: 'shortcutMetadata',
    copySave: 'shortcutCopySave',
    saveInPlace: 'shortcutSaveInPlace',
    previousVideo: 'shortcutPreviousVideo',
    nextVideo: 'shortcutNextVideo',
    playPause: 'shortcutPlayPause',
    seekBackward: 'shortcutSeekBackward',
    seekForward: 'shortcutSeekForward',
    seekBackwardLarge: 'shortcutSeekBackwardLarge',
    seekForwardLarge: 'shortcutSeekForwardLarge',
    toggleFullscreen: 'shortcutToggleFullscreen',
  };

  const accentLabels: Record<AccentColorId, TranslationKey> = {
    blue: 'accentBlue',
    teal: 'accentTeal',
    green: 'accentGreen',
    gold: 'accentGold',
    orange: 'accentOrange',
    red: 'accentRed',
    magenta: 'accentMagenta',
    purple: 'accentPurple',
  };

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

  interface TimeTrimDragState {
    handle: TrimHandle;
    timelineLeft: number;
    lastPointerX: number;
    lastTimestamp: number;
    renderedWidth: number;
  }

  interface TimelineScrubState {
    timelineLeft: number;
    renderedWidth: number;
  }

  interface SelectionDescriptor {
    kind: 'file' | 'directory';
    rootPath: string;
    videoPaths: string[];
  }

  interface DirectoryChangedEvent {
    rootPath: string;
  }

  let media: MediaDescriptor | null = null;
  let crop: CropRect = { x: 0, y: 0, width: 16, height: 16 };
  let aspect: AspectPreset = 'free';
  let videoSrc = '';
  let videoSourceRevision = 0;
  let videoElement: HTMLVideoElement;
  let stageWidth = 0;
  let stageHeight = 0;
  let currentTime = 0;
  let isPlaying = false;
  let resumePlaybackAfterLoad = false;
  let isMuted = false;
  let isLoading = false;
  let isPreparingProxy = false;
  let usingProxy = false;
  let errorMessage = '';
  let successPath = '';
  let successTimer: ReturnType<typeof setTimeout> | null = null;
  let dragState: DragState | null = null;
  let timeTrimDragState: TimeTrimDragState | null = null;
  let timelineScrubState: TimelineScrubState | null = null;
  let selectedTrimHandle: TrimHandle | null = null;
  let seekFrame: number | null = null;
  let unlistenDragDrop: UnlistenFn | undefined;
  let unlistenTheme: UnlistenFn | undefined;
  let unlistenDirectoryChanges: UnlistenFn | undefined;
  let unlistenExportEvents: UnlistenFn[] = [];
  let exportEventsReady = false;
  let exportEventsError = '';
  let destroyed = false;
  let playlist: string[] = [];
  let playlistIndex = 0;
  let directoryPath: string | null = null;
  let directoryRefreshTimer: ReturnType<typeof setTimeout> | null = null;
  let directoryRefreshPending = false;
  let isRefreshingDirectory = false;
  let showSaveMenu = false;
  let exportJobId: string | null = null;
  let exportProgress = 0;
  let exportOutTime = 0;
  let isStartingExport = false;
  let inPlaceExportPath: string | null = null;
  let showSettings = false;
  let showInspector = true;
  let showTransport = true;
  let isVideoFullscreen = false;
  let systemTheme: AppTheme = 'dark';
  let systemAccent = '#0078D4';
  let trim: TrimRange = fullTrimRange(1);
  let timelineStripSrc = '';
  let settingsReady = false;
  let settings: AppSettings = cloneSettings(defaultSettings);
  let settingsDraft: AppSettings = cloneSettings(defaultSettings);
  let persistedSettings: AppSettings = cloneSettings(defaultSettings);
  let settingsSaveQueue: Promise<void> = Promise.resolve();
  let settingsRevision = 0;
  let shortcutRecording: ShortcutActionId | null = null;
  let shortcutError = '';
  let settingsCategory: SettingsCategory = 'export';
  let showAccentPalette = false;
  let appliedWindowThemeMode = '';
  let encoderAvailability: VideoEncoderAvailability = {
    h264: { nvidia: false, intel: false, amd: false },
    h265: { nvidia: false, intel: false, amd: false },
  };
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
  $: duration = media?.durationSeconds ?? 0;
  $: totalFrames = media
    ? totalVideoFrames(media.frameCount)
    : 1;
  $: safeTrim = sanitizeTrimRange(trim, totalFrames);
  $: trimStartSeconds = frameToSeconds(safeTrim.startFrame, totalFrames, duration);
  $: trimEndSeconds = frameToSeconds(safeTrim.endFrame, totalFrames, duration);
  $: selectedDuration = trimDuration(safeTrim, totalFrames, duration);
  $: timeTrimmed = !isFullTrim(safeTrim, totalFrames);
  $: trimSelectionStyle = `left:${(safeTrim.startFrame / totalFrames) * 100}%;right:${100 - (safeTrim.endFrame / totalFrames) * 100}%`;
  $: trimLeftMaskStyle = `width:${(safeTrim.startFrame / totalFrames) * 100}%`;
  $: trimRightMaskStyle = `width:${100 - (safeTrim.endFrame / totalFrames) * 100}%`;
  $: playheadStyle = `left:${duration > 0 ? (Math.min(duration, Math.max(0, currentTime)) / duration) * 100 : 0}%`;
  $: timelineStripStyle = timelineStripSrc ? `background-image:url("${timelineStripSrc}")` : '';
  $: language = resolveLanguage(showSettings ? settingsDraft : settings, systemLanguage);
  $: text = (key: TranslationKey, values: Record<string, string | number> = {}) =>
    translate(language, key, values);
  $: if (typeof document !== 'undefined') document.documentElement.lang = language;
  $: resolvedAppearance = resolveAppearance(settings.appearance, systemTheme, systemAccent);
  $: if (typeof document !== 'undefined') {
    applySystemAppearance(resolvedAppearance.theme, resolvedAppearance.accent);
  }
  $: requestedWindowTheme = settings.appearance.themeMode === 'system' ? null : settings.appearance.theme;
  $: if (settingsReady) synchronizeWindowTheme(requestedWindowTheme);
  $: profileSupported =
    media !== null &&
    (settings.export.profile !== 'metadata' || (media.metadataCropSupported && !timeTrimmed));
  $: canOverwrite =
    media !== null &&
    canSaveInPlace(media.sourcePath, settings.export.profile);
  $: profileLabel = profileName(settings.export.profile);
  $: exportLocked = isStartingExport || exportJobId !== null;

  onMount(() => {
    systemLanguage = navigator.language || 'en-US';
    void initializeSettings();
    void initializeEncoderAvailability();
    void initializeSystemAppearance();
    void initializeDirectoryEvents();
    void initializeStartupSelection();

    void initializeDragDrop();
    void initializeExportEvents();

    window.addEventListener('keydown', handleKeyboard);
    window.addEventListener('pointerdown', handleGlobalPointerDown);
    window.addEventListener('wheel', handleGlobalWheel, { passive: true });
    window.addEventListener('focus', refreshSystemAccent);
  });

  async function initializeSettings() {
    try {
      const loaded = await loadSettings();
      if (destroyed) return;
      settings = loaded;
      settingsDraft = cloneSettings(loaded);
      persistedSettings = cloneSettings(loaded);
    } catch (error) {
      if (!destroyed) errorMessage = clientError('settingsLoadFailed', error);
    } finally {
      if (!destroyed) settingsReady = true;
    }
  }

  async function initializeEncoderAvailability() {
    try {
      const availability = await invoke<VideoEncoderAvailability>('available_video_encoders');
      if (!destroyed && isEncoderAvailability(availability)) encoderAvailability = availability;
    } catch (error) {
      recordWarning('Video encoder availability probe failed', error);
    }
  }

  function isEncoderAvailability(value: unknown): value is VideoEncoderAvailability {
    if (typeof value !== 'object' || value === null) return false;
    const availability = value as Partial<VideoEncoderAvailability>;
    return [availability.h264, availability.h265].every(
      (codecs) =>
        typeof codecs?.nvidia === 'boolean' &&
        typeof codecs.intel === 'boolean' &&
        typeof codecs.amd === 'boolean',
    );
  }

  async function initializeStartupSelection() {
    try {
      const path = await invoke<string | null>('startup_selection');
      if (!destroyed && path) await loadSelection(path);
    } catch (error) {
      if (!destroyed) errorMessage = backendOrClientError('selectionFailed', error);
    }
  }

  onDestroy(() => {
    destroyed = true;
    unlistenDragDrop?.();
    unlistenTheme?.();
    unlistenDirectoryChanges?.();
    for (const unlisten of unlistenExportEvents) unlisten();
    window.removeEventListener('keydown', handleKeyboard);
    window.removeEventListener('pointerdown', handleGlobalPointerDown);
    window.removeEventListener('wheel', handleGlobalWheel);
    window.removeEventListener('focus', refreshSystemAccent);
    endCropDrag();
    endTimeTrimDrag();
    endTimelineScrub();
    if (seekFrame !== null) cancelAnimationFrame(seekFrame);
    if (successTimer !== null) clearTimeout(successTimer);
    if (directoryRefreshTimer !== null) clearTimeout(directoryRefreshTimer);
  });

  async function initializeDirectoryEvents() {
    try {
      const unlisten = await listen<DirectoryChangedEvent>('directory-changed', (event) => {
        if (!directoryPath || !sameWindowsPath(event.payload.rootPath, directoryPath)) return;
        scheduleDirectoryRefresh();
      });
      if (destroyed) unlisten();
      else unlistenDirectoryChanges = unlisten;
    } catch (error) {
      if (!destroyed) errorMessage = clientError('directoryWatchUnavailable', error);
    }
  }

  async function initializeDragDrop() {
    try {
      const unlisten = await getCurrentWebview().onDragDropEvent((event) => {
        if (event.payload.type !== 'drop' || event.payload.paths.length === 0) return;
        void loadDroppedPaths(event.payload.paths);
      });
      if (destroyed) {
        unlisten();
      } else {
        unlistenDragDrop = unlisten;
      }
    } catch (error) {
      if (!destroyed) errorMessage = clientError('dragDropUnavailable', error);
    }
  }

  async function initializeExportEvents() {
    const acquired: UnlistenFn[] = [];
    try {
      acquired.push(
        await listen<ExportProgressEvent>('export-progress', (event) => {
          if (event.payload.jobId !== exportJobId) return;
          exportProgress = clampProgress(event.payload.fraction);
          exportOutTime = event.payload.outTimeSeconds;
        }),
      );
      acquired.push(
        await listen<ExportCompleteEvent>('export-complete', (event) => {
          if (event.payload.jobId !== exportJobId) return;
          void handleExportComplete(event.payload);
        }),
      );
      acquired.push(
        await listen<ExportErrorEvent>('export-error', (event) => {
          if (event.payload.jobId !== exportJobId) return;
          const shouldRestore = inPlaceExportPath !== null;
          exportJobId = null;
          exportProgress = 0;
          inPlaceExportPath = null;
          if (shouldRestore) restoreSourcePreview();
          if (!event.payload.cancelled) {
            errorMessage = `${text('exportFailed')}${readableError(event.payload.error)}`;
          }
          if (directoryRefreshPending) scheduleDirectoryRefresh();
        }),
      );
      if (destroyed) {
        for (const unlisten of acquired) unlisten();
        return;
      }
      unlistenExportEvents = acquired;
      exportEventsReady = true;
    } catch (error) {
      for (const unlisten of acquired) unlisten();
      if (!destroyed) {
        exportEventsError = clientError('exportEventsUnavailable', error);
        errorMessage = exportEventsError;
      }
    }
  }

  async function initializeSystemAppearance() {
    const appWindow = getCurrentWindow();
    try {
      const current = await appWindow.theme();
      systemTheme = current ?? fallbackTheme(prefersDarkMode());
    } catch (error) {
      recordWarning('system theme query failed', error);
      systemTheme = fallbackTheme(prefersDarkMode());
    }
    await refreshSystemAccent();
    try {
      const unlisten = await appWindow.onThemeChanged((event) => {
        systemTheme = event.payload;
        void refreshSystemAccent();
      });
      if (destroyed) unlisten();
      else unlistenTheme = unlisten;
    } catch (error) {
      recordWarning('system theme listener registration failed', error);
      // Browser-only development keeps the media-query fallback.
    }
  }

  function synchronizeWindowTheme(theme: AppTheme | null) {
    const key = theme ?? 'system';
    if (key === appliedWindowThemeMode) return;
    appliedWindowThemeMode = key;
    void getCurrentWindow()
      .setTheme(theme)
      .catch((error) => recordWarning('window theme update failed', error));
  }

  function prefersDarkMode(): boolean {
    return typeof window.matchMedia === 'function' && window.matchMedia('(prefers-color-scheme: dark)').matches;
  }

  async function refreshSystemAccent() {
    try {
      systemAccent = normalizeAccent(await invoke<string>('system_accent_color'));
    } catch (error) {
      recordWarning('system accent query failed', error);
    }
  }

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
    try {
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
    } catch (error) {
      errorMessage = clientError('openDialogFailed', error);
    }
  }

  async function chooseDirectory() {
    try {
      const selected = await open({ multiple: false, directory: true });
      if (typeof selected === 'string') await loadSelection(selected);
    } catch (error) {
      errorMessage = clientError('openDialogFailed', error);
    }
  }

  async function loadSelection(path: string) {
    if (isLoading || isPreparingProxy || exportJobId) return;
    isLoading = true;
    try {
      const selection = await invoke<SelectionDescriptor>('inspect_selection', { path });
      isLoading = false;
      await replacePlaylist(
        selection.videoPaths,
        selection.kind === 'directory' ? selection.rootPath : null,
      );
    } catch (error) {
      errorMessage = backendOrClientError('selectionFailed', error);
    } finally {
      isLoading = false;
    }
  }

  async function loadDroppedPaths(paths: readonly string[]) {
    if (paths.length === 0 || isLoading || isPreparingProxy || exportJobId) return;
    isLoading = true;
    try {
      const selections = await Promise.all(
        paths.map((path) => invoke<SelectionDescriptor>('inspect_selection', { path })),
      );
      const seen = new Set<string>();
      const videoPaths = selections
        .flatMap((selection) => selection.videoPaths)
        .filter((path) => {
          const key = path.toLowerCase();
          if (seen.has(key)) return false;
          seen.add(key);
          return true;
        });
      const selectedDirectory =
        selections.length === 1 && selections[0].kind === 'directory'
          ? selections[0].rootPath
          : null;
      isLoading = false;
      await replacePlaylist(videoPaths, selectedDirectory);
    } catch (error) {
      errorMessage = backendOrClientError('selectionFailed', error);
    } finally {
      isLoading = false;
    }
  }

  async function replacePlaylist(videoPaths: string[], selectedDirectory: string | null) {
    if (videoPaths.length === 0) return;
    const previous = { playlist, playlistIndex, directoryPath };
    playlist = [...videoPaths];
    playlistIndex = 0;
    directoryPath = selectedDirectory;
    if (!(await loadVideo(playlist[0], true))) {
      playlist = previous.playlist;
      playlistIndex = previous.playlistIndex;
      directoryPath = previous.directoryPath;
      return;
    }
    await updateDirectoryWatch(selectedDirectory);
  }

  async function updateDirectoryWatch(path: string | null) {
    try {
      await invoke('watch_directory', { path });
    } catch (error) {
      errorMessage = backendOrClientError('directoryWatchUnavailable', error);
    }
  }

  function scheduleDirectoryRefresh() {
    if (!directoryPath || destroyed) return;
    if (directoryRefreshTimer !== null) clearTimeout(directoryRefreshTimer);
    directoryRefreshTimer = setTimeout(() => {
      directoryRefreshTimer = null;
      void refreshDirectory();
    }, 150);
  }

  async function refreshDirectory() {
    const expectedDirectory = directoryPath;
    if (!expectedDirectory || destroyed) return;
    if (
      isLoading ||
      isPreparingProxy ||
      isStartingExport ||
      exportJobId !== null ||
      isRefreshingDirectory
    ) {
      directoryRefreshPending = true;
      return;
    }

    isRefreshingDirectory = true;
    directoryRefreshPending = false;
    try {
      const selection = await invoke<SelectionDescriptor>('inspect_selection', {
        path: expectedDirectory,
      });
      if (
        destroyed ||
        !directoryPath ||
        !sameWindowsPath(expectedDirectory, directoryPath) ||
        selection.kind !== 'directory'
      ) {
        return;
      }

      const currentPath = media?.sourcePath;
      const currentIndex = currentPath
        ? selection.videoPaths.findIndex((path) => sameWindowsPath(path, currentPath))
        : -1;
      if (currentIndex >= 0) {
        playlist = [...selection.videoPaths];
        playlistIndex = currentIndex;
        return;
      }

      const previous = { playlist, playlistIndex };
      playlist = [...selection.videoPaths];
      playlistIndex = Math.min(previous.playlistIndex, playlist.length - 1);
      const resumePlayback = isPlaying || Boolean(videoElement && !videoElement.paused);
      if (!(await loadVideo(playlist[playlistIndex], true, resumePlayback))) {
        playlist = previous.playlist;
        playlistIndex = previous.playlistIndex;
      }
    } catch (error) {
      if (directoryPath && sameWindowsPath(expectedDirectory, directoryPath)) {
        errorMessage = backendOrClientError('selectionFailed', error);
      }
    } finally {
      isRefreshingDirectory = false;
      if (directoryRefreshPending && !isStartingExport && exportJobId === null) {
        scheduleDirectoryRefresh();
      }
    }
  }

  function sameWindowsPath(left: string, right: string): boolean {
    return left.toLocaleLowerCase('en-US') === right.toLocaleLowerCase('en-US');
  }

  async function loadVideo(
    path: string,
    keepPlaylist = false,
    resumePlayback = false,
  ): Promise<boolean> {
    if (isLoading || isPreparingProxy || exportJobId) return false;
    isLoading = true;
    errorMessage = exportEventsError;
    dismissSuccess();
    usingProxy = false;
    timelineStripSrc = '';
    currentTime = 0;
    isPlaying = false;
    resumePlaybackAfterLoad = resumePlayback;
    selectedTrimHandle = null;
    showSaveMenu = false;
    videoElement?.pause();

    try {
      const descriptor = await invoke<MediaDescriptor>('probe_video', { path });
      media = descriptor;
      trim = fullTrimRange(
        totalVideoFrames(descriptor.frameCount),
      );
      crop = fullFrame({ width: descriptor.displayWidth, height: descriptor.displayHeight });
      aspect = 'free';
      videoSrc = freshMediaSource(descriptor.sourcePath);
      if (!keepPlaylist) {
        playlist = [descriptor.sourcePath];
        playlistIndex = 0;
        directoryPath = null;
      } else {
        const found = playlist.findIndex(
          (item) => item.toLowerCase() === descriptor.sourcePath.toLowerCase(),
        );
        if (found >= 0) playlistIndex = found;
      }
      await tick();
      videoElement?.load();
      void loadTimelineStrip(descriptor);
      return true;
    } catch (error) {
      resumePlaybackAfterLoad = false;
      errorMessage = backendOrClientError('selectionFailed', error);
      return false;
    } finally {
      isLoading = false;
    }
  }

  async function loadTimelineStrip(descriptor: MediaDescriptor) {
    try {
      const stripPath = await invoke<string>('create_timeline_strip', {
        path: descriptor.sourcePath,
        durationSeconds: descriptor.durationSeconds,
      });
      if (media?.sourcePath === descriptor.sourcePath) {
        timelineStripSrc = convertFileSrc(stripPath);
      }
    } catch (error) {
      recordWarning('timeline strip generation failed', error);
      // The timeline remains fully usable with its lightweight fallback pattern.
    }
  }

  async function navigatePlaylist(offset: number) {
    if (playlist.length < 2 || isLoading || exportJobId) return;
    const next = Math.min(playlist.length - 1, Math.max(0, playlistIndex + offset));
    if (next === playlistIndex) return;
    const resumePlayback =
      directoryPath !== null && (isPlaying || (videoElement && !videoElement.paused));
    await loadVideo(playlist[next], true, Boolean(resumePlayback));
  }

  async function selectPlaylistVideo(event: Event) {
    const next = Number((event.currentTarget as HTMLSelectElement).value);
    if (!Number.isInteger(next) || !playlist[next] || next === playlistIndex) return;
    const resumePlayback =
      directoryPath !== null && (isPlaying || (videoElement && !videoElement.paused));
    await loadVideo(playlist[next], true, Boolean(resumePlayback));
  }

  function handleVideoCanPlay() {
    if (!resumePlaybackAfterLoad) return;
    resumePlaybackAfterLoad = false;
    void playVideo();
  }

  function freshMediaSource(path: string): string {
    videoSourceRevision += 1;
    const source = convertFileSrc(path);
    const separator = source.includes('?') ? '&' : '?';
    return `${source}${separator}vidmetryRevision=${videoSourceRevision}`;
  }

  async function handleVideoError() {
    if (!media || usingProxy || isPreparingProxy || isLoading || !videoSrc) return;
    isPreparingProxy = true;
    errorMessage = '';
    try {
      const proxyPath = await invoke<string>('create_preview', { path: media.sourcePath });
      usingProxy = true;
      videoSrc = freshMediaSource(proxyPath);
      await tick();
      videoElement?.load();
    } catch (error) {
      errorMessage = `${text('previewFailed')}${readableError(error)}`;
    } finally {
      isPreparingProxy = false;
    }
  }

  async function togglePlayback() {
    if (!videoElement || !media) return;
    if (videoElement.paused) {
      if (currentTime < trimStartSeconds || currentTime >= trimEndSeconds) {
        seekToFrame(safeTrim.startFrame);
      }
      await playVideo();
    } else {
      videoElement.pause();
    }
  }

  async function playVideo() {
    if (!videoElement) return;
    try {
      await videoElement.play();
    } catch (error) {
      errorMessage = clientError('playbackFailed', error);
    }
  }

  function scrubTo(event: Event) {
    const requested = Number((event.currentTarget as HTMLInputElement).value);
    const frame = Math.min(
      safeTrim.endFrame,
      Math.max(safeTrim.startFrame, secondsToFrame(requested, totalFrames, duration)),
    );
    const next = frameToSeconds(frame, totalFrames, duration);
    seekToTime(next);
  }

  function handlePlaybackScrubberKey(event: KeyboardEvent) {
    if (!shortcutMatchesEvent(settings.shortcuts.playPause, event)) return;
    event.preventDefault();
    event.stopPropagation();
    void togglePlayback();
  }

  function seekToTimelinePointer(clientX: number, state: TimelineScrubState) {
    const requestedFrame = pointerFrameFromTimeline(
      clientX,
      state.timelineLeft,
      state.renderedWidth,
      totalFrames,
      0,
      0,
    );
    seekToFrame(Math.min(safeTrim.endFrame, Math.max(safeTrim.startFrame, requestedFrame)));
  }

  function beginTimelineScrub(event: PointerEvent) {
    if (!media || event.button !== 0 || exportLocked) return;
    const input = event.currentTarget as HTMLInputElement;
    const timeline = input.closest('.trim-timeline');
    if (!(timeline instanceof HTMLElement)) return;
    event.preventDefault();
    event.stopPropagation();
    input.focus({ preventScroll: true });
    const timelineRect = timeline.getBoundingClientRect();
    timelineScrubState = {
      timelineLeft: timelineRect.left,
      renderedWidth: timelineRect.width,
    };
    seekToTimelinePointer(event.clientX, timelineScrubState);
    window.addEventListener('pointermove', continueTimelineScrub);
    window.addEventListener('pointerup', endTimelineScrub, { once: true });
  }

  function continueTimelineScrub(event: PointerEvent) {
    if (!timelineScrubState) return;
    seekToTimelinePointer(event.clientX, timelineScrubState);
  }

  function endTimelineScrub() {
    timelineScrubState = null;
    window.removeEventListener('pointermove', continueTimelineScrub);
    window.removeEventListener('pointerup', endTimelineScrub);
  }

  function seekToTime(next: number) {
    currentTime = Math.min(trimEndSeconds, Math.max(trimStartSeconds, next));
    if (seekFrame !== null) cancelAnimationFrame(seekFrame);
    seekFrame = requestAnimationFrame(() => {
      if (videoElement && Number.isFinite(currentTime)) videoElement.currentTime = currentTime;
      seekFrame = null;
    });
  }

  function seekToFrame(frame: number) {
    seekToTime(frameToSeconds(frame, totalFrames, duration));
  }

  function handleTimeUpdate() {
    if (!videoElement) return;
    const next = videoElement.currentTime;
    const frameSeconds = duration / Math.max(1, totalFrames);
    if (next + frameSeconds / 2 < trimStartSeconds) {
      seekToFrame(safeTrim.startFrame);
      return;
    }
    if (next >= trimEndSeconds - frameSeconds / 3) {
      if (settings.loopPlayback) {
        const shouldResume = !videoElement.paused;
        seekToFrame(safeTrim.startFrame);
        if (shouldResume) void playVideo();
      } else {
        videoElement.pause();
        currentTime = trimEndSeconds;
      }
      return;
    }
    currentTime = next;
  }

  function handleVideoEnded() {
    isPlaying = false;
    if (settings.loopPlayback) {
      seekToFrame(safeTrim.startFrame);
      void playVideo();
    } else {
      currentTime = trimEndSeconds;
    }
  }

  function toggleMute() {
    isMuted = !isMuted;
    if (videoElement) videoElement.muted = isMuted;
  }

  function toggleLoop() {
    updateSettings({ ...settings, loopPlayback: !settings.loopPlayback });
  }

  function handleKeyboard(event: KeyboardEvent) {
    if (successPath) dismissSuccess();
    const target = event.target instanceof Element ? event.target : null;
    const focusedControl =
      target?.matches('input, select, textarea, button, [contenteditable="true"]') ?? false;
    if (shortcutRecording) {
      captureShortcut(event);
      return;
    }
    if (event.code === 'Escape') {
      if (isVideoFullscreen) {
        event.preventDefault();
        void setVideoFullscreen(false);
        return;
      }
      showSaveMenu = false;
      if (!exportJobId) showSettings = false;
      return;
    }
    if (
      settingsReady &&
      shortcutAllowedFromTarget(settings.shortcuts.openSettings, focusedControl) &&
      shortcutMatchesEvent(settings.shortcuts.openSettings, event)
    ) {
      event.preventDefault();
      if (!showSettings) openSettingsDialog();
      return;
    }
    if (showSettings) return;
    if (
      shortcutAllowedFromTarget(settings.shortcuts.openVideo, focusedControl) &&
      shortcutMatchesEvent(settings.shortcuts.openVideo, event)
    ) {
      event.preventDefault();
      if (!isLoading && !isPreparingProxy && exportJobId === null) void chooseVideo();
      return;
    }
    if (
      shortcutAllowedFromTarget(settings.shortcuts.openFolder, focusedControl) &&
      shortcutMatchesEvent(settings.shortcuts.openFolder, event)
    ) {
      event.preventDefault();
      if (!isLoading && !isPreparingProxy && exportJobId === null) void chooseDirectory();
      return;
    }
    if (
      media &&
      shortcutAllowedFromTarget(settings.shortcuts.toggleFullscreen, focusedControl) &&
      shortcutMatchesEvent(settings.shortcuts.toggleFullscreen, event)
    ) {
      event.preventDefault();
      void setVideoFullscreen(!isVideoFullscreen);
      return;
    }
    if (!media) return;
    const profileShortcuts: Array<[ShortcutActionId, ExportProfile]> = [
      ['profileCompatible', 'compatible'],
      ['profileLossless', 'lossless'],
      ['profileMetadata', 'metadata'],
    ];
    const selection = profileShortcuts.find(
      ([action]) =>
        shortcutAllowedFromTarget(settings.shortcuts[action], focusedControl) &&
        shortcutMatchesEvent(settings.shortcuts[action], event),
    );
    if (selection) {
      event.preventDefault();
      setProfile(selection[1]);
      return;
    }
    if (
      shortcutAllowedFromTarget(settings.shortcuts.saveInPlace, focusedControl) &&
      shortcutMatchesEvent(settings.shortcuts.saveInPlace, event)
    ) {
      event.preventDefault();
      if (canOverwrite) void saveInPlace();
      return;
    }
    if (
      shortcutAllowedFromTarget(settings.shortcuts.copySave, focusedControl) &&
      shortcutMatchesEvent(settings.shortcuts.copySave, event)
    ) {
      event.preventDefault();
      void saveCopy();
      return;
    }
    const playlistShortcuts: Array<[ShortcutActionId, number]> = [
      ['previousVideo', -1],
      ['nextVideo', 1],
    ];
    const playlistNavigation = playlistShortcuts.find(
      ([action]) =>
        shortcutAllowedFromTarget(settings.shortcuts[action], focusedControl) &&
        shortcutMatchesEvent(settings.shortcuts[action], event),
    );
    if (playlistNavigation) {
      event.preventDefault();
      void navigatePlaylist(playlistNavigation[1]);
      return;
    }
    if (
      shortcutAllowedFromTarget(settings.shortcuts.playPause, focusedControl) &&
      shortcutMatchesEvent(settings.shortcuts.playPause, event)
    ) {
      event.preventDefault();
      void togglePlayback();
      return;
    }
    const seekShortcuts: Array<[ShortcutActionId, number, number]> = [
      ['seekBackward', -1, 1],
      ['seekForward', 1, 1],
      ['seekBackwardLarge', -1, 10],
      ['seekForwardLarge', 1, 10],
    ];
    const seek = seekShortcuts.find(
      ([action]) =>
        shortcutAllowedFromTarget(settings.shortcuts[action], focusedControl) &&
        shortcutMatchesEvent(settings.shortcuts[action], event),
    );
    if (seek) {
      event.preventDefault();
      const [, direction, amount] = seek;
      const currentFrame = secondsToFrame(currentTime, totalFrames, duration);
      seekToFrame(
        Math.min(safeTrim.endFrame, Math.max(safeTrim.startFrame, currentFrame + direction * amount)),
      );
    }
  }

  function shortcutAllowedFromTarget(chord: string, focusedControl: boolean): boolean {
    return !focusedControl || chord.includes('+');
  }

  async function setVideoFullscreen(fullscreen: boolean) {
    try {
      await getCurrentWindow().setFullscreen(fullscreen);
      isVideoFullscreen = fullscreen;
      showSaveMenu = false;
    } catch (error) {
      errorMessage = clientError('fullscreenFailed', error);
    }
  }

  function handleGlobalPointerDown(event: PointerEvent) {
    if (!(event.target instanceof Element)) return;
    if (!event.target.closest('.save-options')) showSaveMenu = false;
    if (!event.target.closest('.trim-handle')) selectedTrimHandle = null;
    if (successPath && !event.target.closest('.success-banner')) dismissSuccess();
  }

  function handleGlobalWheel() {
    if (successPath) dismissSuccess();
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

  function beginTimeTrimDrag(event: PointerEvent, handle: TrimHandle) {
    if (!media || event.button !== 0 || exportLocked) return;
    const handleElement = event.currentTarget as HTMLButtonElement;
    const timeline = handleElement.closest('.trim-timeline');
    if (!(timeline instanceof HTMLElement)) return;
    event.preventDefault();
    event.stopPropagation();
    selectedTrimHandle = handle;
    handleElement.focus({ preventScroll: true });
    videoElement?.pause();
    const timelineRect = timeline.getBoundingClientRect();
    timeTrimDragState = {
      handle,
      timelineLeft: timelineRect.left,
      lastPointerX: event.clientX,
      lastTimestamp: event.timeStamp,
      renderedWidth: timelineRect.width,
    };
    seekToFrame(handle === 'start' ? safeTrim.startFrame : Math.max(0, safeTrim.endFrame - 1));
    window.addEventListener('pointermove', continueTimeTrimDrag);
    window.addEventListener('pointerup', endTimeTrimDrag, { once: true });
  }

  function continueTimeTrimDrag(event: PointerEvent) {
    if (!timeTrimDragState) return;
    const coalesced = event.getCoalescedEvents?.() ?? [];
    const previous = coalesced.at(-1);
    const previousX = previous?.clientX ?? timeTrimDragState.lastPointerX;
    const previousTimestamp = previous?.timeStamp ?? timeTrimDragState.lastTimestamp;
    const elapsed = Math.max(1, event.timeStamp - previousTimestamp);
    const velocity = Math.abs(event.clientX - previousX) / elapsed;
    const requestedFrame = pointerFrameFromTimeline(
      event.clientX,
      timeTrimDragState.timelineLeft,
      timeTrimDragState.renderedWidth,
      totalFrames,
      0,
      velocity,
    );
    timeTrimDragState.lastPointerX = event.clientX;
    timeTrimDragState.lastTimestamp = event.timeStamp;
    trim = updateTrimHandle(
      safeTrim,
      timeTrimDragState.handle,
      requestedFrame,
      totalFrames,
    );
    const previewFrame =
      timeTrimDragState.handle === 'start' ? trim.startFrame : Math.max(trim.startFrame, trim.endFrame - 1);
    seekToFrame(previewFrame);
  }

  function endTimeTrimDrag() {
    timeTrimDragState = null;
    window.removeEventListener('pointermove', continueTimeTrimDrag);
    window.removeEventListener('pointerup', endTimeTrimDrag);
  }

  function handleTrimKey(event: KeyboardEvent, handle: TrimHandle) {
    if (exportLocked) return;
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    event.stopPropagation();
    const current = handle === 'start' ? safeTrim.startFrame : safeTrim.endFrame;
    const amount = event.shiftKey ? 10 : 1;
    let requested = current;
    if (event.key === 'ArrowLeft') requested -= amount;
    if (event.key === 'ArrowRight') requested += amount;
    if (event.key === 'Home') requested = handle === 'start' ? 0 : safeTrim.startFrame + 1;
    if (event.key === 'End') requested = handle === 'start' ? safeTrim.endFrame - 1 : totalFrames;
    trim = updateTrimHandle(safeTrim, handle, requested, totalFrames);
    seekToFrame(handle === 'start' ? trim.startFrame : Math.max(trim.startFrame, trim.endFrame - 1));
  }

  function selectTrimHandle(handle: TrimHandle) {
    selectedTrimHandle = handle;
  }

  function handleTrimBlur(event: FocusEvent) {
    const next = event.relatedTarget;
    if (!(next instanceof Element) || !next.closest('.trim-handle')) {
      selectedTrimHandle = null;
    }
  }

  function resetTimeTrim() {
    trim = fullTrimRange(totalFrames);
    seekToFrame(0);
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
    if (!media || !profileSupported || !exportEventsReady || exportJobId || isStartingExport) return;
    const suggestion = suggestOutput(media.sourcePath, settings.export.profile);
    try {
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
    } catch (error) {
      errorMessage = clientError('saveDialogFailed', error);
    }
  }

  async function saveInPlace() {
    showSaveMenu = false;
    if (!media || !canOverwrite || !exportEventsReady || exportJobId || isStartingExport) return;
    if (!window.confirm(text('overwriteConfirm', { name: media.fileName }))) return;
    await startExport(media.sourcePath, true);
  }

  async function startExport(outputPath: string, inPlace: boolean) {
    if (!media) return;
    isStartingExport = true;
    errorMessage = '';
    dismissSuccess();
    endCropDrag();
    endTimeTrimDrag();
    endTimelineScrub();
    const request: ExportRequest = {
      sourcePath: media.sourcePath,
      outputPath,
      crop: { ...crop },
      trim: { ...safeTrim },
      settings: { ...settings.export },
      overwrite: true,
      inPlace,
    };
    try {
      videoElement?.pause();
      if (inPlace) {
        inPlaceExportPath = request.sourcePath;
        videoSrc = '';
        await tick();
        videoElement?.load();
      }
      exportProgress = 0;
      exportOutTime = 0;
      exportJobId = await invoke<string>('start_export', { request });
    } catch (error) {
      if (inPlace) restoreSourcePreview();
      inPlaceExportPath = null;
      errorMessage = `${text('exportStartError')}${readableError(error)}`;
    } finally {
      isStartingExport = false;
      if (!exportJobId && directoryRefreshPending) scheduleDirectoryRefresh();
    }
  }

  async function handleExportComplete(event: ExportCompleteEvent) {
    exportProgress = 1;
    exportJobId = null;
    const replacedSource = inPlaceExportPath !== null;
    inPlaceExportPath = null;
    if (replacedSource) await loadVideo(event.outputPath, true);
    if (directoryPath) await refreshDirectory();
    showSuccess(event.outputPath);
  }

  function showSuccess(path: string) {
    dismissSuccess();
    successPath = path;
    successTimer = setTimeout(() => {
      successPath = '';
      successTimer = null;
    }, 3000);
  }

  function dismissSuccess() {
    if (successTimer !== null) {
      clearTimeout(successTimer);
      successTimer = null;
    }
    successPath = '';
  }

  async function revealSavedFile() {
    if (!successPath) return;
    const path = successPath;
    try {
      await invoke('reveal_in_explorer', { path });
      dismissSuccess();
    } catch (error) {
      errorMessage = `${text('revealFailed')}${readableError(error)}`;
    }
  }

  async function cancelExport() {
    if (!exportJobId) return;
    try {
      await invoke('cancel_export', { jobId: exportJobId });
    } catch (error) {
      errorMessage = backendOrClientError('cancelExportFailed', error);
    }
  }

  function restoreSourcePreview() {
    if (!media) return;
    usingProxy = false;
    videoSrc = freshMediaSource(media.sourcePath);
    void tick().then(() => videoElement?.load());
  }

  function openSettingsDialog() {
    settingsDraft = cloneSettings(settings);
    shortcutError = '';
    showSaveMenu = false;
    showSettings = true;
  }

  function closeSettingsDialog() {
    shortcutRecording = null;
    shortcutError = '';
    showAccentPalette = false;
    showSettings = false;
  }

  function updateSettings(value: AppSettings) {
    let next: AppSettings;
    try {
      next = parseSettings(value);
    } catch (error) {
      errorMessage = clientError('settingsSaveFailed', error);
      return;
    }

    settings = next;
    settingsDraft = cloneSettings(next);
    const revision = ++settingsRevision;
    const snapshot = cloneSettings(next);
    settingsSaveQueue = settingsSaveQueue
      .catch(() => undefined)
      .then(async () => {
        const explorerIntegrationChanged =
          snapshot.explorerIntegration !== persistedSettings.explorerIntegration;
        let explorerIntegrationApplied = false;
        try {
          if (explorerIntegrationChanged) {
            await invoke('set_explorer_integration', { enabled: snapshot.explorerIntegration });
            explorerIntegrationApplied = true;
          }
          await persistSettings(snapshot);
          persistedSettings = cloneSettings(snapshot);
        } catch (error) {
          if (explorerIntegrationApplied) {
            try {
              await invoke('set_explorer_integration', {
                enabled: persistedSettings.explorerIntegration,
              });
            } catch (rollbackError) {
              recordWarning('Explorer integration rollback failed', rollbackError);
            }
          }
          throw error;
        }
      })
      .catch((error) => {
        if (revision === settingsRevision) {
          settings = cloneSettings(persistedSettings);
          settingsDraft = cloneSettings(persistedSettings);
          errorMessage = backendOrClientError('settingsSaveFailed', error);
        }
      });
  }

  function updateDraft<K extends keyof AppSettings>(key: K, value: AppSettings[K]) {
    updateSettings({ ...settingsDraft, [key]: value });
  }

  function updateExportDraft<K extends keyof AppSettings['export']>(
    key: K,
    value: AppSettings['export'][K],
  ) {
    updateSettings({
      ...settingsDraft,
      export: { ...settingsDraft.export, [key]: value },
    });
  }

  function updateAppearanceDraft<K extends keyof AppSettings['appearance']>(
    key: K,
    value: AppSettings['appearance'][K],
  ) {
    updateSettings({
      ...settingsDraft,
      appearance: { ...settingsDraft.appearance, [key]: value },
    });
  }

  function selectSettingsCategory(category: SettingsCategory) {
    settingsCategory = category;
    shortcutRecording = null;
    shortcutError = '';
    showAccentPalette = false;
  }

  function handleSettingsNavigation(event: KeyboardEvent, category: SettingsCategory) {
    if (!['ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.code)) return;
    event.preventDefault();
    const currentIndex = settingsCategories.findIndex((item) => item.value === category);
    const nextIndex =
      event.code === 'Home'
        ? 0
        : event.code === 'End'
          ? settingsCategories.length - 1
          : (currentIndex + (event.code === 'ArrowDown' ? 1 : -1) + settingsCategories.length) %
            settingsCategories.length;
    const next = settingsCategories[nextIndex];
    if (!next) return;
    selectSettingsCategory(next.value);
    void tick().then(() => {
      document.querySelector<HTMLButtonElement>(`[data-settings-category="${next.value}"]`)?.focus();
    });
  }

  function startShortcutRecording(action: ShortcutActionId) {
    shortcutRecording = action;
    shortcutError = '';
  }

  function captureShortcut(event: KeyboardEvent) {
    if (!shortcutRecording) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.code === 'Escape') {
      shortcutRecording = null;
      shortcutError = '';
      return;
    }
    const chord = shortcutChordFromEvent(event);
    if (!chord) {
      if (!['ControlLeft', 'ControlRight', 'AltLeft', 'AltRight', 'ShiftLeft', 'ShiftRight'].includes(event.code)) {
        shortcutError = text('shortcutInvalid');
      }
      return;
    }
    if (reservedShortcutChords.has(chord)) {
      shortcutError = text('shortcutReserved');
      return;
    }
    const conflict = findShortcutConflict(shortcutRecording, chord, settingsDraft.shortcuts);
    if (conflict) {
      shortcutError = text('shortcutConflict', { action: text(shortcutLabels[conflict]) });
      return;
    }
    updateSettings({
      ...settingsDraft,
      shortcuts: { ...settingsDraft.shortcuts, [shortcutRecording]: chord },
    });
    shortcutRecording = null;
    shortcutError = '';
  }

  function resetShortcuts() {
    shortcutRecording = null;
    shortcutError = '';
    updateSettings({ ...settingsDraft, shortcuts: { ...defaultShortcuts } });
  }

  function shortcutTitle(label: string, action: ShortcutActionId): string {
    return `${label} (${formatShortcutChord(settings.shortcuts[action])})`;
  }

  function profileShortcutSummary(): string {
    return [
      `${text('compatible')}: ${formatShortcutChord(settings.shortcuts.profileCompatible)}`,
      `${text('lossless')}: ${formatShortcutChord(settings.shortcuts.profileLossless)}`,
      `${text('metadata')}: ${formatShortcutChord(settings.shortcuts.profileMetadata)}`,
    ].join(' · ');
  }

  function encoderAvailable(codec: VideoCodec, encoder: VideoEncoder): boolean {
    if (encoder === 'automatic' || encoder === 'software') return true;
    return encoderAvailability[codec][encoder];
  }

  function setProfile(profile: ExportProfile) {
    const next = { ...settingsDraft.export, profile };
    if (profile === 'compatible') {
      if (next.audioMode === 'flac' || next.audioMode === 'pcm') next.audioMode = 'auto';
    } else if (profile === 'lossless') {
      next.pixelFormat = 'source';
    }
    updateSettings({ ...settingsDraft, export: next });
  }

  function profileName(profile: ExportProfile): string {
    return text(profile);
  }

  function fileName(path: string): string {
    const separator = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
    return separator >= 0 ? path.slice(separator + 1) : path;
  }

  function readableError(error: unknown): string {
    if (isAppErrorPayload(error)) return localizeAppError(language, error);
    if (typeof error === 'string') return error;
    if (error instanceof Error) return error.message;
    return text('unknownError');
  }

  function clientError(key: TranslationKey, error: unknown): string {
    const message = text(key);
    const detail = typeof error === 'string' ? error : error instanceof Error ? error.message : '';
    return detail ? text('errorWithDetail', { message, detail }) : message;
  }

  function backendOrClientError(key: TranslationKey, error: unknown): string {
    return isAppErrorPayload(error) ? readableError(error) : clientError(key, error);
  }

  function recordWarning(context: string, error: unknown) {
    const detail = error instanceof Error ? error.message : serializeDiagnostic(error);
    void logWarning(`${context}: ${detail}`).catch(() => undefined);
  }

  function serializeDiagnostic(value: unknown): string {
    try {
      if (typeof value === 'string') return value;
      return JSON.stringify(value) ?? String(value);
    } catch {
      return String(value);
    }
  }
</script>

<svelte:head>
  <title>{media ? `${media.fileName} — Vidmetry` : 'Vidmetry'}</title>
</svelte:head>

<div
  class="app-shell"
  class:has-media={media !== null}
  class:transport-collapsed={media !== null && !showTransport}
  class:video-fullscreen={isVideoFullscreen}
>
  <header class="app-header" class:launcher-header={media === null}>
    <div class="brand" aria-label="Vidmetry">
      <img class="brand-icon" src={appIconUrl} alt="" aria-hidden="true" />
      <span>Vidmetry</span>
    </div>

    {#if media}
      <div class="source-summary" title={media.sourcePath}>
        <strong>{media.fileName}</strong>
        <span>{media.displayWidth} × {media.displayHeight} · {formatFrameRate(media.frameRate)} · {media.videoCodec.toUpperCase()}</span>
        <small title={profileShortcutSummary()}>{text('activeProfile', { profile: profileLabel })}</small>
      </div>

      <div class="header-actions">
        <button class="square-button" type="button" aria-label={text('openAnother')} title={shortcutTitle(text('openAnother'), 'openVideo')} onclick={chooseVideo} disabled={isLoading || isPreparingProxy || exportJobId !== null}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5.5 3.5h8l4 4v5.2M5.5 3.5v17h7M13.5 3.5v4h4M16.5 16.5h5M19 14v5" /></svg>
        </button>
        <button class="square-button" type="button" aria-label={text('openFolder')} title={shortcutTitle(text('openFolder'), 'openFolder')} onclick={chooseDirectory} disabled={isLoading || isPreparingProxy || exportJobId !== null}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3.5 6.5h6l2 2h9v10.5a1.5 1.5 0 0 1-1.5 1.5h-14A1.5 1.5 0 0 1 3.5 19z" /><path d="M3.5 9h17" /></svg>
        </button>
        <button class="square-button settings-button" type="button" aria-label={text('settings')} title={shortcutTitle(text('settings'), 'openSettings')} onclick={openSettingsDialog} disabled={!settingsReady}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 8.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7Z" /><path d="M19.4 15a1.8 1.8 0 0 0 .36 1.98l.06.06-2.78 2.78-.06-.06A1.8 1.8 0 0 0 15 19.4a1.8 1.8 0 0 0-1.08 1.65V21h-3.84v-.08A1.8 1.8 0 0 0 9 19.4a1.8 1.8 0 0 0-1.98.36l-.06.06-2.78-2.78.06-.06A1.8 1.8 0 0 0 4.6 15a1.8 1.8 0 0 0-1.65-1.08H3v-3.84h.08A1.8 1.8 0 0 0 4.6 9a1.8 1.8 0 0 0-.36-1.98l-.06-.06 2.78-2.78.06.06A1.8 1.8 0 0 0 9 4.6a1.8 1.8 0 0 0 1.08-1.65V3h3.84v.08A1.8 1.8 0 0 0 15 4.6a1.8 1.8 0 0 0 1.98-.36l.06-.06 2.78 2.78-.06.06A1.8 1.8 0 0 0 19.4 9a1.8 1.8 0 0 0 1.65 1.08H21v3.84h-.08A1.8 1.8 0 0 0 19.4 15Z" /></svg>
        </button>
        {#if canOverwrite}
          <div class="save-options">
            <button
              class="button primary save-options-trigger"
              type="button"
              aria-haspopup="menu"
              aria-expanded={showSaveMenu}
              title={!exportEventsReady ? text('exportEventsUnavailable') : !profileSupported ? (timeTrimmed ? text('timeTrimMetadataUnavailable') : text('metadataUnavailable')) : `${text('saveOptions')} (${formatShortcutChord(settings.shortcuts.copySave)} / ${formatShortcutChord(settings.shortcuts.saveInPlace)})`}
              disabled={!exportEventsReady || !profileSupported || exportJobId !== null || isStartingExport}
              onclick={() => (showSaveMenu = !showSaveMenu)}
            >
              <span>{text('saveOptions')}</span>
              <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m3 6 5 5 5-5" /></svg>
            </button>
            {#if showSaveMenu}
              <div class="save-menu" role="menu">
                <button type="button" role="menuitem" title={shortcutTitle(text('copySave'), 'copySave')} onclick={saveCopy}>{text('copySave')}</button>
                <button type="button" role="menuitem" title={shortcutTitle(text('save'), 'saveInPlace')} onclick={saveInPlace}>{text('save')}</button>
              </div>
            {/if}
          </div>
        {:else}
          <button class="button primary" type="button" disabled={!exportEventsReady || !profileSupported || exportJobId !== null || isStartingExport} onclick={saveCopy} title={!exportEventsReady ? text('exportEventsUnavailable') : !profileSupported ? (timeTrimmed ? text('timeTrimMetadataUnavailable') : text('metadataUnavailable')) : shortcutTitle(text('copySave'), 'copySave')}>
            {text('copySave')}
          </button>
        {/if}
      </div>
    {:else}
      <button class="square-button settings-button launcher-settings" type="button" aria-label={text('settings')} title={shortcutTitle(text('settings'), 'openSettings')} onclick={openSettingsDialog} disabled={!settingsReady}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 8.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7Z" /><path d="M19.4 15a1.8 1.8 0 0 0 .36 1.98l.06.06-2.78 2.78-.06-.06A1.8 1.8 0 0 0 15 19.4a1.8 1.8 0 0 0-1.08 1.65V21h-3.84v-.08A1.8 1.8 0 0 0 9 19.4a1.8 1.8 0 0 0-1.98.36l-.06.06-2.78-2.78.06-.06A1.8 1.8 0 0 0 4.6 15a1.8 1.8 0 0 0-1.65-1.08H3v-3.84h.08A1.8 1.8 0 0 0 4.6 9a1.8 1.8 0 0 0-.36-1.98l-.06-.06 2.78-2.78.06.06A1.8 1.8 0 0 0 9 4.6a1.8 1.8 0 0 0 1.08-1.65V3h3.84v.08A1.8 1.8 0 0 0 15 4.6a1.8 1.8 0 0 0 1.98-.36l.06-.06 2.78 2.78-.06.06A1.8 1.8 0 0 0 19.4 9a1.8 1.8 0 0 0 1.65 1.08H21v3.84h-.08A1.8 1.8 0 0 0 19.4 15Z" /></svg>
      </button>
    {/if}
  </header>

  {#if media}
    <main class="editor-grid" class:inspector-collapsed={!showInspector}>
      <section class="stage-panel" aria-label={text('videoPreview')}>
        {#if playlist.length > 1}
          <div class="playlist-bar" title={directoryPath ?? ''}>
            <button class="playlist-nav" type="button" aria-label={text('previousVideo')} title={shortcutTitle(text('previousVideo'), 'previousVideo')} disabled={playlistIndex === 0 || isLoading || exportJobId !== null} onclick={() => navigatePlaylist(-1)}><svg viewBox="0 0 16 16" aria-hidden="true"><path d="m10 3-5 5 5 5" /></svg></button>
            <label>
              <span>{text('chooseFromFolder')}</span>
              <select value={playlistIndex} onchange={selectPlaylistVideo} disabled={isLoading || exportJobId !== null}>
                {#each playlist as path, index}
                  <option value={index}>{index + 1}. {fileName(path)}</option>
                {/each}
              </select>
            </label>
            <span class="playlist-count">{text('folderPosition', { current: playlistIndex + 1, total: playlist.length })}</span>
            <button class="playlist-nav" type="button" aria-label={text('nextVideo')} title={shortcutTitle(text('nextVideo'), 'nextVideo')} disabled={playlistIndex === playlist.length - 1 || isLoading || exportJobId !== null} onclick={() => navigatePlaylist(1)}><svg viewBox="0 0 16 16" aria-hidden="true"><path d="m6 3 5 5-5 5" /></svg></button>
          </div>
        {/if}

        <div class="video-stage" use:observeStage title={shortcutTitle(isVideoFullscreen ? text('exitFullscreen') : text('enterFullscreen'), 'toggleFullscreen')}>
          <div class="video-frame" style={frameStyle}>
            <video
              bind:this={videoElement}
              src={videoSrc}
              playsinline
              preload="metadata"
              onerror={handleVideoError}
              oncanplay={handleVideoCanPlay}
              ontimeupdate={handleTimeUpdate}
              onplay={() => (isPlaying = true)}
              onpause={() => (isPlaying = false)}
              onended={handleVideoEnded}
            ><track kind="captions" /></video>

            <div class="crop-layer">
              <div class="crop-box" class:is-dragging={dragState?.handle === 'move'} style={boxStyle} role="presentation" onpointerdown={(event) => beginCropDrag(event, 'move')}>
                <div class="thirds vertical one"></div><div class="thirds vertical two"></div>
                <div class="thirds horizontal one"></div><div class="thirds horizontal two"></div>
                {#each handles as handle}
                  <button class="crop-handle {handle.value}" type="button" aria-label={text(handle.label)} onpointerdown={(event) => beginCropDrag(event, handle.value)}></button>
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

      {#if showInspector}
      <aside class="inspector" aria-label={text('cropArea')}>
        <div class="inspector-heading">
          <div><span class="section-label">{text('frame')}</span><h2>{text('cropArea')}</h2></div>
          <div class="inspector-actions">
            <button class="text-button" type="button" onclick={resetCrop}>{text('reset')}</button>
            <button class="pane-button" type="button" aria-label={text('closeCropPane')} title={text('closeCropPane')} onclick={() => (showInspector = false)}>
              <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m6 3 5 5-5 5" /></svg>
            </button>
          </div>
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
      {:else}
        <div class="pane-restore inspector-restore">
          <button class="pane-button" type="button" aria-label={text('openCropPane')} title={text('openCropPane')} onclick={() => (showInspector = true)}>
            <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m10 3-5 5 5 5" /></svg>
          </button>
        </div>
      {/if}
    </main>

    {#if showTransport}
    <footer class="transport">
      <div class="transport-playback">
        <button class="icon-button" type="button" aria-label={isPlaying ? text('pause') : text('play')} title={shortcutTitle(isPlaying ? text('pause') : text('play'), 'playPause')} onclick={togglePlayback}>{isPlaying ? 'Ⅱ' : '▶'}</button>
        <span class="time current">{formatTime(currentTime)}</span>
      </div>

      <section class="trim-editor" aria-label={text('timeRange')}>
        <div class="trim-readout">
          <span>{text('trimStart')} <strong>{formatTime(trimStartSeconds)}</strong> · F{safeTrim.startFrame}</span>
          <span class="selected-duration">{text('selectedDuration')} <strong>{formatTime(selectedDuration)}</strong></span>
          <span>{text('trimEnd')} <strong>{formatTime(trimEndSeconds)}</strong> · F{safeTrim.endFrame}</span>
          {#if timeTrimmed}<button class="trim-reset" type="button" onclick={resetTimeTrim}>{text('resetTrim')}</button>{/if}
        </div>
        <div class="trim-timeline" class:is-dragging={timeTrimDragState !== null} style={timelineStripStyle}>
          <div class="timeline-fallback" aria-hidden="true"></div>
          <div class="trim-mask left" style={trimLeftMaskStyle} aria-hidden="true"></div>
          <div class="trim-mask right" style={trimRightMaskStyle} aria-hidden="true"></div>
          <div class="trim-selection" style={trimSelectionStyle} aria-hidden="true"></div>
          <input
            class="timeline-scrubber"
            type="range"
            aria-label={text('seek')}
            min="0"
            max={duration}
            step={duration / Math.max(1, totalFrames)}
            value={currentTime}
            disabled={exportLocked}
            oninput={scrubTo}
            onpointerdown={beginTimelineScrub}
            onkeydown={handlePlaybackScrubberKey}
          />
          <span class="timeline-playhead" style={playheadStyle} aria-hidden="true"></span>
          <button
            class="trim-handle start"
            class:active={timeTrimDragState?.handle === 'start'}
            class:selected={selectedTrimHandle === 'start'}
            style={`left:${(safeTrim.startFrame / totalFrames) * 100}%`}
            type="button"
            role="slider"
            aria-label={text('startTrimHandle')}
            aria-valuemin="0"
            aria-valuemax={safeTrim.endFrame - 1}
            aria-valuenow={safeTrim.startFrame}
            aria-valuetext={formatTime(trimStartSeconds)}
            disabled={exportLocked}
            onpointerdown={(event) => beginTimeTrimDrag(event, 'start')}
            onkeydown={(event) => handleTrimKey(event, 'start')}
            onfocus={() => selectTrimHandle('start')}
            onblur={handleTrimBlur}
          ><span aria-hidden="true"></span></button>
          <button
            class="trim-handle end"
            class:active={timeTrimDragState?.handle === 'end'}
            class:selected={selectedTrimHandle === 'end'}
            style={`left:${(safeTrim.endFrame / totalFrames) * 100}%`}
            type="button"
            role="slider"
            aria-label={text('endTrimHandle')}
            aria-valuemin={safeTrim.startFrame + 1}
            aria-valuemax={totalFrames}
            aria-valuenow={safeTrim.endFrame}
            aria-valuetext={formatTime(trimEndSeconds)}
            disabled={exportLocked}
            onpointerdown={(event) => beginTimeTrimDrag(event, 'end')}
            onkeydown={(event) => handleTrimKey(event, 'end')}
            onfocus={() => selectTrimHandle('end')}
            onblur={handleTrimBlur}
          ><span aria-hidden="true"></span></button>
        </div>
      </section>

      <div class="transport-options">
        <span class="time total">{formatTime(duration)}</span>
        <button class="icon-button transport-option" class:active={settings.loopPlayback} type="button" aria-label={settings.loopPlayback ? text('disableLoop') : text('enableLoop')} title={settings.loopPlayback ? text('disableLoop') : text('enableLoop')} onclick={toggleLoop}>↻</button>
        <button class="icon-button transport-option" type="button" aria-label={isMuted ? text('unmute') : text('mute')} title={isMuted ? text('unmute') : text('mute')} onclick={toggleMute}>{isMuted ? '×' : '♪'}</button>
        <button class="pane-button transport-close" type="button" aria-label={text('closeTrimPane')} title={text('closeTrimPane')} onclick={() => (showTransport = false)}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m3 6 5 5 5-5" /></svg>
        </button>
      </div>
    </footer>
    {:else}
      <div class="pane-restore transport-restore">
        <button class="pane-button" type="button" aria-label={text('openTrimPane')} title={text('openTrimPane')} onclick={() => (showTransport = true)}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m3 10 5-5 5 5" /></svg>
        </button>
      </div>
    {/if}
  {:else}
    <main class="empty-state" use:observeStage>
      <div class="empty-visual" aria-hidden="true">
        <span class="corner top-left"></span><span class="corner top-right"></span>
        <span class="corner bottom-left"></span><span class="corner bottom-right"></span>
        <span class="play-symbol">▶</span>
      </div>
      <p class="empty-description">{text('emptyDescription')}</p>
      <div class="empty-actions">
        <button class="button primary large" type="button" title={shortcutTitle(text('openVideo'), 'openVideo')} onclick={chooseVideo} disabled={isLoading}>{text('openVideo')}</button>
        <button class="button secondary large" type="button" title={shortcutTitle(text('openFolder'), 'openFolder')} onclick={chooseDirectory} disabled={isLoading}>{text('openFolder')}</button>
      </div>
    </main>
  {/if}

  {#if exportJobId && media}
    <div class="inline-export" role="status">
      <div><strong>{text('exporting')}</strong><span>{Math.round(exportProgress * 100)}% · {formatTime(exportOutTime)} / {formatTime(selectedDuration)}</span></div>
      <div class="inline-progress"><span style={`width:${exportProgress * 100}%`}></span></div>
      <button type="button" onclick={cancelExport}>{text('cancel')}</button>
    </div>
  {/if}

  {#if showSettings}
    <div class="modal-backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && closeSettingsDialog()}>
      <div class="settings-dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title">
        <div class="dialog-heading">
          <h2 id="settings-title">{text('settingsTitle')}</h2>
          <button class="dialog-close" type="button" aria-label={text('close')} onclick={closeSettingsDialog}>×</button>
        </div>

        <div class="settings-layout">
          <nav class="settings-nav" aria-label={text('settingsCategories')}>
            {#each settingsCategories as category}
              <button
                class:active={settingsCategory === category.value}
                type="button"
                data-settings-category={category.value}
                aria-current={settingsCategory === category.value ? 'page' : undefined}
                onclick={() => selectSettingsCategory(category.value)}
                onkeydown={(event) => handleSettingsNavigation(event, category.value)}
              >{text(category.label)}</button>
            {/each}
          </nav>

          <div class="settings-page">
          {#if settingsCategory === 'export'}
            <section class="settings-section">
              <h3>{text('settingsExport')}</h3>
              <div class="profile-settings">
                {#each exportProfiles as profile}
                  <button class:active={settingsDraft.export.profile === profile} type="button" onclick={() => setProfile(profile as ExportProfile)}>
                    <strong>{profileName(profile as ExportProfile)}</strong>
                    <small>{text(`${profile}Description` as TranslationKey)}</small>
                  </button>
                {/each}
              </div>
            </section>

            {#if settingsDraft.export.profile !== 'metadata'}
              <section class="settings-section">
                <h4>{text('encodingSettings')}</h4>
                <div class="settings-grid">
                  {#if settingsDraft.export.profile === 'compatible'}
                    <label class="settings-field"><span>{text('videoCodec')}</span><select value={settingsDraft.export.videoCodec} onchange={(event) => updateExportDraft('videoCodec', (event.currentTarget as HTMLSelectElement).value as VideoCodec)}><option value="h264">H.264</option><option value="h265">H.265 / HEVC</option></select></label>
                    <label class="settings-field"><span>{text('encoder')}</span><select value={settingsDraft.export.encoder} onchange={(event) => updateExportDraft('encoder', (event.currentTarget as HTMLSelectElement).value as VideoEncoder)}><option value="automatic">{text('automaticEncoder')}</option><option value="nvidia" disabled={!encoderAvailable(settingsDraft.export.videoCodec, 'nvidia')}>nvenc</option><option value="intel" disabled={!encoderAvailable(settingsDraft.export.videoCodec, 'intel')}>qsv</option><option value="amd" disabled={!encoderAvailable(settingsDraft.export.videoCodec, 'amd')}>amf</option><option value="software">{settingsDraft.export.videoCodec === 'h264' ? 'libx264' : 'libx265'}</option></select></label>
                    <label class="settings-field"><span>{text('crf')}</span><input type="number" min="0" max="51" step="1" value={settingsDraft.export.crf} onchange={(event) => updateExportDraft('crf', Number((event.currentTarget as HTMLInputElement).value))} /><small>{text('crfHint')}</small></label>
                    <label class="settings-field"><span>{text('preset')}</span><select value={settingsDraft.export.preset} onchange={(event) => updateExportDraft('preset', (event.currentTarget as HTMLSelectElement).value as EncoderPreset)}>{#each encoderPresets as preset}<option value={preset}>{preset}</option>{/each}</select></label>
                  {/if}
                  <label class="settings-field"><span>{text('pixelFormatSetting')}</span><select value={settingsDraft.export.pixelFormat} onchange={(event) => updateExportDraft('pixelFormat', (event.currentTarget as HTMLSelectElement).value as PixelFormat)}>{#each pixelFormats as format}<option value={format}>{format === 'source' ? text('sourcePixelFormat') : format}</option>{/each}</select></label>
                </div>
              </section>

              <section class="settings-section">
                <h4>{text('audioSettings')}</h4>
                <div class="settings-grid">
                  <label class="settings-field"><span>{text('audioMode')}</span><select value={settingsDraft.export.audioMode} onchange={(event) => updateExportDraft('audioMode', (event.currentTarget as HTMLSelectElement).value as AudioMode)}><option value="auto">{text('audioAuto')}</option><option value="copy">{text('audioCopy')}</option><option value="aac">{text('audioAac')}</option>{#if settingsDraft.export.profile === 'lossless'}<option value="flac">{text('audioFlac')}</option><option value="pcm">{text('audioPcm')}</option>{/if}<option value="none">{text('audioNone')}</option></select></label>
                  <label class="settings-field"><span>{text('audioBitrate')}</span><div class="unit-input"><input type="number" min="32" max="1024" step="8" value={settingsDraft.export.audioBitrateKbps} disabled={settingsDraft.export.audioMode !== 'aac'} onchange={(event) => updateExportDraft('audioBitrateKbps', Number((event.currentTarget as HTMLInputElement).value))} /><em>kbps</em></div></label>
                </div>
              </section>

              <section class="settings-section">
                <h4>{text('timingSettings')}</h4>
                <div class="settings-grid">
                  <label class="settings-field"><span>{text('frameRateMode')}</span><select value={settingsDraft.export.frameRateMode} onchange={(event) => updateExportDraft('frameRateMode', (event.currentTarget as HTMLSelectElement).value as FrameRateMode)}><option value="passthrough">{text('fpsPassthrough')}</option><option value="constant">{text('fpsConstant')}</option></select></label>
                  <label class="settings-field"><span>{text('constantFps')}</span><div class="unit-input"><input type="number" min="1" max="240" step="0.001" value={settingsDraft.export.constantFrameRate} disabled={settingsDraft.export.frameRateMode !== 'constant'} onchange={(event) => updateExportDraft('constantFrameRate', Number((event.currentTarget as HTMLInputElement).value))} /><em>fps</em></div></label>
                </div>
              </section>

              <section class="settings-section">
                <h4>{text('fileSettings')}</h4>
                <div class="check-list">
                  <label><input type="checkbox" checked={settingsDraft.export.preserveMetadata} onchange={(event) => updateExportDraft('preserveMetadata', (event.currentTarget as HTMLInputElement).checked)} />{text('preserveMetadata')}</label>
                  {#if settingsDraft.export.profile === 'lossless'}<label><input type="checkbox" checked={settingsDraft.export.copySubtitles} onchange={(event) => updateExportDraft('copySubtitles', (event.currentTarget as HTMLInputElement).checked)} />{text('copySubtitles')}</label>{/if}
                </div>
              </section>
            {:else}
              <p class="metadata-warning">{text('metadataNote')}</p>
            {/if}
          {:else if settingsCategory === 'playback'}
            <section class="settings-section">
              <h3>{text('settingsPlayback')}</h3>
              <div class="check-list"><label><input type="checkbox" checked={settingsDraft.loopPlayback} onchange={(event) => updateDraft('loopPlayback', (event.currentTarget as HTMLInputElement).checked)} />{text('enableLoop')}</label></div>
            </section>
          {:else if settingsCategory === 'appearance'}
            <section class="settings-section">
              <h3>{text('settingsAppearance')}</h3>
              <div class="appearance-group">
                <h4>{text('themeMode')}</h4>
                <div class="radio-row">
                  <label><input type="radio" name="theme-mode" checked={settingsDraft.appearance.themeMode === 'system'} onchange={() => updateAppearanceDraft('themeMode', 'system' as AppearanceMode)} />{text('appearanceSystem')}</label>
                  <label><input type="radio" name="theme-mode" checked={settingsDraft.appearance.themeMode === 'manual'} onchange={() => updateAppearanceDraft('themeMode', 'manual' as AppearanceMode)} />{text('appearanceManual')}</label>
                </div>
                <label class="settings-field compact"><span>{text('themeChoice')}</span><select value={settingsDraft.appearance.theme} disabled={settingsDraft.appearance.themeMode !== 'manual'} onchange={(event) => updateAppearanceDraft('theme', (event.currentTarget as HTMLSelectElement).value as AppTheme)}><option value="light">{text('lightTheme')}</option><option value="dark">{text('darkTheme')}</option></select></label>
              </div>
              <div class="appearance-group">
                <h4>{text('accentColor')}</h4>
                <div class="radio-row">
                  <label><input type="radio" name="accent-mode" checked={settingsDraft.appearance.accentMode === 'system'} onchange={() => updateAppearanceDraft('accentMode', 'system' as AppearanceMode)} />{text('appearanceSystem')}</label>
                  <label><input type="radio" name="accent-mode" checked={settingsDraft.appearance.accentMode === 'manual'} onchange={() => updateAppearanceDraft('accentMode', 'manual' as AppearanceMode)} />{text('appearanceManual')}</label>
                </div>
                <button class="palette-trigger" type="button" aria-label={showAccentPalette ? text('collapseAccentColors') : text('chooseAccentColor')} aria-expanded={showAccentPalette} disabled={settingsDraft.appearance.accentMode !== 'manual'} title={showAccentPalette ? text('collapseAccentColors') : text('chooseAccentColor')} onclick={() => (showAccentPalette = !showAccentPalette)}>
                  <span class="color-swatch" style={`background:${accentPalette[settingsDraft.appearance.accentColor][resolvedAppearance.theme]}`}></span>
                  <span>{text(accentLabels[settingsDraft.appearance.accentColor])}</span>
                  <span aria-hidden="true">{showAccentPalette ? '▴' : '▾'}</span>
                </button>
                {#if showAccentPalette && settingsDraft.appearance.accentMode === 'manual'}
                  <div class="accent-palette" role="listbox" aria-label={text('accentColorChoice')}>
                    {#each accentColorIds as color}
                      <button type="button" role="option" aria-selected={settingsDraft.appearance.accentColor === color} title={text(accentLabels[color])} onclick={() => updateAppearanceDraft('accentColor', color)}>
                        <span class="color-swatch" style={`background:${accentPalette[color][resolvedAppearance.theme]}`}></span>
                        <span>{text(accentLabels[color])}</span>
                      </button>
                    {/each}
                  </div>
                {/if}
              </div>
            </section>
          {:else if settingsCategory === 'shortcuts'}
            <section class="settings-section">
              <h3>{text('settingsShortcuts')}</h3>
              <p class="shortcut-help">{text('shortcutHelp')}</p>
              <div class="shortcut-list">
                {#each shortcutActionIds as action}
                  <div class="shortcut-row">
                    <span>{text(shortcutLabels[action])}</span>
                    <button class:recording={shortcutRecording === action} type="button" aria-label={`${text('recordShortcut')}: ${text(shortcutLabels[action])}`} aria-pressed={shortcutRecording === action} onclick={() => startShortcutRecording(action)}>
                      <kbd>{shortcutRecording === action ? text('pressShortcut') : formatShortcutChord(settingsDraft.shortcuts[action])}</kbd>
                    </button>
                  </div>
                {/each}
              </div>
              {#if shortcutError}<p class="shortcut-error" role="alert">{shortcutError}</p>{/if}
              <button class="button secondary reset-shortcuts" type="button" onclick={resetShortcuts}>{text('resetShortcuts')}</button>
            </section>
          {:else if settingsCategory === 'explorer'}
            <section class="settings-section">
              <h3>{text('explorerIntegration')}</h3>
              <div class="check-list"><label><input type="checkbox" checked={settingsDraft.explorerIntegration} onchange={(event) => updateDraft('explorerIntegration', (event.currentTarget as HTMLInputElement).checked)} />{text('enableExplorerIntegration')}</label></div>
            </section>
          {:else}
            <section class="settings-section">
              <h3>{text('language')}</h3>
              <div class="radio-row">
                <label><input type="radio" name="language-mode" checked={settingsDraft.languageMode === 'system'} onchange={() => updateDraft('languageMode', 'system' as LanguageMode)} />{text('languageSystem')}</label>
                <label><input type="radio" name="language-mode" checked={settingsDraft.languageMode === 'manual'} onchange={() => updateDraft('languageMode', 'manual' as LanguageMode)} />{text('languageManual')}</label>
              </div>
              <label class="settings-field compact"><span>{text('language')}</span><select value={settingsDraft.language} disabled={settingsDraft.languageMode !== 'manual'} onchange={(event) => updateDraft('language', (event.currentTarget as HTMLSelectElement).value as Language)}><option value="ja">{text('japanese')}</option><option value="en">{text('english')}</option></select></label>
            </section>
          {/if}
          </div>
        </div>

        <div class="dialog-actions">
          <button class="button primary" type="button" onclick={closeSettingsDialog}>{text('close')}</button>
        </div>
      </div>
    </div>
  {/if}

  {#if errorMessage}<div class="error-banner" role="alert"><span>{errorMessage}</span><button type="button" aria-label={text('closeError')} onclick={() => (errorMessage = '')}>×</button></div>{/if}
  {#if successPath}
    <div class="success-banner" role="status">
      <span class="success-label">{text('saved')}</span>
      <a class="success-path" href={successPath} title={text('openSavedLocation')} aria-label={`${text('openSavedLocation')}: ${successPath}`} onclick={(event) => { event.preventDefault(); void revealSavedFile(); }}>{successPath}</a>
      <button class="notice-close" type="button" aria-label={text('closeNotice')} onclick={dismissSuccess}>×</button>
    </div>
  {/if}
</div>
