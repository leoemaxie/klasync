<script lang="ts">
  import { Sparkles, Plus } from '@lucide/svelte';
  import { triggerHaptic } from '$lib/native/haptics';

  let {
    onGenerate,
  }: {
    onGenerate: (topic: string) => void | Promise<void>;
  } = $props();

  let topic = $state('');
  let isGenerating = $state(false);
  const quickTopics = ['Key Formulas', 'Core Definitions', 'Exam Review'];

  async function handleGenerate(val?: string) {
    const query = (val || topic).trim();
    if (!query || isGenerating) return;
    triggerHaptic('medium');
    isGenerating = true;
    try {
      await onGenerate(query);
      topic = '';
    } finally {
      isGenerating = false;
    }
  }
</script>

<div class="generator-box">
  <div class="generator-input-row">
    <input
      type="text"
      bind:value={topic}
      placeholder="Generate topic flashcards (e.g. Concurrency, Memory)..."
      class="generator-input"
      onkeydown={(e) => e.key === 'Enter' && handleGenerate()}
    />
    <button
      type="button"
      class="primary gen-btn"
      onclick={() => handleGenerate()}
      disabled={isGenerating || !topic.trim()}
    >
      <Sparkles size={13} />
      <span>{isGenerating ? 'Generating...' : 'Generate'}</span>
    </button>
  </div>

  <div class="quick-topics">
    <span class="quick-label">QUICK:</span>
    {#each quickTopics as tag}
      <button
        type="button"
        class="quick-pill"
        disabled={isGenerating}
        onclick={() => handleGenerate(tag)}
      >
        <Plus size={10} /> {tag}
      </button>
    {/each}
  </div>
</div>

<style>
  .generator-box { display: flex; flex-direction: column; gap: var(--spacing-6); margin-bottom: var(--spacing-4); }
  .generator-input-row { display: flex; gap: var(--spacing-6); }
  .generator-input { flex: 1; font-size: 12px; padding: 6px 10px; background: rgba(16, 9, 4, 0.6); border: 1px solid var(--color-cork-border); color: var(--color-warm-cream); border-radius: 4px; }
  .generator-input:focus { border-color: var(--color-warm-cream); outline: none; }
  .gen-btn { display: inline-flex; align-items: center; gap: 4px; font-size: 10px; padding: 6px 12px; text-transform: uppercase; white-space: nowrap; }
  .quick-topics { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
  .quick-label { font-size: 9px; letter-spacing: 0.1em; color: var(--color-driftwood); font-family: var(--font-mono, monospace); }
  .quick-pill { font-size: 10px; padding: 2px 8px; border-radius: 999px; background: transparent; border: 1px dashed var(--color-cork-border); color: var(--color-driftwood); cursor: pointer; display: inline-flex; align-items: center; gap: 3px; transition: border-color 0.2s, color 0.2s; }
  .quick-pill:hover:not(:disabled) { border-color: var(--color-warm-cream); color: var(--color-warm-cream); }
</style>
