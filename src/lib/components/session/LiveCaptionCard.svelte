<script lang="ts">
  import Skeleton from '$lib/components/shared/Skeleton.svelte';

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
    <p class="eyebrow">
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
      <div class="caption-skeleton">
        <Skeleton
          height="56px"
          label="Waiting for speech-to-text audio stream..."
        />
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
    gap: var(--spacing-14);
    min-height: 200px;
    justify-content: space-between;
  }
  .caption-card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .caption-counter {
    font-size: 10px;
    color: var(--color-driftwood);
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .caption-content {
    flex: 1;
    display: flex;
    align-items: center;
    padding: 8px 0;
  }
  .caption-text {
    font-family: var(--font-body);
    font-weight: 500;
    color: var(--color-warm-cream);
    line-height: inherit;
    transition: all 0.2s ease;
  }
  .dyslexic-mode .caption-text {
    font-family: 'OpenDyslexic', 'Comic Sans MS', sans-serif;
    letter-spacing: 0.05em;
    word-spacing: 0.1em;
  }
  .caption-next-btn {
    align-self: flex-start;
  }
  @media (max-width: 640px) {
    .caption-next-btn {
      width: 100%;
    }
  }
</style>
