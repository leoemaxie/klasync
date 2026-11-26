<script lang="ts">
  import { onMount } from 'svelte';
  import {
    ListChecks,
    NotebookPen,
    Copy,
    Check,
    Sparkles,
  } from '@lucide/svelte';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import { fetchArchiveSummaries, type RollingSummary } from '$lib/api/aiStudy';
  import { useStudio } from '$lib/studio/lectureStudio.svelte';

  const studio = useStudio();

  let items = $state<RollingSummary[]>([]);
  let status = $state<'loading' | 'ready' | 'error'>('loading');
  let error = $state('');
  let copied = $state(false);

  const ordered = $derived([...items].sort((a, b) => a.sequence - b.sequence));

  async function load() {
    status = 'loading';
    error = '';
    try {
      items = await fetchArchiveSummaries(studio.sessionKey);
      status = 'ready';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Unable to load the summary.';
      status = 'error';
    }
  }
  onMount(() => void load());

  const asText = (s: RollingSummary) =>
    [s.summary, ...(s.key_points ?? []).map((p) => `- ${p}`)].join('\n');

  async function copyAll() {
    try {
      await navigator.clipboard.writeText(ordered.map(asText).join('\n\n'));
      copied = true;
      studio.announce('Summary copied');
      setTimeout(() => (copied = false), 1800);
    } catch {
      studio.announce('Copy is not available here');
    }
  }

  function addToNotes(s: RollingSummary) {
    const points = s.key_points?.length ? s.key_points : [s.summary];
    // One bullet per key point, with no timestamp (summaries are not tied to
    // a single moment of the recording).
    for (const p of points) studio.addNote(p, null);
  }
</script>

<div class="summary">
  {#if status === 'loading'}
    <SkeletonCard lines={4} label="Loading summary…" />
  {:else if status === 'error'}
    <EmptyState
      tone="error"
      icon={ListChecks}
      title="Couldn’t load the summary"
      description={error}
    >
      <Button variant="outline" onclick={() => void load()}>Try again</Button>
    </EmptyState>
  {:else if ordered.length === 0}
    <EmptyState
      icon={Sparkles}
      title="No summary yet"
      description="A summary is captured while the lecture runs. This lecture doesn’t have one."
    />
  {:else}
    <header class="head">
      <p class="meta">
        {ordered.length}
        {ordered.length === 1 ? 'part' : 'parts'}
      </p>
      <Button variant="outline" size="sm" onclick={copyAll}>
        {#if copied}<Check size={14} /> Copied{:else}<Copy size={14} /> Copy all{/if}
      </Button>
    </header>
    <ol class="parts">
      {#each ordered as item, i (item.id)}
        <li class="part">
          <div class="part-head">
            <span class="num">{i + 1}</span>
            {#if item.source === 'heuristic'}<span class="src"
                >Keyword based</span
              >{/if}
            <Button
              size="sm"
              class="to-notes"
              title="Add the key points to your notes"
              onclick={() => addToNotes(item)}
            >
              <NotebookPen size={14} /> Add to notes
            </Button>
          </div>
          <p class="body">{item.summary}</p>
          {#if item.key_points?.length}
            <ul class="points">
              {#each item.key_points as point}<li>{point}</li>{/each}
            </ul>
          {/if}
        </li>
      {/each}
    </ol>
  {/if}
</div>

<style>
  .summary {
    display: flex;
    flex-direction: column;
    gap: 12px;
    height: 100%;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-width: thin;
    scrollbar-color: var(--color-cork-border) transparent;
    padding-right: 2px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .meta {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .parts {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .part {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 16px;
    background: var(--surface-inset);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
  }
  .part-head {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .part-head :global(.to-notes) {
    margin-left: auto;
  }
  .num {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--surface-3);
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
  }
  .src {
    padding: 2px 8px;
    border: 1px solid var(--border-default);
    border-radius: 999px;
    font-size: 11px;
    color: var(--text-secondary);
  }
  .body {
    margin: 0;
    font-size: 15px;
    line-height: 1.65;
    color: var(--text-primary);
    text-wrap: pretty;
  }
  .points {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .points li {
    position: relative;
    padding-left: 18px;
    font-size: 14px;
    line-height: 1.55;
    color: var(--text-secondary);
  }
  .points li::before {
    content: '';
    position: absolute;
    left: 3px;
    top: 0.62em;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-ember-accent);
  }
</style>
