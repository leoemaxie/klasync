<script lang="ts">
  import { Sparkles } from '@lucide/svelte';
  import { triggerHaptic } from '$lib/native/haptics';

  let {
    onGenerate,
  }: {
    onGenerate: (topic: string) => void;
  } = $props();

  let topic = $state('');
  let isGenerating = $state(false);

  function handleSubmit() {
    if (!topic.trim()) return;
    triggerHaptic('medium');
    isGenerating = true;
    try {
      onGenerate(topic.trim());
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
      placeholder="Generate topic flashcards (e.g. Concurrency)..."
      class="generator-input"
      onkeydown={(e) => e.key === 'Enter' && handleSubmit()}
    />
    <button
      type="button"
      class="primary gen-btn"
      onclick={handleSubmit}
      disabled={isGenerating || !topic.trim()}
    >
      <Sparkles size={12} />
      <span>{isGenerating ? 'Generating...' : 'Generate'}</span>
    </button>
  </div>
</div>

<style>
  .generator-box { margin-bottom: var(--spacing-6); }
  .generator-input-row { display: flex; gap: var(--spacing-6); }
  .generator-input { flex: 1; font-size: 12px; padding: 6px 10px; }
  .gen-btn { display: inline-flex; align-items: center; gap: 4px; font-size: 10px; padding: 6px 12px; text-transform: uppercase; white-space: nowrap; }
</style>
