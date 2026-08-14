<script lang="ts">
  import { Search, Copy, Check, Type } from '@lucide/svelte';

  let { transcript = '' }: { transcript?: string } = $props();

  let fontSize = $state(16);
  let filterTerm = $state('');
  let isAllCopied = $state(false);
  let copiedLineIndex = $state<number | null>(null);

  const lines = $derived(
    transcript
      ? transcript
          .split('\n')
          .filter((line) =>
            line.toLowerCase().includes(filterTerm.toLowerCase())
          )
      : []
  );

  function handleCopyAll() {
    if (!transcript) return;
    navigator.clipboard.writeText(transcript).then(() => {
      isAllCopied = true;
      setTimeout(() => (isAllCopied = false), 2000);
    });
  }

  function handleCopyLine(line: string, index: number) {
    navigator.clipboard.writeText(line).then(() => {
      copiedLineIndex = index;
      setTimeout(() => (copiedLineIndex = null), 1500);
    });
  }
</script>

<div class="panel transcript-viewer">
  <div class="transcript-header">
    <div>
      <p class="eyebrow">LECTURE TRANSCRIPT READER</p>
      <span class="transcript-stats">{lines.length} {lines.length === 1 ? 'line' : 'lines'} recorded</span>
    </div>

    <div class="header-controls">
      {#if transcript}
        <button type="button" class="copy-all-btn outline" onclick={handleCopyAll} title="Copy full transcript">
          {#if isAllCopied}
            <Check size={13} color="var(--color-warm-cream)" />
            <span>Copied Full</span>
          {:else}
            <Copy size={13} />
            <span>Copy Transcript</span>
          {/if}
        </button>

        <div class="font-controls" role="group" aria-label="Font size controls">
          <Type size={13} color="var(--color-driftwood)" />
          <button
            type="button"
            class="outline icon-btn"
            aria-label="Decrease font size"
            onclick={() => (fontSize = Math.max(13, fontSize - 1))}>A-</button
          >
          <span class="font-size-label">{fontSize}px</span>
          <button
            type="button"
            class="outline icon-btn"
            aria-label="Increase font size"
            onclick={() => (fontSize = Math.min(28, fontSize + 1))}>A+</button
          >
        </div>
      {/if}
    </div>
  </div>

  {#if transcript}
    <div class="search-bar-wrap">
      <Search size={14} class="search-icon" />
      <input
        id="transcript-filter"
        type="search"
        bind:value={filterTerm}
        placeholder="Filter keywords, equations, or timestamps..."
        class="transcript-search"
      />
      {#if filterTerm}
        <button type="button" class="clear-search-btn text" onclick={() => (filterTerm = '')}>
          Clear
        </button>
      {/if}
    </div>

    <div
      class="transcript-content"
      style="font-size: {fontSize}px;"
      role="log"
      aria-label="Lecture transcript content"
    >
      {#if lines.length > 0}
        <div class="transcript-lines-list">
          {#each lines as line, i (i)}
            <div class="transcript-row">
              <span class="line-idx">{(i + 1).toString().padStart(2, '0')}</span>
              <p class="transcript-line">{line}</p>
              <button
                type="button"
                class="line-copy-btn text"
                onclick={() => handleCopyLine(line, i)}
                title="Copy line"
              >
                {#if copiedLineIndex === i}
                  <Check size={11} color="var(--color-warm-cream)" />
                {:else}
                  <Copy size={11} />
                {/if}
              </button>
            </div>
          {/each}
        </div>
      {:else}
        <div class="empty-filter-state">
          <p class="hint">No transcript lines match "{filterTerm}".</p>
        </div>
      {/if}
    </div>
  {:else}
    <div class="empty-transcript-box">
      <p class="empty-text">No transcript text recorded for this session.</p>
    </div>
  {/if}
</div>

<style>
  .transcript-viewer {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14);
    padding: var(--spacing-18);
    background: rgba(16, 9, 4, 0.4);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
  }
  .transcript-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px 16px;
    border-bottom: 1px solid var(--color-cork-border);
    padding-bottom: var(--spacing-12);
  }
  .transcript-stats {
    font-size: 11px;
    color: var(--color-driftwood);
  }
  .header-controls {
    display: flex;
    align-items: center;
    gap: var(--spacing-10);
    flex-wrap: wrap;
  }
  .copy-all-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    padding: 5px 10px;
    text-transform: uppercase;
  }
  .font-controls {
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    padding: 2px 6px;
    border-radius: 4px;
  }
  .font-controls .icon-btn {
    padding: 2px 6px;
    font-size: 10px;
    min-width: 24px;
    border: none;
    background: transparent;
  }
  .font-controls .icon-btn:hover {
    background: rgba(255, 237, 215, 0.1);
  }
  .font-size-label {
    font-size: 11px;
    color: var(--color-warm-cream);
    min-width: 32px;
    text-align: center;
    font-family: var(--font-mono, monospace);
  }
  .search-bar-wrap {
    position: relative;
    display: flex;
    align-items: center;
  }
  :global(.search-bar-wrap .search-icon) {
    position: absolute;
    left: 10px;
    color: var(--color-driftwood);
  }
  .transcript-search {
    width: 100%;
    padding-left: 32px !important;
    font-size: 13px;
    margin: 0;
  }
  .clear-search-btn {
    position: absolute;
    right: 10px;
    font-size: 11px;
    color: var(--color-ember-accent);
    text-transform: uppercase;
  }
  .transcript-content {
    min-height: 280px;
    max-height: 480px;
    overflow-y: auto;
    padding-right: 8px;
    line-height: 1.7;
    color: var(--color-warm-cream);
    background: rgba(16, 9, 4, 0.5);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    padding: var(--spacing-12);
  }
  .transcript-lines-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .transcript-row {
    display: flex;
    align-items: flex-start;
    gap: var(--spacing-10);
    padding: 4px 6px;
    border-radius: 4px;
    transition: background 0.15s ease;
  }
  .transcript-row:hover {
    background: rgba(255, 237, 215, 0.05);
  }
  .line-idx {
    font-family: var(--font-mono, monospace);
    font-size: 10px;
    color: var(--color-driftwood);
    opacity: 0.7;
    margin-top: 4px;
    user-select: none;
    min-width: 18px;
  }
  .transcript-line {
    margin: 0;
    flex: 1;
  }
  .line-copy-btn {
    opacity: 0;
    padding: 2px;
    color: var(--color-driftwood);
    transition: opacity 0.15s ease;
  }
  .transcript-row:hover .line-copy-btn {
    opacity: 1;
  }
  .line-copy-btn:hover {
    color: var(--color-warm-cream);
  }
  .empty-filter-state,
  .empty-transcript-box {
    padding: var(--spacing-24);
    text-align: center;
  }
  .empty-text {
    font-size: 13px;
    color: var(--color-driftwood);
  }
</style>
