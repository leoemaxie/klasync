<script lang="ts">
  import { onMount } from 'svelte';
  import { Network, Sparkles } from '@lucide/svelte';
  import {
    fetchSessionVisualizations,
    generateSessionVisualization,
    runStudyJob,
    type SessionVisualization,
  } from '$lib/api/aiStudy';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import { useStudio } from '$lib/studio/lectureStudio.svelte';
  import ConceptExplorer from './ConceptExplorer.svelte';

  const studio = useStudio();

  let items = $state<SessionVisualization[]>([]);
  let status = $state<'loading' | 'ready'>('loading');
  let generating = $state(false);
  let error = $state('');
  let topic = $state('');
  let selectedIndex = $state(0);

  const current = $derived(items[selectedIndex]);

  async function load() {
    try {
      items = await fetchSessionVisualizations(studio.sessionKey);
      selectedIndex = 0;
      error = '';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Unable to load the map.';
    }
  }

  async function generate() {
    if (generating) return;
    generating = true;
    error = '';
    try {
      const focus = topic.trim() || undefined;
      await runStudyJob(() =>
        generateSessionVisualization(studio.sessionKey, focus)
      );
      await load();
      topic = '';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not build the map.';
    } finally {
      generating = false;
    }
  }

  onMount(async () => {
    try {
      await load();
    } finally {
      status = 'ready';
    }
    // Same as before: build a map the first time a lecture is opened.
    if (!items.length && !error) void generate();
  });
</script>

<div class="map-pane">
  <form
    class="focus-row"
    onsubmit={(e) => {
      e.preventDefault();
      void generate();
    }}
  >
    <input
      type="text"
      bind:value={topic}
      placeholder="Optional focus, e.g. the Calvin cycle"
      aria-label="Focus the concept map on a topic"
      maxlength="200"
    />
    <Button variant="accent" type="submit" disabled={generating}>
      <Sparkles size={14} />
      {generating ? 'Building…' : items.length ? 'New map' : 'Build map'}
    </Button>
  </form>

  {#if error}
    <p class="err" role="alert">{error}</p>
  {/if}

  {#if status === 'loading' || (generating && !items.length)}
    <SkeletonCard lines={4} label="Building concept map…" />
  {:else if current}
    {#if items.length > 1}
      <div class="history" role="group" aria-label="Earlier maps">
        {#each items as item, i (item.id)}
          <Button
            size="sm"
            variant="outline"
            pressed={i === selectedIndex}
            onclick={() => (selectedIndex = i)}>{item.title}</Button
          >
        {/each}
      </div>
    {/if}
    <p class="meta">
      {current.source === 'ai' ? 'AI generated' : 'Keyword based'} ·
      {current.spec.nodes?.length ?? 0} concepts
    </p>
    <ConceptExplorer visualization={current} />
  {:else}
    <EmptyState
      icon={Network}
      title="No concept map yet"
      description="A map of the lecture’s key ideas is built from its transcript."
    />
  {/if}
</div>

<style>
  .map-pane {
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: 100%;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-width: thin;
    scrollbar-color: var(--color-cork-border) transparent;
    padding-right: 2px;
  }
  .map-pane > :global(*) {
    flex-shrink: 0;
  }
  .focus-row {
    display: flex;
    gap: 8px;
    flex: none;
  }
  .focus-row input {
    flex: 1;
    min-width: 0;
    height: var(--control-h);
    min-height: var(--control-h);
    margin: 0;
    padding: 0 16px;
    border: 1px solid var(--border-default);
    border-radius: 999px;
    background: var(--surface-inset);
    color: var(--text-primary);
    font-family: var(--font-body);
    font-size: 13.5px;
    letter-spacing: 0;
    text-transform: none;
  }
  .focus-row input::placeholder {
    color: var(--text-tertiary);
    font-style: normal;
  }
  .focus-row input:focus {
    outline: none;
    border-color: var(--border-strong);
    box-shadow: var(--focus-ring);
  }
  .history {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .meta {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .err {
    margin: 0;
    font-size: 13px;
    color: var(--color-error);
  }
</style>
