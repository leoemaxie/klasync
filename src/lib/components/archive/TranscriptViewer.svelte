<script lang="ts">
  import { Search, Copy, Check, Type } from '@lucide/svelte';

  let { transcript = '' }: { transcript?: string } = $props();
  let fontSize = $state(15);
  let filterTerm = $state('');
  let isAllCopied = $state(false);

  const lines = $derived(
    transcript
      ? transcript
          .split('\n')
          .filter((l) => l.toLowerCase().includes(filterTerm.toLowerCase()))
      : []
  );

  function handleCopyAll() {
    if (!transcript) return;
    navigator.clipboard.writeText(transcript).then(() => {
      isAllCopied = true;
      setTimeout(() => (isAllCopied = false), 2000);
    });
  }
</script>

<div class="transcript-viewer">
  <div class="transcript-header">
    <div class="transcript-title-group">
      <h2 class="section-title">Lecture Transcript</h2>
      <span class="lines-pill"
        >{lines.length} {lines.length === 1 ? 'Line' : 'Lines'}</span
      >
    </div>
    <div class="header-controls">
      {#if transcript}
        <button type="button" class="outline copy-btn" onclick={handleCopyAll}>
          {#if isAllCopied}<Check size={13} /> <span>Copied</span>{:else}<Copy
              size={13}
            /> <span>Copy All</span>{/if}
        </button>
        <div class="font-controls" role="group" aria-label="Font size">
          <Type size={13} color="var(--color-warm-cream-dim)" />
          <button
            type="button"
            class="font-btn"
            title="Decrease font size"
            aria-label="Decrease font size"
            onclick={() => (fontSize = Math.max(12, fontSize - 1))}>A-</button
          >
          <span class="font-size-label">{fontSize}px</span>
          <button
            type="button"
            class="font-btn"
            title="Increase font size"
            aria-label="Increase font size"
            onclick={() => (fontSize = Math.min(24, fontSize + 1))}>A+</button
          >
        </div>
      {/if}
    </div>
  </div>

  {#if transcript}
    <div class="search-bar-wrap">
      <Search size={14} class="search-ico" />
      <input
        type="search"
        bind:value={filterTerm}
        placeholder="Search within transcript..."
        class="transcript-search"
        aria-label="Search transcript content"
      />
    </div>
    <div class="transcript-content" style="font-size: {fontSize}px;" role="log">
      {#if lines.length > 0}
        {#each lines as line, i (i)}
          <div class="transcript-row">
            <span class="line-idx">{(i + 1).toString().padStart(2, '0')}</span>
            <p class="transcript-line">{line}</p>
          </div>
        {/each}
      {:else}
        <div class="empty-search-state">
          <p class="empty-hint">
            No lines matching <strong>"{filterTerm}"</strong>.
          </p>
        </div>
      {/if}
    </div>
  {:else}
    <div class="empty-transcript-state">
      <p class="empty-hint">
        No transcript available for this lecture session.
      </p>
    </div>
  {/if}
</div>

<style>
  .transcript-viewer {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-10);
    width: 100%;
  }
  .transcript-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--spacing-8);
    flex-wrap: wrap;
  }
  .transcript-title-group {
    display: flex;
    align-items: center;
    gap: var(--spacing-8);
  }
  .section-title {
    font-size: 15px;
    font-weight: 700;
    color: var(--color-warm-cream);
    margin: 0;
    letter-spacing: -0.01em;
  }
  .lines-pill {
    font-size: 11px;
    font-weight: 600;
    color: var(--color-warm-cream-dim);
    background: rgba(255, 237, 215, 0.05);
    border: 1px solid var(--color-cork-border);
    padding: 2px 8px;
    border-radius: 999px;
  }
  .header-controls {
    display: flex;
    align-items: center;
    gap: var(--spacing-8);
    flex-wrap: wrap;
  }
  .copy-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 0 12px;
    height: 32px;
    min-height: 32px;
    border-radius: var(--radius-controls, 4px);
    box-sizing: border-box;
  }
  .font-controls {
    display: flex;
    align-items: center;
    gap: 6px;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    padding: 0 8px;
    height: 32px;
    border-radius: var(--radius-controls, 4px);
    box-sizing: border-box;
  }
  .font-btn {
    background: transparent;
    border: none;
    color: var(--color-warm-cream-dim);
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
    padding: 4px 6px;
    border-radius: 3px;
    transition:
      color 0.15s ease,
      background 0.15s ease;
  }
  .font-btn:hover {
    color: var(--color-warm-cream);
    background: rgba(255, 237, 215, 0.1);
  }
  .font-size-label {
    font-size: 11px;
    color: var(--color-warm-cream);
    font-family: var(--font-mono, monospace);
    font-weight: 600;
    min-width: 28px;
    text-align: center;
  }
  .search-bar-wrap {
    position: relative;
    display: flex;
    align-items: center;
  }
  :global(.search-bar-wrap .search-ico) {
    position: absolute;
    left: 12px;
    color: var(--color-warm-cream-dim);
    pointer-events: none;
  }
  .transcript-search {
    width: 100%;
    height: 38px;
    min-height: 38px;
    padding-left: 34px !important;
    padding-right: 14px;
    font-size: 13px;
    margin: 0;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    color: var(--color-warm-cream);
    border-radius: var(--radius-controls, 4px);
    box-sizing: border-box;
    transition: border-color 0.15s ease;
  }
  .transcript-search::placeholder {
    color: rgba(255, 237, 215, 0.45);
  }
  .transcript-search:focus {
    border-color: var(--color-warm-cream);
    outline: none;
  }
  .transcript-content {
    min-height: 260px;
    max-height: 520px;
    overflow-y: auto;
    line-height: 1.7;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards, 8px);
    padding: var(--spacing-14);
    box-sizing: border-box;
  }
  .transcript-row {
    display: flex;
    gap: 12px;
    padding: 6px 0;
    border-bottom: 1px solid rgba(255, 237, 215, 0.04);
  }
  .transcript-row:last-child {
    border-bottom: none;
  }
  .line-idx {
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    color: var(--color-warm-cream-dim);
    min-width: 22px;
    flex-shrink: 0;
    padding-top: 2px;
    user-select: none;
    font-weight: 600;
  }
  .transcript-line {
    margin: 0;
    flex: 1;
    color: var(--color-warm-cream);
    word-break: break-word;
  }
  .empty-search-state,
  .empty-transcript-state {
    padding: var(--spacing-20) var(--spacing-12);
    text-align: center;
  }
  .empty-hint {
    font-size: 13px;
    color: var(--color-warm-cream-dim);
    margin: 0;
  }
  .empty-hint strong {
    color: var(--color-warm-cream);
  }
  @media (max-width: 640px) {
    .transcript-content {
      max-height: 60vh;
      padding: var(--spacing-10);
    }
  }
</style>
