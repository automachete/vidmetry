<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import { convertFileSrc, invoke } from '@tauri-apps/api/core';
  import { open, save } from '@tauri-apps/plugin-dialog';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';

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
  import { formatFrameRate, formatTime, type MediaDescriptor } from './lib/media';
  import {
    clampProgress,
    suggestOutput,
    type ExportCompleteEvent,
    type ExportErrorEvent,
    type ExportProfile,
    type ExportProgressEvent,
  } from './lib/export';

  const handles: Array<{ value: CropHandle; label: string }> = [
    { value: 'north-west', label: '左上を調整' },
    { value: 'north', label: '上辺を調整' },
    { value: 'north-east', label: '右上を調整' },
    { value: 'east', label: '右辺を調整' },
    { value: 'south-east', label: '右下を調整' },
    { value: 'south', label: '下辺を調整' },
    { value: 'south-west', label: '左下を調整' },
    { value: 'west', label: '左辺を調整' },
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
  let dragState: DragState | null = null;
  let seekFrame: number | null = null;
  let unlistenDragDrop: UnlistenFn | undefined;
  let unlistenExportEvents: UnlistenFn[] = [];
  let showExportDialog = false;
  let selectedProfile: ExportProfile = 'compatible';
  let exportJobId: string | null = null;
  let exportProgress = 0;
  let exportOutTime = 0;
  let isStartingExport = false;
  let successMessage = '';

  $: bounds = media
    ? { width: media.displayWidth, height: media.displayHeight }
    : { width: 16, height: 16 };
  $: frameGeometry = containFrame(stageWidth, stageHeight, bounds);
  $: frameStyle = `left:${frameGeometry.left}px;top:${frameGeometry.top}px;width:${frameGeometry.width}px;height:${frameGeometry.height}px`;
  $: boxStyle = cropStyle(crop, bounds);
  $: activeRatio = aspectRatio(aspect, bounds);
  $: duration = Math.max(media?.durationSeconds ?? 0, videoElement?.duration || 0);

  onMount(() => {
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === 'drop' && event.payload.paths[0]) {
          void loadVideo(event.payload.paths[0]);
        }
      })
      .then((unlisten) => {
        unlistenDragDrop = unlisten;
      })
      .catch(() => undefined);

    window.addEventListener('keydown', handleKeyboard);

    void Promise.all([
      listen<ExportProgressEvent>('export-progress', (event) => {
        if (event.payload.jobId !== exportJobId) return;
        exportProgress = clampProgress(event.payload.fraction);
        exportOutTime = event.payload.outTimeSeconds;
      }),
      listen<ExportCompleteEvent>('export-complete', (event) => {
        if (event.payload.jobId !== exportJobId) return;
        exportProgress = 1;
        exportJobId = null;
        showExportDialog = false;
        successMessage = `保存しました: ${event.payload.outputPath}`;
      }),
      listen<ExportErrorEvent>('export-error', (event) => {
        if (event.payload.jobId !== exportJobId) return;
        exportJobId = null;
        exportProgress = 0;
        if (event.payload.cancelled) {
          showExportDialog = false;
        } else {
          errorMessage = `書き出しに失敗しました。${event.payload.message}`;
        }
      }),
    ]).then((unlisteners) => {
      unlistenExportEvents = unlisteners;
    });
  });

  onDestroy(() => {
    unlistenDragDrop?.();
    for (const unlisten of unlistenExportEvents) unlisten();
    window.removeEventListener('keydown', handleKeyboard);
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
    return {
      destroy() {
        observer.disconnect();
      },
    };
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
          name: '動画',
          extensions: ['mp4', 'mov', 'mkv', 'webm', 'avi', 'm4v', 'wmv', 'mts', 'm2ts', 'mpg', 'mpeg'],
        },
      ],
    });
    if (typeof selected === 'string') await loadVideo(selected);
  }

  async function loadVideo(path: string) {
    if (isLoading || isPreparingProxy) return;
    isLoading = true;
    errorMessage = '';
    usingProxy = false;
    currentTime = 0;
    isPlaying = false;
    videoElement?.pause();

    try {
      const descriptor = await invoke<MediaDescriptor>('probe_video', { path });
      media = descriptor;
      crop = fullFrame({ width: descriptor.displayWidth, height: descriptor.displayHeight });
      aspect = 'free';
      if (!descriptor.metadataCropSupported && selectedProfile === 'metadata') {
        selectedProfile = 'compatible';
      }
      videoSrc = convertFileSrc(descriptor.sourcePath);
      await tick();
      videoElement?.load();
    } catch (error) {
      errorMessage = readableError(error);
    } finally {
      isLoading = false;
    }
  }

  async function handleVideoError() {
    if (!media || usingProxy || isPreparingProxy || isLoading) return;
    isPreparingProxy = true;
    errorMessage = '';
    try {
      const proxyPath = await invoke<string>('create_preview', { path: media.sourcePath });
      usingProxy = true;
      videoSrc = convertFileSrc(proxyPath);
      await tick();
      videoElement?.load();
    } catch (error) {
      errorMessage = `プレビューを準備できませんでした。${readableError(error)}`;
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

  function handleKeyboard(event: KeyboardEvent) {
    const target = event.target as HTMLElement | null;
    if (target?.matches('input, select, textarea, button')) return;
    if (!media) return;
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
    crop = dragCrop(
      dragState.start,
      dragState.handle,
      delta.x,
      delta.y,
      bounds,
      activeRatio,
    );
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

  function openExportDialog() {
    if (!media || exportJobId) return;
    successMessage = '';
    exportProgress = 0;
    exportOutTime = 0;
    showExportDialog = true;
  }

  function closeExportDialog() {
    if (!exportJobId && !isStartingExport) showExportDialog = false;
  }

  async function beginExport() {
    if (!media || exportJobId || isStartingExport) return;
    isStartingExport = true;
    errorMessage = '';
    try {
      const suggestion = suggestOutput(media.sourcePath, selectedProfile);
      const outputPath = await save({
        defaultPath: suggestion.path,
        filters: [{ name: suggestion.filterName, extensions: [suggestion.extension] }],
      });
      if (!outputPath) return;
      videoElement?.pause();
      exportProgress = 0;
      exportOutTime = 0;
      exportJobId = await invoke<string>('start_export', {
        request: {
          sourcePath: media.sourcePath,
          outputPath,
          crop,
          profile: selectedProfile,
          // The native save dialog has already asked before replacing a file.
          overwrite: true,
        },
      });
    } catch (error) {
      errorMessage = `書き出しを開始できませんでした。${readableError(error)}`;
    } finally {
      isStartingExport = false;
    }
  }

  async function cancelExport() {
    if (!exportJobId) return;
    try {
      await invoke('cancel_export', { jobId: exportJobId });
    } catch (error) {
      errorMessage = readableError(error);
    }
  }

  function readableError(error: unknown): string {
    if (typeof error === 'string') return error;
    if (error instanceof Error) return error.message;
    return '不明なエラーが発生しました。';
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
      </div>
    {:else}
      <div class="source-summary"><span>ローカル動画クロッパー</span></div>
    {/if}

    <div class="header-actions">
      <button class="button secondary" type="button" onclick={chooseVideo} disabled={isLoading || isPreparingProxy}>
        {media ? '別の動画を開く' : '動画を開く'}
      </button>
      <button class="button primary" type="button" disabled={!media || exportJobId !== null} onclick={openExportDialog}>
        書き出し
      </button>
    </div>
  </header>

  {#if media}
    <main class="editor-grid">
      <section class="stage-panel" aria-label="動画プレビュー">
        <div class="video-stage" use:observeStage>
          <div class="video-frame" style={frameStyle}>
            <video
              bind:this={videoElement}
              src={videoSrc}
              playsinline
              preload="metadata"
              onerror={handleVideoError}
              onloadedmetadata={handleLoadedMetadata}
              ontimeupdate={() => (currentTime = videoElement.currentTime)}
              onplay={() => (isPlaying = true)}
              onpause={() => (isPlaying = false)}
              onended={() => (isPlaying = false)}
            >
              <track kind="captions" />
            </video>

            <div class="crop-layer">
              <div
                class="crop-box"
                class:is-dragging={dragState?.handle === 'move'}
                style={boxStyle}
                role="presentation"
                onpointerdown={(event) => beginCropDrag(event, 'move')}
              >
                <div class="thirds vertical one"></div>
                <div class="thirds vertical two"></div>
                <div class="thirds horizontal one"></div>
                <div class="thirds horizontal two"></div>
                {#each handles as handle}
                  <button
                    class="crop-handle {handle.value}"
                    type="button"
                    aria-label={handle.label}
                    onpointerdown={(event) => beginCropDrag(event, handle.value)}
                  ></button>
                {/each}
              </div>
            </div>
          </div>

          {#if isLoading || isPreparingProxy}
            <div class="stage-status" role="status">
              <span class="spinner"></span>
              <strong>{isPreparingProxy ? '互換プレビューを作成中…' : '動画を解析中…'}</strong>
              {#if isPreparingProxy}<small>元動画は変更しません</small>{/if}
            </div>
          {/if}
        </div>
        {#if usingProxy}
          <p class="proxy-note">互換プロキシでプレビュー中 · 書き出しには元動画を使用します</p>
        {/if}
      </section>

      <aside class="inspector" aria-label="クロップ設定">
        <div class="inspector-heading">
          <div>
            <span class="section-label">FRAME</span>
            <h2>切り取り範囲</h2>
          </div>
          <button class="text-button" type="button" onclick={resetCrop}>リセット</button>
        </div>

        <label class="field full-width">
          <span>縦横比</span>
          <select value={aspect} onchange={setAspect}>
            <option value="free">自由</option>
            <option value="source">元の比率</option>
            <option value="1:1">1 : 1</option>
            <option value="4:3">4 : 3</option>
            <option value="16:9">16 : 9</option>
            <option value="9:16">9 : 16</option>
          </select>
        </label>

        <div class="field-grid">
          <label class="field">
            <span>X</span>
            <div class="number-input"><input type="number" min="0" step="2" value={crop.x} onchange={(event) => updateCropField('x', event)} /><em>px</em></div>
          </label>
          <label class="field">
            <span>Y</span>
            <div class="number-input"><input type="number" min="0" step="2" value={crop.y} onchange={(event) => updateCropField('y', event)} /><em>px</em></div>
          </label>
          <label class="field">
            <span>幅</span>
            <div class="number-input"><input type="number" min="16" step="2" value={crop.width} onchange={(event) => updateCropField('width', event)} /><em>px</em></div>
          </label>
          <label class="field">
            <span>高さ</span>
            <div class="number-input"><input type="number" min="16" step="2" value={crop.height} onchange={(event) => updateCropField('height', event)} /><em>px</em></div>
          </label>
        </div>

        <div class="output-size">
          <span>出力フレーム</span>
          <strong>{crop.width} × {crop.height}</strong>
        </div>

        <div class="divider"></div>

        <div class="media-details">
          <span class="section-label">SOURCE</span>
          <dl>
            <div><dt>コーデック</dt><dd>{media.videoCodec.toUpperCase()}</dd></div>
            <div><dt>画素形式</dt><dd>{media.pixelFormat}{media.bitDepth ? ` · ${media.bitDepth}bit` : ''}</dd></div>
            <div><dt>回転</dt><dd>{media.rotationDegrees}°</dd></div>
            <div><dt>音声</dt><dd>{media.audioCodec?.toUpperCase() ?? 'なし'}</dd></div>
          </dl>
        </div>
      </aside>
    </main>

    <footer class="transport">
      <button class="icon-button" type="button" aria-label={isPlaying ? '一時停止' : '再生'} onclick={togglePlayback}>
        {isPlaying ? 'Ⅱ' : '▶'}
      </button>
      <span class="time current">{formatTime(currentTime)}</span>
      <input
        class="scrubber"
        type="range"
        aria-label="再生位置"
        min="0"
        max={duration || 0}
        step="0.001"
        value={currentTime}
        oninput={scrubTo}
      />
      <span class="time">{formatTime(duration)}</span>
      <button class="icon-button mute" type="button" aria-label={isMuted ? 'ミュート解除' : 'ミュート'} onclick={toggleMute}>
        {isMuted ? '×' : '♪'}
      </button>
    </footer>
  {:else}
    <main class="empty-state" use:observeStage>
      <div class="empty-visual" aria-hidden="true">
        <span class="corner top-left"></span>
        <span class="corner top-right"></span>
        <span class="corner bottom-left"></span>
        <span class="corner bottom-right"></span>
        <span class="play-symbol">▶</span>
      </div>
      <p class="eyebrow">SPATIAL VIDEO CROP</p>
      <h1>映像の必要な部分だけを、正確に。</h1>
      <p>動画をここへドロップするか、ファイルを選択してください。メディアはこのPC内だけで処理されます。</p>
      <button class="button primary large" type="button" onclick={chooseVideo} disabled={isLoading}>動画を選択</button>
      <span class="shortcut">MP4 · MOV · MKV · WebM ほか</span>
    </main>
  {/if}

  {#if showExportDialog && media}
    <div class="modal-backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && closeExportDialog()}>
      <div class="export-dialog" role="dialog" aria-modal="true" aria-labelledby="export-title">
        <div class="dialog-heading">
          <div>
            <span class="section-label">EXPORT</span>
            <h2 id="export-title">動画を書き出す</h2>
          </div>
          <button class="dialog-close" type="button" aria-label="閉じる" disabled={exportJobId !== null} onclick={closeExportDialog}>×</button>
        </div>

        {#if exportJobId}
          <div class="export-running" role="status">
            <div class="progress-ring" style={`--progress:${Math.round(exportProgress * 360)}deg`}>
              <span>{Math.round(exportProgress * 100)}<small>%</small></span>
            </div>
            <div>
              <h3>書き出しています…</h3>
              <p>{formatTime(exportOutTime)} / {formatTime(media.durationSeconds)}</p>
              <small>完成するまで元動画と保存先の一時ファイルには触れません。</small>
            </div>
          </div>
          <div class="progress-track"><span style={`width:${exportProgress * 100}%`}></span></div>
          <button class="button danger full" type="button" onclick={cancelExport}>キャンセル</button>
        {:else}
          <div class="profile-list" aria-label="保存方式">
            <button class:active={selectedProfile === 'compatible'} type="button" onclick={() => (selectedProfile = 'compatible')}>
              <span class="profile-icon">MP4</span>
              <span><strong>互換MP4</strong><small>H.264 · CRF 17。小さく扱いやすい高画質ファイル。</small></span>
              <em>推奨</em>
            </button>
            <button class:active={selectedProfile === 'lossless'} type="button" onclick={() => (selectedProfile = 'lossless')}>
              <span class="profile-icon">FFV1</span>
              <span><strong>可逆保存</strong><small>デコード後の画素を劣化させないMKV。容量は大きくなります。</small></span>
            </button>
            <button
              class:active={selectedProfile === 'metadata'}
              type="button"
              disabled={!media.metadataCropSupported}
              onclick={() => (selectedProfile = 'metadata')}
            >
              <span class="profile-icon">COPY</span>
              <span>
                <strong>メタデータのみ</strong>
                <small>{media.metadataCropSupported ? '映像を再圧縮せず表示領域を指定。非対応プレイヤーでは無視されます。' : 'H.264 / HEVC素材でのみ利用できます。'}</small>
              </span>
            </button>
          </div>

          <div class="export-summary">
            <span>出力フレーム</span>
            <strong>{selectedProfile === 'metadata' ? '符号化サイズは維持' : `${crop.width} × ${crop.height}`}</strong>
          </div>
          <div class="dialog-actions">
            <button class="button secondary" type="button" onclick={closeExportDialog}>戻る</button>
            <button class="button primary" type="button" disabled={isStartingExport} onclick={beginExport}>
              {isStartingExport ? '保存先を確認中…' : '保存先を選んで開始'}
            </button>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  {#if errorMessage}
    <div class="error-banner" role="alert">
      <span>{errorMessage}</span>
      <button type="button" aria-label="エラーを閉じる" onclick={() => (errorMessage = '')}>×</button>
    </div>
  {/if}
  {#if successMessage}
    <div class="success-banner" role="status">
      <span>{successMessage}</span>
      <button type="button" aria-label="通知を閉じる" onclick={() => (successMessage = '')}>×</button>
    </div>
  {/if}
</div>
