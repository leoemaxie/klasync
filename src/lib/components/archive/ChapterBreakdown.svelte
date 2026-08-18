<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchSessionChapters, type SessionChapter } from '$lib/api';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';

  let { sessionId = 'sess-demo-312' }: { sessionId?: string } = $props();

  let chapters = $state<SessionChapter[]>([]);
  let isLoading = $state(true);

  onMount(async () => {
    try {
      chapters = await fetchSessionChapters(sessionId);
    } finally {
      isLoading = false;
    }
  });

  function formatTime(sec: number): string {
    return `${Math.floor(sec / 60)
      .toString()
      .padStart(2, '0')}:${(sec % 60).toString().padStart(2, '0')}`;
  }
</script>

<div class="chapter-breakdown-panel">
  <div class="chapter-header">
    <p class="eyebrow">LECTURE TOPIC CHAPTERS</p>
    <span class="chapter-count"
      >{chapters.length} {chapters.length === 1 ? 'Chapter' : 'Chapters'}</span
    >
  </div>

  {#if isLoading}
    <SkeletonCard lines={2} label="Loading chapters..." />
  {:else if chapters.length}
    <div class="chapters-list" role="region" aria-label="Lecture chapters">
      {#each chapters as ch}
        <div class="chapter-card">
          <span class="time-badge"
            >{formatTime(ch.start_timestamp_sec)} – {formatTime(
              ch.end_timestamp_sec
            )}</span
          >
          <div class="chapter-info">
            <h3 class="chapter-title">{ch.title}</h3>
            <p class="chapter-desc">{ch.summary}</p>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <p class="hint">No chapters generated yet for this lecture session.</p>
  {/if}
</div>

<style>
  .chapter-breakdown-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-10);
    padding: var(--spacing-12);
    background: rgba(16, 9, 4, 0.4);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
  }
  .chapter-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .chapter-count {
    font-size: 11px;
    color: var(--color-driftwood);
  }
  .chapters-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-8);
    overscroll-behavior-y: contain;
    max-height: 480px;
    overflow-y: auto;
  }
  .chapter-card {
    display: flex;
    gap: var(--spacing-10);
    padding: var(--spacing-12);
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    align-items: flex-start;
  }
  .time-badge {
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    font-weight: 700;
    color: var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.12);
    border: 1px solid rgba(220, 80, 0, 0.2);
    padding: 3px 7px;
    border-radius: 4px;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .chapter-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }
  .chapter-title {
    font-size: 15px;
    font-weight: 600;
    margin: 0;
    color: var(--color-warm-cream);
    line-height: 1.35;
  }
  .chapter-desc {
    font-size: 13px;
    color: var(--color-warm-cream-dim);
    line-height: 1.55;
    margin: 0;
  }
  @media (max-width: 520px) {
    .chapter-card {
      flex-direction: column;
      gap: 6px;
    }
  }
</style>
