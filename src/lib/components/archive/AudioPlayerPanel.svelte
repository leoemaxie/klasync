<script lang="ts">
  import { getAudioChunkUrl } from '$lib/api';

  let { sessionCode = '' }: { sessionCode?: string } = $props();

  let audioElement: HTMLAudioElement | null = $state(null);
  let isPlaying = $state(false);
  let currentTime = $state(0);
  let duration = $state(0);
  let playbackRate = $state<'1.0x' | '1.25x' | '1.5x'>('1.0x');

  const chunkUrl = $derived(
    sessionCode ? getAudioChunkUrl(sessionCode, 1) : ''
  );

  const numericPlaybackRate = $derived(
    playbackRate === '1.25x' ? 1.25 : playbackRate === '1.5x' ? 1.5 : 1.0
  );

  $effect(() => {
    if (audioElement) {
      audioElement.playbackRate = numericPlaybackRate;
    }
  });

  function togglePlay() {
    if (!audioElement) return;
    if (audioElement.paused) {
      void audioElement.play().catch(() => {});
      isPlaying = true;
    } else {
      audioElement.pause();
      isPlaying = false;
    }
  }

  function formatTime(secs: number): string {
    if (!secs || isNaN(secs)) return '00:00';
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  }
</script>

<div class="panel audio-player-panel">
  <div class="player-header">
    <p class="eyebrow">LECTURE AUDIO REPLAY STREAM</p>
    <span class="player-format">48kHz PCM STEREOPHONIC</span>
  </div>

  <audio
    bind:this={audioElement}
    src={chunkUrl}
    preload="metadata"
    bind:currentTime
    bind:duration
    onplay={() => (isPlaying = true)}
    onpause={() => (isPlaying = false)}
    onended={() => (isPlaying = false)}
  ></audio>

  <div class="player-controls">
    <div class="progress-wrap">
      <label for="playback-scrubber" class="sr-only">Playback position</label>
      <input
        id="playback-scrubber"
        type="range"
        min="0"
        max={duration || 100}
        step="0.1"
        bind:value={currentTime}
        class="audio-scrubber"
        aria-valuetext="{Math.round(currentTime)} seconds played"
      />
      <div class="time-stamps">
        <span>{formatTime(currentTime)}</span>
        <span>{formatTime(duration)}</span>
      </div>
    </div>

    <div class="control-buttons-row">
      <button
        type="button"
        class="primary play-btn"
        onclick={togglePlay}
        aria-label={isPlaying ? 'Pause audio' : 'Play audio'}
      >
        {isPlaying ? 'PAUSE' : 'PLAY AUDIO'}
      </button>

      <div class="speed-selector" role="group" aria-label="Playback speed">
        <button
          type="button"
          class={playbackRate === '1.0x' ? 'outline active' : 'text'}
          aria-pressed={playbackRate === '1.0x'}
          onclick={() => (playbackRate = '1.0x')}>1.0x</button
        >
        <button
          type="button"
          class={playbackRate === '1.25x' ? 'outline active' : 'text'}
          aria-pressed={playbackRate === '1.25x'}
          onclick={() => (playbackRate = '1.25x')}>1.25x</button
        >
        <button
          type="button"
          class={playbackRate === '1.5x' ? 'outline active' : 'text'}
          aria-pressed={playbackRate === '1.5x'}
          onclick={() => (playbackRate = '1.5x')}>1.5x</button
        >
      </div>
    </div>
  </div>
</div>

<style>
  .audio-player-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-16);
    padding: var(--spacing-18);
  }
  .player-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 12px;
  }
  .player-format {
    font-size: 11px;
    letter-spacing: 0.1em;
    color: var(--color-driftwood);
  }
  .player-controls {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-16);
  }
  .progress-wrap {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .audio-scrubber {
    width: 100%;
    accent-color: var(--color-ember-accent);
    cursor: pointer;
  }
  .time-stamps {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--color-driftwood);
  }
  .control-buttons-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--spacing-14);
    flex-wrap: wrap;
  }
  .play-btn {
    min-width: 120px;
  }
  .speed-selector {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .speed-selector button {
    font-size: 10px;
    padding: 5px 10px;
    border-radius: var(--radius-buttons-outlined);
  }
  .speed-selector button.active {
    background: var(--color-ember-accent);
    color: var(--color-warm-cream);
    border-color: var(--color-ember-accent);
  }
  @media (max-width: 480px) {
    .audio-player-panel {
      padding: var(--spacing-14);
    }
    .control-buttons-row {
      flex-direction: column;
      align-items: stretch;
      gap: var(--spacing-12);
    }
    .play-btn {
      width: 100%;
      text-align: center;
    }
    .speed-selector {
      width: 100%;
      justify-content: space-between;
    }
    .speed-selector button {
      flex: 1;
      text-align: center;
    }
  }
</style>
