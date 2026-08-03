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

  <div class="player-controls-row">
    <button
      class="primary play-btn"
      onclick={() => (isPlaying = !isPlaying)}
      aria-label={isPlaying ? 'Pause audio' : 'Play audio'}
    >
      {isPlaying ? 'PAUSE' : 'PLAY AUDIO'}
    </button>

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

<style>
  .audio-player-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14);
  }
  .player-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .player-format {
    font-size: 11px;
    letter-spacing: 0.1em;
    color: var(--color-driftwood);
  }
  .player-controls-row {
    display: flex;
    align-items: center;
    gap: var(--spacing-18);
  }
  .play-btn {
    min-width: 110px;
  }
  .progress-wrap {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .audio-scrubber {
    width: 100%;
    accent-color: var(--color-ember-accent);
  }
  .time-stamps {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    color: var(--color-driftwood);
  }
  .speed-selector {
    display: flex;
    gap: 6px;
  }
  .speed-selector button {
    font-size: 10px;
    padding: 3px 8px;
  }
</style>
