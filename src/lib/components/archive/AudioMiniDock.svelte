<script lang="ts">
  import { Volume2, Play, Pause } from '@lucide/svelte';
  import { triggerHaptic } from '$lib/native/haptics';

  let {
    sessionTitle = '',
    isPlaying = false,
    onTogglePlay,
    onExpand,
  }: {
    sessionTitle?: string;
    isPlaying?: boolean;
    onTogglePlay?: () => void;
    onExpand?: () => void;
  } = $props();

  function handlePlayTap(e: MouseEvent) {
    e.stopPropagation();
    triggerHaptic('light');
    onTogglePlay?.();
  }

  function handleDockTap() {
    triggerHaptic('selection');
    onExpand?.();
  }
</script>

<div
  class="audio-mini-dock"
  onclick={handleDockTap}
  role="button"
  tabindex="0"
  onkeydown={(e) => e.key === 'Enter' && handleDockTap()}
>
  <div class="dock-left">
    <div class="dock-icon-wrap" class:playing={isPlaying}>
      <Volume2 size={15} color="var(--color-warm-cream)" />
    </div>
    <div class="dock-info">
      <span class="dock-label">LECTURE AUDIO REPLAY</span>
      <span class="dock-title">{sessionTitle || 'Active Lecture Audio'}</span>
    </div>
  </div>

  <button
    type="button"
    class="dock-play-btn"
    onclick={handlePlayTap}
    aria-label={isPlaying ? 'Pause' : 'Play'}
  >
    {#if isPlaying}<Pause size={15} />{:else}<Play size={15} />{/if}
  </button>
</div>

<style>
  .audio-mini-dock {
    display: none;
    position: fixed;
    bottom: calc(62px + env(safe-area-inset-bottom, 0px));
    left: 12px;
    right: 12px;
    background: rgba(28, 16, 8, 0.95);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border: 1px solid var(--color-ember-accent);
    border-radius: 8px;
    padding: 8px 12px;
    z-index: 85;
    justify-content: space-between;
    align-items: center;
    cursor: pointer;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.6);
  }
  .dock-left {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .dock-icon-wrap {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--color-bark-brown);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .dock-icon-wrap.playing {
    background: var(--color-ember-accent);
  }
  .dock-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .dock-label {
    font-size: 8px;
    letter-spacing: 0.1em;
    color: var(--color-ember-accent);
    font-weight: 700;
  }
  .dock-title {
    font-size: 11px;
    color: var(--color-warm-cream);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dock-play-btn {
    width: 34px;
    height: 34px;
    border-radius: 50%;
    background: var(--color-warm-cream);
    color: var(--color-walnut-shadow);
    border: none;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  @media (max-width: 800px) {
    .audio-mini-dock {
      display: flex;
    }
  }
</style>
