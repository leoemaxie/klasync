<script lang="ts">
  let {
    captions = [],
    captionIndex = 0,
    dyslexicFont = false,
    fontSize = '18px',
    lineHeight = 1.6,
    onNextCaption,
  }: {
    captions: string[];
    captionIndex: number;
    dyslexicFont?: boolean;
    fontSize?: string;
    lineHeight?: number;
    onNextCaption: () => void;
  } = $props();
</script>

<article
  class="panel caption-card"
  class:dyslexic-mode={dyslexicFont}
  style="font-size: {fontSize}; line-height: {lineHeight};"
  aria-live="polite"
>
  <div class="caption-card-header">
    <p class="eyebrow-bright">
      <span class="eyebrow-accent">●</span> REAL-TIME CAPTION STREAM
    </p>
    {#if captions.length > 0}
      <span class="caption-counter"
        >Chunk {captionIndex + 1} of {captions.length}</span
      >
    {/if}
  </div>

  <div class="caption-content">
    {#if captions.length === 0}
      <div class="caption-empty-box">
        <p class="empty-prompt">WAITING FOR LIVE CAPTIONS...</p>
        <span class="empty-hint"
          >Captions will appear automatically as the lecturer speaks.</span
        >
      </div>
    {:else}
      <p class="caption-text">
        {captions[captionIndex] ?? 'WAITING FOR LECTURER SPEECH...'}
      </p>
    {/if}
  </div>

  <div class="caption-actions">
    <button
      type="button"
      class="outline caption-next-btn"
      onclick={onNextCaption}
      disabled={captions.length <= 1}
    >
      Next Caption Chunk →
    </button>
  </div>
</article>

<style>
  .caption-card {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-12);
    min-height: 220px;
    justify-content: space-between;
    height: 100%;
  }
  .eyebrow-bright {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.12em;
    color: var(--color-warm-cream-dim);
    text-transform: uppercase;
  }
  .caption-card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .caption-counter {
    font-size: 10px;
    color: var(--color-warm-cream-dim);
    text-transform: uppercase;
    font-family: var(--font-mono, monospace);
  }
  .caption-content {
    flex: 1;
    display: flex;
    align-items: center;
    padding: 8px 0;
  }
  .caption-empty-box {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .empty-prompt {
    font-size: 15px;
    font-weight: 700;
    color: var(--color-warm-cream);
  }
  .empty-hint {
    font-size: 11px;
    color: var(--color-driftwood);
  }
  .caption-text {
    font-family: var(--font-body);
    font-weight: 500;
    color: var(--color-warm-cream);
    line-height: inherit;
  }
  .dyslexic-mode .caption-text {
    font-family: 'OpenDyslexic', 'Comic Sans MS', sans-serif;
    letter-spacing: 0.05em;
  }
  .caption-next-btn {
    align-self: flex-start;
    font-size: 11px;
  }
  .caption-next-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }
  @media (max-width: 640px) {
    .caption-next-btn {
      width: 100%;
    }
  }
</style>
