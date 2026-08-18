<script lang="ts">
  import { renderMarkdown } from '$lib/utils/markdown';
  import { FileText } from '@lucide/svelte';

  let { content = '' }: { content?: string } = $props();

  const renderedHtml = $derived(renderMarkdown(content));
</script>

<div class="notes-preview-canvas" role="region" aria-label="Notes preview">
  {#if content.trim()}
    <!-- eslint-disable-next-line svelte/no-at-html-tags -->
    <article class="markdown-body">
      {@html renderedHtml}
    </article>
  {:else}
    <div class="empty-preview">
      <FileText size={20} color="var(--color-driftwood)" />
      <p class="empty-text">No notes to preview. Switch to Edit mode to write.</p>
    </div>
  {/if}
</div>

<style>
  .notes-preview-canvas {
    min-height: 220px;
    max-height: 520px;
    overflow-y: auto;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    padding: var(--spacing-14);
    color: var(--color-warm-cream);
    line-height: 1.65;
  }
  .empty-preview {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 180px;
    gap: 8px;
    text-align: center;
  }
  .empty-text {
    font-size: 12px;
    color: var(--color-driftwood);
    margin: 0;
  }
  :global(.markdown-body .md-h1) {
    font-family: var(--font-display);
    font-size: 19px;
    color: var(--color-warm-cream);
    margin: 0 0 10px 0;
    padding-bottom: 6px;
    border-bottom: 1px solid var(--color-cork-border);
  }
  :global(.markdown-body .md-h2) {
    font-family: var(--font-display);
    font-size: 16px;
    color: var(--color-ember-accent);
    margin: 14px 0 6px 0;
  }
  :global(.markdown-body .md-h3) {
    font-size: 14px;
    font-weight: 600;
    color: var(--color-warm-cream-dim);
    margin: 10px 0 4px 0;
  }
  :global(.markdown-body .md-p) {
    font-size: 13px;
    color: var(--color-warm-cream-dim);
    margin: 0 0 10px 0;
  }
  :global(.markdown-body .md-li, .markdown-body .md-li-num) {
    font-size: 13px;
    margin: 3px 0 3px 18px;
    color: var(--color-warm-cream);
  }
  :global(.markdown-body .md-bold) {
    color: var(--color-warm-cream);
    font-weight: 700;
  }
  :global(.markdown-body .md-italic) {
    color: var(--color-driftwood);
    font-style: italic;
  }
  :global(.markdown-body .md-inline-code) {
    font-family: var(--font-mono, monospace);
    font-size: 11.5px;
    background: rgba(255, 237, 215, 0.08);
    border: 1px solid var(--color-cork-border);
    padding: 1px 5px;
    border-radius: 3px;
    color: var(--color-ember-accent);
  }
  :global(.markdown-body .md-code-block) {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    background: rgba(8, 4, 2, 0.8);
    border: 1px solid var(--color-cork-border);
    border-radius: 4px;
    padding: 10px;
    overflow-x: auto;
    margin: 8px 0;
  }
  :global(.markdown-body .md-quote) {
    border-left: 2px solid var(--color-ember-accent);
    padding-left: 10px;
    margin: 8px 0;
    color: var(--color-driftwood);
    font-style: italic;
  }
</style>
