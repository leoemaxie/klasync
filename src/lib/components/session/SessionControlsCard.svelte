<script lang="ts">
  import ButtonSpinner from '$lib/components/shared/ButtonSpinner.svelte';
  import { Play, Pause, Circle, Mic, Power } from '@lucide/svelte';

  let {
    isPaused = false,
    isRecording = false,
    isTogglingPause = false,
    isTogglingRec = false,
    actionError = '',
    apiNotice = '',
    onPauseToggle,
    onRecordingToggle,
    onEndSession
  }: {
    isPaused?: boolean;
    isRecording?: boolean;
    isTogglingPause?: boolean;
    isTogglingRec?: boolean;
    actionError?: string;
    apiNotice?: string;
    onPauseToggle: () => void;
    onRecordingToggle: () => void;
    onEndSession: () => void;
  } = $props();
</script>

<div class="controls-card">
  <p class="section-label">LECTURER BROADCAST CONTROLS</p>
  <div class="action-buttons-grid">
    <button type="button" class="control-btn pause-toggle" class:is-active-paused={isPaused} aria-pressed={isPaused} onclick={onPauseToggle} disabled={isTogglingPause}>
      {#if isTogglingPause}<ButtonSpinner label="Updating status..." />{:else if isPaused}<Play size={16} aria-hidden="true" /> Resume Lecture Room{:else}<Pause size={16} aria-hidden="true" /> Pause Lecture Room{/if}
    </button>
    <button type="button" class="control-btn rec-toggle" class:is-active-rec={isRecording} aria-pressed={isRecording} onclick={onRecordingToggle} disabled={isTogglingRec}>
      {#if isTogglingRec}<ButtonSpinner label="Updating recording..." />{:else if isRecording}<Circle size={13} fill="currentColor" aria-hidden="true" /> Stop Recording{:else}<Mic size={16} aria-hidden="true" /> Start Audio Recording{/if}
    </button>
  </div>
</div>

{#if actionError || apiNotice}
  <div class="error-notice" role="alert">{actionError || apiNotice}</div>
{/if}

<div class="end-session-row">
  <button type="button" class="end-session-btn" onclick={onEndSession}>
    <Power size={15} aria-hidden="true" /> End Live Lecture Session
  </button>
</div>

<style>
  .controls-card { background: rgba(16, 9, 4, 0.6); border: 1px solid var(--color-cork-border); border-radius: 8px; padding: var(--spacing-18); }
  .section-label { font-size: 11px; letter-spacing: 0.1em; text-transform: uppercase; color: var(--color-driftwood); font-weight: 600; margin: 0 0 6px 0; }
  .action-buttons-grid { display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-14); margin-top: 8px; }
  .control-btn { background: rgba(56, 36, 22, 0.6); border: 1px solid var(--color-cork-border); border-radius: 6px; color: var(--color-warm-cream); padding: 12px 18px; font-size: 12px; font-weight: 500; cursor: pointer; display: inline-flex; align-items: center; justify-content: center; gap: 8px; }
  .control-btn:hover:not(:disabled) { border-color: var(--color-warm-cream); background: var(--color-bark-brown); }
  .control-btn.is-active-paused { background: rgba(220, 80, 0, 0.15); border-color: var(--color-ember-accent); }
  .control-btn.is-active-rec { background: rgba(239, 68, 68, 0.15); border-color: #ef4444; color: #f87171; }
  .error-notice { font-size: 12px; color: var(--color-ember-accent); background: rgba(220, 80, 0, 0.1); border: 1px solid var(--color-cork-border); padding: 10px 14px; border-radius: 6px; }
  .end-session-row { display: flex; justify-content: flex-end; margin-top: 4px; }
  .end-session-btn { background: transparent; border: 1px solid rgba(220, 80, 0, 0.4); color: var(--color-ember-accent); padding: 9px 18px; border-radius: 6px; font-size: 11px; font-weight: 600; text-transform: uppercase; cursor: pointer; display: inline-flex; align-items: center; gap: 6px; }
  .end-session-btn:hover { background: rgba(220, 80, 0, 0.15); border-color: var(--color-ember-accent); }
  @media (max-width: 600px) { .action-buttons-grid { grid-template-columns: 1fr; } }
</style>
