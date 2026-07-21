<script lang="ts">
  let { transcript = "" }: { transcript?: string } = $props();

  let fontSize = $state(18);
  let filterTerm = $state("");

  const sampleTranscript = $derived(
    transcript ||
      `[00:00] Good morning everyone. Today we are exploring Human-Computer Interaction and feedback loops.
[02:15] Accessibility is not an afterthought; it is fundamental infrastructure.
[05:30] Feedback makes a system responsive. We capture speech signal, convert it to text, and empower learners.`
  );

  const lines = $derived(
    sampleTranscript
      .split("\n")
      .filter((line) => line.toLowerCase().includes(filterTerm.toLowerCase()))
  );
</script>

<div class="panel transcript-viewer">
  <div class="transcript-header">
    <p class="eyebrow">LECTURE TRANSCRIPT READER</p>
    <div class="font-controls">
      <button class="outline" onclick={() => (fontSize = Math.max(14, fontSize - 2))}>A-</button>
      <span class="font-size-label">{fontSize}px</span>
      <button class="outline" onclick={() => (fontSize = Math.min(32, fontSize + 2))}>A+</button>
    </div>
  </div>

  <input bind:value={filterTerm} placeholder="Filter transcript keywords..." class="transcript-search" />

  <div class="transcript-content" style="font-size: {fontSize}px;">
    {#each lines as line}
      <p class="transcript-line">{line}</p>
    {/each}
  </div>
</div>

<style>
  .transcript-viewer { display: flex; flex-direction: column; gap: var(--spacing-14); }
  .transcript-header { display: flex; justify-content: space-between; align-items: center; }
  .font-controls { display: flex; align-items: center; gap: var(--spacing-8); }
  .font-controls button { padding: 4px 10px; font-size: 11px; }
  .font-size-label { font-size: 11px; color: var(--color-driftwood); }
  .transcript-search { margin-bottom: var(--spacing-8); }
  .transcript-content { max-height: 320px; overflow-y: auto; padding-right: 8px; line-height: 1.7; color: var(--color-warm-cream); }
  .transcript-line { margin-bottom: 10px; }
</style>
