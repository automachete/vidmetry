<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  let status = 'ローカルで、正確に、映像を切り取る。';

  async function verifyBackend() {
    try {
      status = await invoke<string>('health_check');
    } catch {
      status = 'ブラウザプレビューとして実行中';
    }
  }
</script>

<svelte:head>
  <title>Vidmetry</title>
</svelte:head>

<main class="welcome-shell">
  <section class="welcome-card" aria-labelledby="welcome-title">
    <div class="mark" aria-hidden="true">V</div>
    <p class="eyebrow">VIDMETRY 0.1</p>
    <h1 id="welcome-title">動画のフレームを、迷わず切り取る。</h1>
    <p class="lead">再生しながら範囲を決め、品質を理解して保存するための小さなデスクトップツールです。</p>
    <button type="button" onclick={verifyBackend}>バックエンドを確認</button>
    <p class="status" role="status">{status}</p>
  </section>
</main>

