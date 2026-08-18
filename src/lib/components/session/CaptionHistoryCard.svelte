<script lang="ts">
  import { MessageSquare } from '@lucide/svelte';

  let {
    captions = [],
  }: {
    captions: string[];
  } = $props();
</script>

<div class="panel history-card">
  <div class="card-header">
    <p class="eyebrow">HISTORY ({captions.length})</p>
  </div>

  {#if captions.length === 0}
    <div class="empty-history">
      <MessageSquare size={24} aria-hidden="true" class="empty-icon" />
      <p class="empty-title">No captions sent yet</p>
      <p class="hint">
        Sent captions will appear here.
      </p>
    </div>
  {:else}
    <div class="caption-list" role="log" aria-label="Published captions">
      {#each captions as caption, index (index)}
        <div class="history-item">
          <span class="chunk-badge">#{index + 1}</span>
          <p class="caption-text">{caption}</p>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .history-card {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14);
  }
  .eyebrow {
    font-size: 10px;
    letter-spacing: 0.1em;
    color: var(--color-warm-cream-dim);
    font-weight: 700;
    margin: 0;
  }
  .empty-history {
    text-align: center;
    padding: 24px;
    border: 1px dashed var(--color-cork-border);
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }
  :global(.empty-icon) {
    color: var(--color-driftwood);
  }
  .empty-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--color-warm-cream);
    margin: 0;
  }
  .caption-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 280px;
    overflow-y: auto;
  }
  .history-item {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    background: rgba(10, 5, 2, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
  }
  .chunk-badge {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--color-warm-cream);
    background: rgba(74, 183, 114, 0.12);
    border: 1px solid rgba(74, 183, 114, 0.25);
    padding: 2px 6px;
    border-radius: 4px;
    flex-shrink: 0;
    text-transform: uppercase;
  }
  .caption-text {
    font-size: 13px;
    color: var(--color-warm-cream);
    margin: 0;
    line-height: 1.4;
  }
</style>
