<script lang="ts">
  import ButtonSpinner from '$lib/components/shared/ButtonSpinner.svelte';
  import { Send } from '@lucide/svelte';

  let {
    captionDraft = $bindable(''),
    apiNotice = '',
    isPublishing = false,
    onPublishCaption
  }: {
    captionDraft: string;
    apiNotice?: string;
    isPublishing?: boolean;
    onPublishCaption: () => Promise<void> | void;
  } = $props();

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      onPublishCaption();
    }
  }
</script>

<div class="panel transmitter-card">
  <div class="card-header">
    <div class="header-main">
      <p class="eyebrow">LIVE CAPTION STREAMING</p>
      <h2 class="card-title">Broadcast Caption Chunk</h2>
    </div>
    <span class="live-signal-badge">
      <span class="pulse-dot">●</span> MIC ACTIVE
    </span>
  </div>

  <div class="input-block">
    <div class="textarea-wrap">
      <textarea
        id="caption-input"
        bind:value={captionDraft}
        placeholder="Type or speak a caption chunk for students..."
        rows={3}
        onkeydown={handleKeyDown}
        class="caption-textarea"
      ></textarea>
    </div>
  </div>

  <div class="action-row">
    <button
      type="button"
      class="primary publish-btn"
      onclick={onPublishCaption}
      disabled={!captionDraft.trim() || isPublishing}
    >
      {#if isPublishing}
        <ButtonSpinner label="Broadcasting caption..." /> Publishing...
      {:else}
        <Send size={14} /> Publish Caption Chunk (Enter ↵)
      {/if}
    </button>
    <span class="hint-text">Press Enter to broadcast immediately to all connected students</span>
  </div>

  {#if apiNotice}
    <div class="notice-box">{apiNotice}</div>
  {/if}
</div>

<style>
  .transmitter-card { display: flex; flex-direction: column; gap: var(--spacing-16); }
  .card-header { display: flex; justify-content: space-between; align-items: flex-start; gap: 12px; }
  .eyebrow { font-size: 10px; letter-spacing: 0.1em; color: var(--color-warm-cream-dim); font-weight: 700; margin: 0; }
  .card-title { font-size: 22px; font-weight: 500; color: var(--color-warm-cream); margin: 4px 0 0 0; font-family: var(--font-display); }
  .live-signal-badge { font-size: 9px; font-weight: 700; color: #4ab772; background: rgba(74, 183, 114, 0.12); border: 1px solid rgba(74, 183, 114, 0.3); padding: 4px 10px; border-radius: 4px; display: inline-flex; align-items: center; gap: 4px; }
  .pulse-dot { animation: blink 1s infinite alternate; }
  @keyframes blink { 0% { opacity: 0.3; } 100% { opacity: 1; } }
  .input-block { display: flex; flex-direction: column; gap: 6px; }
  .caption-textarea { width: 100%; padding: 12px 14px; background: rgba(10, 5, 2, 0.8); border: 1px solid var(--color-cork-border); border-radius: 8px; color: var(--color-warm-cream); font-size: 14px; line-height: 1.5; resize: vertical; }
  .caption-textarea:focus { outline: none; border-color: var(--color-warm-cream); }
  .action-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
  .publish-btn { display: inline-flex; align-items: center; gap: 8px; padding: 10px 20px; font-size: 12px; }
  .hint-text { font-size: 11px; color: var(--color-driftwood); }
  .notice-box { font-size: 12px; color: var(--color-ember-accent); background: rgba(220, 80, 0, 0.1); border: 1px solid var(--color-cork-border); padding: 10px 14px; border-radius: 6px; }
</style>
