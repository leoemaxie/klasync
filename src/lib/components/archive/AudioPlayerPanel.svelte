<script lang="ts">
  import { onDestroy } from 'svelte';
  import { getAudioChunkUrl } from '$lib/api';
  import { requestWakeLock, releaseWakeLock } from '$lib/native/wakelock';
  import { triggerHaptic } from '$lib/native/haptics';
  import { Play, Pause, Volume2, Gauge } from '@lucide/svelte';

  let { sessionCode = '' }: { sessionCode?: string } = $props();

  let audioElement: HTMLAudioElement | null = $state(null);
  let isPlaying = $state(false);
  let currentTime = $state(0);
  let duration = $state(0);
  let playbackRate = $state<'1.0x' | '1.25x' | '1.5x'>('1.0x');

  const chunkUrl = $derived(
    sessionCode ? getAudioChunkUrl(sessionCode, 1) : ''
  );
  const numericRate = $derived(
    playbackRate === '1.25x' ? 1.25 : playbackRate === '1.5x' ? 1.5 : 1.0
  );

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
    return `${Math.floor(secs / 60)
      .toString()
      .padStart(2, '0')}:${Math.floor(secs % 60)
      .toString()
      .padStart(2, '0')}`;
  }

  onDestroy(() => {
    void releaseWakeLock();
  });
</script>

<div class="audio-player-panel">
  <div class="player-header">
    <div class="player-title-group">
      <h2 class="section-title">Lecture Audio Replay</h2>
    </div>
  </div>

  <audio
    bind:this={audioElement}
    src={chunkUrl}
    preload="metadata"
    bind:currentTime
    bind:duration
    onplay={() => {
      isPlaying = true;
      void requestWakeLock();
    }}
    onpause={() => {
      isPlaying = false;
      void releaseWakeLock();
    }}
    onended={() => {
      isPlaying = false;
      void releaseWakeLock();
    }}
  ></audio>

  <div class="player-card">
    <div class="progress-wrap">
      <input
        type="range"
        min="0"
        max={duration || 100}
        step="0.1"
        bind:value={currentTime}
        class="audio-scrubber"
        aria-label="Audio timeline scrubber"
      />
      <div class="time-stamps">
        <span>{formatTime(currentTime)}</span>
        <span>{formatTime(duration)}</span>
      </div>
    </div>

    <div class="control-buttons-row">
      <button type="button" class="primary play-btn" onclick={togglePlay}>
        {#if isPlaying}
          <Pause size={14} />
          <span>Pause</span>
        {:else}
          <Play size={14} />
          <span>Play Audio</span>
        {/if}
      </button>

      <div class="speed-controls" role="group" aria-label="Playback speed">
        <span class="speed-label"><Gauge size={12} /> Speed:</span>
        <div class="speed-buttons">
          {#each ['1.0x', '1.25x', '1.5x'] as const as r}
            <button
              type="button"
              class="speed-btn"
              class:active={playbackRate === r}
              onclick={() => setSpeed(r)}>{r}</button
            >
          {/each}
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .audio-player-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-10);
    width: 100%;
  }
  .player-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .player-title-group {
    display: flex;
    align-items: center;
    gap: var(--spacing-8);
  }
  .section-title {
    font-size: 15px;
    font-weight: 700;
    color: var(--color-warm-cream);
    margin: 0;
    letter-spacing: -0.01em;
  }
  .session-pill {
    font-size: 11px;
    font-weight: 600;
    color: var(--color-warm-cream-dim);
    background: rgba(255, 237, 215, 0.05);
    border: 1px solid var(--color-cork-border);
    padding: 2px 8px;
    border-radius: 999px;
  }
  .player-card {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-12);
    padding: var(--spacing-16);
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards, 8px);
  }
  .progress-wrap {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .audio-scrubber {
    width: 100%;
    accent-color: var(--color-ember-accent);
    cursor: pointer;
    height: 6px;
    background: rgba(255, 237, 215, 0.08);
    border-radius: 999px;
  }
  .time-stamps {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--color-warm-cream-dim);
    font-family: var(--font-mono, monospace);
    font-weight: 600;
  }
  .control-buttons-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--spacing-12);
    flex-wrap: wrap;
    padding-top: var(--spacing-4);
  }
  .play-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-width: 130px;
    height: 38px;
    min-height: 38px;
    padding: 0 18px;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    border-radius: var(--radius-controls, 4px);
    box-sizing: border-box;
  }
  .speed-controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .speed-label {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    font-weight: 600;
    color: var(--color-warm-cream-dim);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .speed-buttons {
    display: flex;
    gap: 4px;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    padding: 2px;
    border-radius: var(--radius-controls, 4px);
  }
  .speed-btn {
    font-size: 11px;
    font-weight: 600;
    padding: 5px 10px;
    height: 28px;
    min-height: 28px;
    border-radius: 3px;
    background: transparent;
    border: none;
    color: var(--color-warm-cream-dim);
    cursor: pointer;
    transition:
      background 0.15s ease,
      color 0.15s ease;
  }
  .speed-btn:hover {
    color: var(--color-warm-cream);
  }
  .speed-btn.active {
    background: var(--color-ember-accent);
    color: var(--color-warm-cream);
  }
  @media (max-width: 520px) {
    .control-buttons-row {
      flex-direction: column;
      align-items: stretch;
    }
    .play-btn {
      width: 100%;
    }
    .speed-controls {
      justify-content: space-between;
      width: 100%;
    }
  }
</style>
