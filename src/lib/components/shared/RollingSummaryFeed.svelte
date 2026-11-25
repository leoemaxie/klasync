<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchArchiveSummaries, type RollingSummary } from '$lib/api/aiStudy';
  import { fetchLiveSummaries } from '$lib/api/live';
  import { ListChecks } from '@lucide/svelte';

  let {
    sessionCode = '',
    sessionId = '',
    live = false,
    title = 'Rolling summary',
  }: {
    /** Live mode: public feed keyed by the join code, polled. */
    sessionCode?: string;
    /** Archive mode: student-authenticated feed keyed by session UUID. */
    sessionId?: string;
    live?: boolean;
    title?: string;
  } = $props();

  let summaries = $state<RollingSummary[]>([]);
  let isLoading = $state(true);
  let errorMessage = $state('');

  const newestFirst = $derived([...summaries].reverse());

  async function load() {
    try {
      summaries = live
        ? await fetchLiveSummaries(sessionCode)
        : await fetchArchiveSummaries(sessionId);
      errorMessage = '';
    } catch (error) {
      errorMessage =
        error instanceof Error ? error.message : 'Unable to load summaries.';
    } finally {
      isLoading = false;
    }
  }

  onMount(() => {
    void load();
    if (!live) return;
    const timer = setInterval(load, 12_000);
    return () => clearInterval(timer);
  });

  const clock = (iso: string) =>
    new Date(iso).toLocaleTimeString([], {
      hour: '2-digit',
      minute: '2-digit',
    });
</script>

<section class="panel summary-feed" aria-label={title}>
  <div class="feed-head">
    <div>
      <p class="eyebrow">{live ? 'LIVE NOTES' : 'LECTURE NOTES'}</p>
      <h2>{title}</h2>
    </div>
    <ListChecks size={18} color="var(--color-driftwood)" />
  </div>

  {#if errorMessage}
    <p class="feed-error" role="alert">{errorMessage}</p>
  {:else if isLoading}
    <p class="feed-hint">Loading summaries…</p>
  {:else if newestFirst.length === 0}
    <p class="feed-hint">
      {live
        ? 'A summary appears here every minute or so once the lecture is under way.'
        : 'No summary was captured for this lecture.'}
    </p>
  {:else}
    <ol class="entries" aria-live={live ? 'polite' : 'off'}>
      {#each newestFirst as item (item.id)}
        <li class="entry">
          <span class="stamp">{clock(item.created_at)}</span>
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
</section>

<style>
  .summary-feed {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-10);
  }
  .feed-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
  }
  .feed-head h2 {
    margin: 2px 0 0;
    font-size: 16px;
  }
  .feed-hint {
    margin: 0;
    font-size: 12px;
    color: var(--color-warm-cream-dim);
  }
  .feed-error {
    margin: 0;
    font-size: 12px;
    color: var(--color-error);
  }
  .entries {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-10);
    max-height: 360px;
    overflow-y: auto;
  }
  .entry {
    border-left: 2px solid var(--color-ember-accent);
    padding-left: var(--spacing-10);
  }
  .stamp {
    font-family: var(--font-mono, monospace);
    font-size: 10px;
    color: var(--color-warm-cream-dim);
  }
  .body {
    margin: 2px 0 4px;
    font-size: 13px;
    line-height: 1.5;
    color: var(--color-warm-cream);
  }
  .points {
    margin: 0;
    padding-left: 18px;
    font-size: 12px;
    color: var(--color-warm-cream-dim);
  }
</style>
