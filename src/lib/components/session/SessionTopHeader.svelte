<script lang="ts">
  import { Pause, Mic } from '@lucide/svelte';

  let {
    title = '',
    isPaused = false,
    isRecording = false,
    audioLevel = 0,
  }: {
    title: string;
    isPaused?: boolean;
    isRecording?: boolean;
    audioLevel?: number;
  } = $props();
</script>

<div class="session-top-card">
  <div class="header-main">
    <h2 class="session-title">{title}</h2>
  </div>

  <div class="live-status-pills">
    {#if isPaused}
      <span class="status-pill paused"
        ><Pause size={12} aria-hidden="true" /> PAUSED</span
      >
    {:else}
      <span class="status-pill broadcasting"
        ><span class="live-dot" aria-hidden="true"></span> LIVE</span
      >
    {/if}

    {#if isRecording}
      <span class="status-pill recording active-rec">
        <span class="reverberating-mic-wrap">
          <span class="reverb-ring"></span>
          <span class="reverb-ring delay"></span>
          <Mic size={13} aria-hidden="true" class="reverb-mic-icon" />
        </span>
        RECORDING
        {#if audioLevel > 0}
          <span
            class="level-indicator-bar"
            style="width: {Math.max(10, Math.min(60, audioLevel))}px"
          ></span>
        {/if}
      </span>
    {:else}
      <span class="status-pill idle"
        ><Mic size={12} aria-hidden="true" /> MIC OFF</span
      >
    {/if}
  </div>
</div>

<style>
  .session-top-card {
    background: rgba(16, 9, 4, 0.5);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
    padding: var(--spacing-16) var(--spacing-20);
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--spacing-14);
    flex-wrap: wrap;
  }
  .header-main {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .session-title {
    font-size: 24px;
    font-weight: 500;
    color: var(--color-warm-cream);
    padding: 4px 0;
    margin: 0;
    letter-spacing: -0.01em;
    font-family: var(--font-display);
    line-height: 1.2;
  }
  .live-status-pills {
    display: flex;
    align-items: center;
    gap: var(--spacing-8);
    flex-wrap: wrap;
  }
  .status-pill {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    padding: 6px 12px;
    border-radius: 20px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    text-transform: uppercase;
  }
  .status-pill.broadcasting {
    background: rgba(74, 183, 114, 0.12);
    color: var(--color-warm-cream);
    border: 1px solid rgba(74, 183, 114, 0.3);
  }
  .live-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #4ab772;
    box-shadow: 0 0 8px #4ab772;
    animation: blink 1.5s infinite;
  }
  .status-pill.paused {
    background: rgba(220, 80, 0, 0.12);
    color: var(--color-ember-accent);
    border: 1px solid rgba(220, 80, 0, 0.3);
  }
  .status-pill.recording.active-rec {
    background: rgba(239, 68, 68, 0.2);
    color: var(--color-warm-cream);
    border: 1px solid #ef4444;
    box-shadow: 0 0 16px rgba(239, 68, 68, 0.35);
  }

  .reverberating-mic-wrap {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
  }
  .reverb-ring {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    border-radius: 50%;
    border: 2px solid rgba(248, 113, 113, 0.8);
    animation: mic-reverb-pulse 1.6s cubic-bezier(0, 0.2, 0.8, 1) infinite;
    pointer-events: none;
  }
  .reverb-ring.delay {
    animation-delay: 0.8s;
  }
  @keyframes mic-reverb-pulse {
    0% {
      transform: scale(0.8);
      opacity: 1;
    }
    100% {
      transform: scale(2.4);
      opacity: 0;
    }
  }
  :global(.reverb-mic-icon) {
    color: #f87171;
    z-index: 1;
    animation: mic-vibrate 0.3s ease-in-out infinite alternate;
  }
  @keyframes mic-vibrate {
    0% {
      transform: scale(1);
    }
    100% {
      transform: scale(1.15);
    }
  }

  .level-indicator-bar {
    height: 4px;
    background: #ef4444;
    border-radius: 2px;
    transition: width 0.1s ease;
    box-shadow: 0 0 6px #ef4444;
  }
  .status-pill.idle {
    background: rgba(56, 36, 22, 0.6);
    color: var(--color-driftwood);
    border: 1px solid var(--color-cork-border);
  }
  @keyframes blink {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.3;
    }
  }
</style>
