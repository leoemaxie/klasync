<script lang="ts">
  import { renderMarkdown } from '$lib/utils/markdown';
  import { FileText } from '@lucide/svelte';

  import { parseClock } from '$lib/studio/time';

  let {
    content = '',
    fill = false,
    onSeek,
  }: {
    content?: string;
    /** Fill the parent's height instead of using the standalone max-height. */
    fill?: boolean;
    /** Called with seconds when a [mm:ss] stamp in the notes is pressed. */
    onSeek?: (seconds: number) => void;
  } = $props();
  const renderedHtml = $derived(renderMarkdown(content));

  function onClick(event: MouseEvent) {
    const el = (event.target as HTMLElement).closest<HTMLElement>(
      '[data-seek]'
    );
    if (!el || !onSeek) return;
    const seconds = parseClock(el.dataset.seek ?? '');
    if (seconds != null) onSeek(seconds);
  }
</script>

<div
  class="notes-preview-canvas"
  class:fill
  role="region"
  aria-label="Notes preview"
>
  {#if content.trim()}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
    <!-- eslint-disable-next-line svelte/no-at-html-tags -->
    <article class="markdown-body" onclick={onClick}>
      {@html renderedHtml}
    </article>
  {:else}
    <div class="empty-preview">
      <FileText size={22} color="var(--color-driftwood)" />
      <p class="empty-text">
        No notes to preview. Switch to Edit mode to add study notes.
      </p>
    </div>
  {/if}
</div>

<style>
  .notes-preview-canvas {
    min-height: 280px;
    max-height: 600px;
    overflow-y: auto;
    background: rgba(14, 8, 4, 0.7);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards, 8px);
    padding: var(--spacing-18, 20px);
    color: var(--color-warm-cream);
    box-sizing: border-box;
  }
  .notes-preview-canvas.fill {
    max-height: none;
    height: 100%;
    min-height: 0;
  }
  :global(.markdown-body .md-time) {
    display: inline-flex;
    align-items: center;
    min-width: 0;
    min-height: 0;
    height: 22px;
    padding: 0 7px;
    margin-right: 2px;
    border: 1px solid var(--border-accent);
    border-radius: 999px;
    background: var(--surface-accent);
    color: var(--text-accent);
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0;
    text-transform: none;
    vertical-align: baseline;
  }
  :global(.markdown-body .md-time:hover) {
    background: rgba(220, 80, 0, 0.26);
    color: var(--text-primary);
  }
  .empty-preview {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 220px;
    gap: 8px;
    text-align: center;
  }
  .empty-text {
    font-size: 12.5px;
    color: var(--color-driftwood);
    margin: 0;
  }
  :global(.markdown-body .md-h1) {
    font-family: var(--font-display);
    font-size: 21px;
    font-weight: 700;
    color: var(--color-warm-cream);
    margin: 0 0 12px 0;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--color-cork-border);
    line-height: 1.3;
  }
  :global(.markdown-body .md-h2) {
    font-family: var(--font-display);
    font-size: 16.5px;
    font-weight: 700;
    color: var(--color-ember-accent);
    margin: 18px 0 8px 0;
    letter-spacing: -0.01em;
  }
  :global(.markdown-body .md-h3) {
    font-size: 13.5px;
    font-weight: 700;
    color: var(--color-warm-cream-dim);
    margin: 14px 0 6px 0;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  :global(.markdown-body .md-p) {
    font-size: 13.5px;
    line-height: 1.65;
    color: var(--color-warm-cream);
    margin: 0 0 10px 0;
  }
  :global(.markdown-body .md-ul),
  :global(.markdown-body .md-ol) {
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: 0;
    margin: 0 0 14px 0;
    list-style: none;
  }
  :global(.markdown-body .md-li),
  :global(.markdown-body .md-li-num) {
    display: flex;
    align-items: flex-start;
    gap: 9px;
    font-size: 13.5px;
    line-height: 1.55;
    color: var(--color-warm-cream);
  }
  :global(.markdown-body .md-bullet) {
    width: 5px;
    height: 5px;
    min-width: 5px;
    border-radius: 50%;
    background: var(--color-ember-accent);
    margin-top: 8px;
  }
  :global(.markdown-body .md-num-badge) {
    font-size: 10.5px;
    font-family: var(--font-mono, monospace);
    font-weight: 700;
    color: var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.12);
    border: 1px solid rgba(220, 80, 0, 0.25);
    padding: 1px 5px;
    border-radius: 3px;
    line-height: 1.2;
    margin-top: 2px;
  }
  :global(.markdown-body .md-bold) {
    color: var(--color-warm-cream);
    font-weight: 700;
  }
  :global(.markdown-body .md-italic) {
    color: var(--color-warm-cream-dim);
    font-style: italic;
  }
  :global(.markdown-body .md-inline-code) {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    background: rgba(255, 237, 215, 0.08);
    border: 1px solid var(--color-cork-border);
    padding: 2px 6px;
    border-radius: 4px;
    color: var(--color-ember-accent);
  }
  :global(.markdown-body .md-code-block) {
    background: rgba(8, 4, 2, 0.85);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    padding: 12px;
    margin: 10px 0;
    overflow-x: auto;
    font-family: var(--font-mono, monospace);
    font-size: 12.5px;
    color: var(--color-warm-cream);
  }
  :global(.markdown-body .md-quote) {
    border-left: 3px solid var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.06);
    padding: 8px 14px;
    border-radius: 0 4px 4px 0;
    margin: 10px 0;
    font-style: italic;
    color: var(--color-warm-cream-dim);
  }
</style>
