<script lang="ts">
  let { transcript = '' }: { transcript?: string } = $props();

  let fontSize = $state(18);
  let filterTerm = $state('');

  const lines = $derived(
    transcript
      ? transcript
          .split('\n')
          .filter((line) =>
            line.toLowerCase().includes(filterTerm.toLowerCase())
          )
      : []
  );
</script>

<div class="panel transcript-viewer">
  <div class="transcript-header">
    <p class="eyebrow">LECTURE TRANSCRIPT READER</p>
    {#if transcript}
      <div class="font-controls">
        <button
          class="outline"
          onclick={() => (fontSize = Math.max(14, fontSize - 2))}>A-</button
        >
        <span class="font-size-label">{fontSize}px</span>
        <button
          class="outline"
          onclick={() => (fontSize = Math.min(32, fontSize + 2))}>A+</button
        >
      </div>
    {/if}
  </div>

  {#if transcript}
    <input
      bind:value={filterTerm}
      placeholder="Filter transcript keywords..."
      class="transcript-search"
    />

    <div class="transcript-content" style="font-size: {fontSize}px;">
      {#if lines.length > 0}
        {#each lines as line}
          <p class="transcript-line">{line}</p>
        {/each}
      {:else}
        <p class="hint">No transcript lines match key "{filterTerm}".</p>
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
  }
  .transcript-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .font-controls {
    display: flex;
    align-items: center;
    gap: var(--spacing-8);
  }
  .font-controls button {
    padding: 4px 10px;
    font-size: 11px;
  }
  .font-size-label {
    font-size: 11px;
    color: var(--color-driftwood);
  }
  .transcript-search {
    margin-bottom: var(--spacing-8);
  }
  .transcript-content {
    max-height: 320px;
    overflow-y: auto;
    padding-right: 8px;
    line-height: 1.7;
    color: var(--color-warm-cream);
  }
  .transcript-line {
    margin-bottom: 10px;
  }
  .empty-transcript-box {
    padding: var(--spacing-18);
    text-align: center;
    border: 1px dashed var(--color-cork-border);
    border-radius: 6px;
  }
  .empty-text {
    font-size: 13px;
    color: var(--color-driftwood);
  }
</style>
