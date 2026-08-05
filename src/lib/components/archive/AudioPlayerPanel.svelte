<script lang="ts">
  import { getAudioChunkUrl } from '$lib/api';

  let { sessionCode = '' }: { sessionCode?: string } = $props();

  let isPlaying = $state(false);
  let progress = $state(0);
  let playbackRate = $state<'1.0x' | '1.25x' | '1.5x'>('1.0x');

  const chunkUrl = $derived(
    sessionCode ? getAudioChunkUrl(sessionCode, 1) : ''
  );
</script>

<div class="panel audio-player-panel">
  <div class="player-header">
    <p class="eyebrow">LECTURE AUDIO REPLAY STREAM</p>
    <span class="player-format">48kHz PCM STEREOPHONIC</span>
  </div>

  <audio src={chunkUrl} preload="none"></audio>

  <div class="player-controls">
    <div class="progress-wrap">
      <label for="playback-scrubber" class="sr-only">Playback position</label>
      <input
        id="playback-scrubber"
        type="range"
        min="0"
        max="100"
        bind:value={progress}
        class="audio-scrubber"
        aria-valuetext="{progress}% played"
      />
      <div class="time-stamps">
        <span>04:12</span>
        <span>14:30</span>
      </div>
    </div>

    <div class="control-buttons-row">
      <button
        class="primary play-btn"
        onclick={() => (isPlaying = !isPlaying)}
        aria-label={isPlaying ? 'Pause audio' : 'Play audio'}
      >
        {isPlaying ? 'PAUSE' : 'PLAY AUDIO'}
      </button>

      <div class="speed-selector" role="group" aria-label="Playback speed">
        <button
          class={playbackRate === '1.0x' ? 'outline active' : 'text'}
          aria-pressed={playbackRate === '1.0x'}
          onclick={() => (playbackRate = '1.0x')}>1.0x</button
        >
        <button
          class={playbackRate === '1.25x' ? 'outline active' : 'text'}
          aria-pressed={playbackRate === '1.25x'}
          onclick={() => (playbackRate = '1.25x')}>1.25x</button
        >
        <button
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
