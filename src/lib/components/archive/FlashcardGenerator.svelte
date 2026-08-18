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
      placeholder="Enter topic (e.g. Memory, Concurrency)..."
      class="generator-input"
      aria-label="Generate flashcards by topic"
      onkeydown={(e) => e.key === 'Enter' && handleGenerate()}
    />
    <button
      type="button"
      class="primary gen-btn"
      onclick={() => handleGenerate()}
      disabled={isGenerating || !topic.trim()}
    >
      <Sparkles size={14} />
      <span>{isGenerating ? 'Generating...' : 'Generate'}</span>
    </button>
  </div>

  <div class="quick-topics">
    <span class="quick-label">QUICK TOPICS:</span>
    {#each quickTopics as tag}
      <button
        type="button"
        class="quick-pill"
        disabled={isGenerating}
        onclick={() => handleGenerate(tag)}
      >
        <Plus size={12} />
        <span>{tag}</span>
      </button>
    {/each}
  </div>
</div>

<style>
  .generator-box {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-8);
    margin-bottom: var(--spacing-6);
  }
  .generator-input-row {
    display: flex;
    align-items: stretch;
    gap: var(--spacing-8);
  }
  .generator-input {
    flex: 1;
    font-size: 13px;
    padding: 0 14px;
    height: 38px;
    min-height: 38px;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    color: var(--color-warm-cream);
    border-radius: var(--radius-controls, 4px);
    box-sizing: border-box;
    transition: border-color 0.15s ease, background 0.15s ease;
  }
  .generator-input::placeholder {
    color: rgba(255, 237, 215, 0.4);
  }
  .generator-input:focus {
    border-color: var(--color-warm-cream);
    background: rgba(255, 237, 215, 0.03);
    outline: none;
  }
  .gen-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    font-size: 11px;
    font-weight: 700;
    height: 38px;
    min-height: 38px;
    padding: 0 16px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    white-space: nowrap;
    border-radius: var(--radius-controls, 4px);
    box-sizing: border-box;
  }
  .quick-topics {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .quick-label {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--color-warm-cream-dim);
    font-family: var(--font-mono, monospace);
  }
  .quick-pill {
    font-size: 11px;
    font-weight: 500;
    padding: 4px 12px;
    min-height: 26px;
    border-radius: 999px;
    background: rgba(255, 237, 215, 0.05);
    border: 1px solid rgba(255, 237, 215, 0.12);
    color: var(--color-warm-cream);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    transition:
      border-color 0.15s ease,
      background 0.15s ease,
      color 0.15s ease;
  }
  .quick-pill:hover:not(:disabled) {
    border-color: var(--color-warm-cream);
    background: rgba(255, 237, 215, 0.1);
    color: var(--color-warm-cream);
  }
  @media (max-width: 480px) {
    .generator-input-row {
      flex-direction: column;
    }
    .gen-btn {
      width: 100%;
    }
  }
</style>
