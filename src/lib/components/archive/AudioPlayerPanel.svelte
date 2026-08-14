<script lang="ts">
  import { onDestroy } from 'svelte';
  import { getAudioChunkUrl } from '$lib/api';
  import { requestWakeLock, releaseWakeLock } from '$lib/native/wakelock';
  import { triggerHaptic } from '$lib/native/haptics';

  let { sessionCode = '' }: { sessionCode?: string } = $props();

  let audioElement: HTMLAudioElement | null = $state(null);
  let isPlaying = $state(false);
  let currentTime = $state(0);
  let duration = $state(0);
  let playbackRate = $state<'1.0x' | '1.25x' | '1.5x'>('1.0x');

  const chunkUrl = $derived(sessionCode ? getAudioChunkUrl(sessionCode, 1) : '');
  const numericRate = $derived(playbackRate === '1.25x' ? 1.25 : playbackRate === '1.5x' ? 1.5 : 1.0);

  $effect(() => {
    if (audioElement) audioElement.playbackRate = numericRate;
  });

  function togglePlay() {
    if (!audioElement) return;
    triggerHaptic('light');
    if (audioElement.paused) {
      void audioElement.play().catch(() => {});
      isPlaying = true;
      void requestWakeLock();
    } else {
      audioElement.pause();
      isPlaying = false;
      void releaseWakeLock();
    }
  }

  function setSpeed(rate: '1.0x' | '1.25x' | '1.5x') {
    triggerHaptic('light');
    playbackRate = rate;
  }

  function formatTime(secs: number): string {
    if (!secs || isNaN(secs)) return '00:00';
    return `${Math.floor(secs / 60).toString().padStart(2, '0')}:${Math.floor(secs % 60).toString().padStart(2, '0')}`;
  }

  onDestroy(() => {
    void releaseWakeLock();
  });
</script>

<div class="panel audio-player-panel">
  <div class="player-header">
    <p class="eyebrow">LECTURE AUDIO REPLAY</p>
    <span class="player-format">48kHz PCM STEREOPHONIC</span>
  </div>

  <audio
    bind:this={audioElement}
    src={chunkUrl}
    preload="metadata"
    bind:currentTime
    bind:duration
    onplay={() => { isPlaying = true; void requestWakeLock(); }}
    onpause={() => { isPlaying = false; void releaseWakeLock(); }}
    onended={() => { isPlaying = false; void releaseWakeLock(); }}
  ></audio>

  <div class="player-controls">
    <div class="progress-wrap">
      <input type="range" min="0" max={duration || 100} step="0.1" bind:value={currentTime} class="audio-scrubber" />
      <div class="time-stamps"><span>{formatTime(currentTime)}</span><span>{formatTime(duration)}</span></div>
    </div>

    <div class="control-buttons-row">
      <button type="button" class="primary play-btn" onclick={togglePlay}>
        {isPlaying ? 'PAUSE' : 'PLAY AUDIO'}
      </button>

      <div class="speed-selector" role="group" aria-label="Playback speed">
        {#each ['1.0x', '1.25x', '1.5x'] as const as r}
          <button type="button" class={playbackRate === r ? 'outline active' : 'text'} onclick={() => setSpeed(r)}>{r}</button>
        {/each}
      </div>
    </div>
  </div>
</div>

<style>
  .audio-player-panel { display: flex; flex-direction: column; gap: var(--spacing-12); padding: var(--spacing-14); background: rgba(16, 9, 4, 0.4); border: 1px solid var(--color-cork-border); border-radius: var(--radius-cards); }
  .player-header { display: flex; justify-content: space-between; align-items: center; }
  .player-format { font-size: 10px; color: var(--color-driftwood); letter-spacing: 0.08em; }
  .player-controls { display: flex; flex-direction: column; gap: var(--spacing-10); }
  .progress-wrap { display: flex; flex-direction: column; gap: 4px; }
  .audio-scrubber { width: 100%; accent-color: var(--color-ember-accent); cursor: pointer; }
  .time-stamps { display: flex; justify-content: space-between; font-size: 10px; color: var(--color-driftwood); font-family: var(--font-mono, monospace); }
  .control-buttons-row { display: flex; justify-content: space-between; align-items: center; gap: 10px; }
  .play-btn { min-width: 110px; padding: 8px 16px; font-size: 11px; }
  .speed-selector { display: flex; gap: 4px; }
  .speed-selector button { font-size: 10px; padding: 4px 8px; border-radius: 4px; }
  .speed-selector button.active { background: var(--color-ember-accent); color: var(--color-warm-cream); border-color: var(--color-ember-accent); }
</style>
