<script lang="ts">
  import { Search, Copy, Check, Type } from '@lucide/svelte';

  let { transcript = '' }: { transcript?: string } = $props();
  let fontSize = $state(15);
  let filterTerm = $state('');
  let isAllCopied = $state(false);

  const lines = $derived(
    transcript ? transcript.split('\n').filter((l) => l.toLowerCase().includes(filterTerm.toLowerCase())) : []
  );

  function handleCopyAll() {
    if (!transcript) return;
    navigator.clipboard.writeText(transcript).then(() => {
      isAllCopied = true;
      setTimeout(() => (isAllCopied = false), 2000);
    });
  }
</script>

<div class="panel transcript-viewer">
  <div class="transcript-header">
    <div>
      <p class="eyebrow">LECTURE TRANSCRIPT</p>
      <span class="transcript-stats">{lines.length} lines</span>
    </div>
    <div class="header-controls">
      {#if transcript}
        <button type="button" class="outline copy-btn" onclick={handleCopyAll}>
          {#if isAllCopied}<Check size={12} /> Copied{:else}<Copy size={12} /> Copy{/if}
        </button>
        <div class="font-controls" role="group" aria-label="Font size">
          <Type size={12} color="var(--color-driftwood)" />
          <button type="button" class="text" onclick={() => (fontSize = Math.max(12, fontSize - 1))}>A-</button>
          <span class="font-size-label">{fontSize}px</span>
          <button type="button" class="text" onclick={() => (fontSize = Math.min(24, fontSize + 1))}>A+</button>
        </div>
      {/if}
    </div>
  </div>

  {#if transcript}
    <div class="search-bar-wrap">
      <Search size={13} class="search-icon" />
      <input type="search" bind:value={filterTerm} placeholder="Filter transcript keywords..." class="transcript-search" />
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
        <p class="hint">No transcript lines match "{filterTerm}".</p>
      {/if}
    </div>
  {:else}
    <p class="hint">No transcript text recorded for this session.</p>
  {/if}
</div>

<style>
  .transcript-viewer { display: flex; flex-direction: column; gap: var(--spacing-8); padding: var(--spacing-12); background: rgba(16, 9, 4, 0.4); border: 1px solid var(--color-cork-border); border-radius: var(--radius-cards); }
  .transcript-header, .header-controls { display: flex; justify-content: space-between; align-items: center; gap: 8px; flex-wrap: wrap; }
  .transcript-stats { font-size: 11px; color: var(--color-driftwood); }
  .copy-btn { font-size: 10px; padding: 4px 8px; text-transform: uppercase; }
  .font-controls { display: flex; align-items: center; gap: 4px; background: rgba(16, 9, 4, 0.6); border: 1px solid var(--color-cork-border); padding: 2px 6px; border-radius: 4px; }
  .font-controls button { font-size: 10px; padding: 2px 4px; border: none; }
  .font-size-label { font-size: 10px; color: var(--color-warm-cream); font-family: var(--font-mono, monospace); }
  .search-bar-wrap { position: relative; display: flex; align-items: center; }
  :global(.search-bar-wrap .search-icon) { position: absolute; left: 10px; color: var(--color-driftwood); }
  .transcript-search { width: 100%; padding-left: 28px !important; font-size: 12px; margin: 0; }
  .transcript-content { min-height: 200px; max-height: 480px; overflow-y: auto; line-height: 1.65; background: rgba(16, 9, 4, 0.5); border: 1px solid var(--color-cork-border); border-radius: 6px; padding: var(--spacing-10); }
  .transcript-row { display: flex; gap: 8px; padding: 3px 0; word-break: break-word; }
  .line-idx { font-family: var(--font-mono, monospace); font-size: 10px; color: var(--color-driftwood); opacity: 0.7; min-width: 18px; flex-shrink: 0; }
  .transcript-line { margin: 0; flex: 1; word-break: break-word; }
  @media (max-width: 640px) {
    .transcript-viewer { padding: 10px; }
    .transcript-content { max-height: 55vh; }
  }
</style>
