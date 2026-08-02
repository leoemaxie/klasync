<script lang="ts">
  import { Pause, Mic } from "@lucide/svelte";

  let {
    title = "",
    isPaused = false,
    isRecording = false
  }: {
    title: string;
    isPaused?: boolean;
    isRecording?: boolean;
  } = $props();
</script>

<div class="session-top-card">
  <div class="header-main">
    <p class="eyebrow">LIVE SESSION DASHBOARD</p>
    <h2 class="session-title">{title}</h2>
  </div>

  <div class="live-status-pills">
    {#if isPaused}
      <span class="status-pill paused"><Pause size={12} /> ROOM PAUSED</span>
    {:else}
      <span class="status-pill broadcasting"><span class="live-dot"></span> LIVE</span>
    {/if}

    {#if isRecording}
      <span class="status-pill recording"><span class="rec-dot"></span> RECORDING ACTIVE</span>
    {:else}
      <span class="status-pill idle"><Mic size={12} /> MIC STANDBY</span>
    {/if}
  </div>
</div>

<style>
  .session-top-card { background: rgba(16, 9, 4, 0.6); border: 1px solid var(--color-cork-border); border-radius: 8px; padding: var(--spacing-16) var(--spacing-20); display: flex; justify-content: space-between; align-items: center; gap: var(--spacing-14); flex-wrap: wrap; }
  .session-title { font-size: 24px; font-weight: 500; color: var(--color-warm-cream); padding: 8px 0; margin: 4px 0 0 0; letter-spacing: -0.01em; font-family: var(--font-display); }
  .live-status-pills { display: flex; align-items: center; gap: var(--spacing-8); flex-wrap: wrap; }
  .status-pill { font-size: 10px; font-weight: 600; letter-spacing: 0.08em; padding: 6px 12px; border-radius: 20px; display: inline-flex; align-items: center; gap: 6px; text-transform: uppercase; }
  .status-pill.broadcasting { background: rgba(74, 183, 114, 0.12); color: #4ab772; border: 1px solid rgba(74, 183, 114, 0.3); }
  .live-dot { width: 6px; height: 6px; border-radius: 50%; background: #4ab772; box-shadow: 0 0 8px #4ab772; animation: blink 1.5s infinite; }
  .status-pill.paused { background: rgba(220, 80, 0, 0.12); color: var(--color-ember-accent); border: 1px solid rgba(220, 80, 0, 0.3); }
  .status-pill.recording { background: rgba(239, 68, 68, 0.15); color: #f87171; border: 1px solid rgba(239, 68, 68, 0.3); }
  .rec-dot { width: 6px; height: 6px; border-radius: 50%; background: #ef4444; box-shadow: 0 0 8px #ef4444; animation: blink 1s infinite; }
  .status-pill.idle { background: rgba(56, 36, 22, 0.6); color: var(--color-driftwood); border: 1px solid var(--color-cork-border); }
  @keyframes blink { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }
</style>
