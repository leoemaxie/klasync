<script lang="ts">
  import { onMount } from 'svelte';
  import {
    fetchSessionVisualizations,
    generateSessionVisualization,
    runStudyJob,
    type SessionVisualization,
  } from '$lib/api/aiStudy';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import ConceptMap from './ConceptMap.svelte';
  import { Network, Sparkles } from '@lucide/svelte';

  let { sessionId = '' }: { sessionId?: string } = $props();

  let items = $state<SessionVisualization[]>([]);
  let isLoading = $state(true);
  let isGenerating = $state(false);
  let errorMessage = $state('');
  let topic = $state('');
  let selectedIndex = $state(0);

  const current = $derived(items[selectedIndex]);

  async function load() {
    try {
      items = await fetchSessionVisualizations(sessionId);
      selectedIndex = 0;
      errorMessage = '';
    } catch (error) {
      errorMessage =
        error instanceof Error ? error.message : 'Unable to load visuals.';
    }
  }

  async function generate() {
    isGenerating = true;
    errorMessage = '';
    try {
      const focus = topic.trim() || undefined;
      await runStudyJob(() => generateSessionVisualization(sessionId, focus));
      await load();
      topic = '';
    } catch (error) {
      errorMessage =
        error instanceof Error ? error.message : 'Could not build the visual.';
    } finally {
      isGenerating = false;
    }
  }

  onMount(async () => {
    try {
      await load();
    } finally {
      isLoading = false;
    }
    if (!items.length && !errorMessage) void generate();
  });
</script>

<div class="visuals-panel">
  <div class="visuals-header">
    <h2 class="section-title">Concept Map</h2>
    {#if current}
      <span class="source-pill"
        >{current.source === 'ai' ? 'AI generated' : 'Keyword based'}</span
      >
    {/if}
  </div>

  <div class="focus-row">
    <input
      type="text"
      bind:value={topic}
      placeholder="Optional focus (e.g. Memory management)…"
      aria-label="Focus the concept map on a topic"
      onkeydown={(e) => e.key === 'Enter' && !isGenerating && generate()}
    />
    <button
      type="button"
      class="primary"
      onclick={generate}
      disabled={isGenerating}
    >
      <Sparkles size={14} />
      <span
        >{isGenerating
          ? 'Building…'
          : items.length
            ? 'New map'
            : 'Build map'}</span
      >
    </button>
  </div>

  {#if errorMessage}
    <p class="error" role="alert">{errorMessage}</p>
  {/if}

  {#if isLoading || (isGenerating && !items.length)}
    <SkeletonCard lines={4} label="Building concept map…" />
  {:else if current}
    {#if items.length > 1}
      <div class="history" role="tablist" aria-label="Earlier maps">
        {#each items as item, i}
          <button
            type="button"
            role="tab"
            class="history-btn"
            class:active={i === selectedIndex}
            aria-selected={i === selectedIndex}
            onclick={() => (selectedIndex = i)}>{item.title}</button
          >
        {/each}
      </div>
    {/if}
    <ConceptMap visualization={current} />
    <p class="hint">
      Hover or focus a concept to highlight how it connects to the others.
    </p>
  {:else}
    <div class="empty">
      <Network size={24} color="var(--color-driftwood)" />
      <p class="empty-title">No concept map yet</p>
      <p class="hint">
        A map of the lecture's key ideas is built from its transcript.
      </p>
    </div>
  {/if}
</div>

<style>
  .visuals-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-10);
    width: 100%;
  }
  .visuals-header {
    display: flex;
    align-items: center;
    gap: var(--spacing-8);
  }
  .section-title {
    font-size: 15px;
    font-weight: 700;
    color: var(--color-warm-cream);
    margin: 0;
  }
  .source-pill {
    font-size: 11px;
    font-weight: 600;
    color: var(--color-warm-cream-dim);
    background: rgba(255, 237, 215, 0.05);
    border: 1px solid var(--color-cork-border);
    padding: 2px 8px;
    border-radius: 999px;
  }
  .focus-row {
    display: flex;
    gap: var(--spacing-8);
  }
  .focus-row input {
    flex: 1;
    min-width: 0;
    height: 38px;
    padding: 0 14px;
    font-size: 13px;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    color: var(--color-warm-cream);
    border-radius: var(--radius-controls, 4px);
    box-sizing: border-box;
  }
  .focus-row button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 38px;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    white-space: nowrap;
  }
  .error {
    margin: 0;
    font-size: 12px;
    color: var(--color-error);
  }
  .history {
    display: flex;
    gap: 6px;
    overflow-x: auto;
  }
  .history-btn {
    font-size: 11px;
    padding: 4px 10px;
    border-radius: 999px;
    background: transparent;
    border: 1px solid var(--color-cork-border);
    color: var(--color-warm-cream-dim);
    cursor: pointer;
    white-space: nowrap;
  }
  .history-btn.active {
    color: var(--color-warm-cream);
    border-color: var(--color-ember-accent);
  }
  .hint {
    margin: 0;
    font-size: 11px;
    color: var(--color-warm-cream-dim);
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: var(--spacing-24) 0;
    text-align: center;
  }
  .empty-title {
    margin: 0;
    font-weight: 700;
    color: var(--color-warm-cream);
  }
  @media (max-width: 520px) {
    .focus-row {
      flex-direction: column;
    }
  }
</style>
