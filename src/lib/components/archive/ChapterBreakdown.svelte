<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchSessionChapters, type SessionChapter } from '$lib/api';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import { Bookmark, Clock } from '@lucide/svelte';

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
    <div class="chapter-title-group">
      <h2 class="section-title">Topic Chapters</h2>
      <span class="chapter-count-pill"
        >{chapters.length} {chapters.length === 1 ? 'Chapter' : 'Chapters'}</span
      >
    </div>
  </div>

  {#if isLoading}
    <SkeletonCard lines={3} label="Loading chapters..." />
  {:else if chapters.length}
    <div class="chapters-list" role="region" aria-label="Lecture chapters">
      {#each chapters as ch, idx (ch.id || idx)}
        <div class="chapter-card">
          <div class="time-col">
            <span class="time-badge">
              <Clock size={12} />
              <span>{formatTime(ch.start_timestamp_sec)} – {formatTime(ch.end_timestamp_sec)}</span>
            </span>
          </div>
          <div class="chapter-info">
            <h3 class="chapter-title">{ch.title}</h3>
            <p class="chapter-desc">{ch.summary}</p>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <div class="empty-chapters-state">
      <div class="empty-icon-wrap">
        <Bookmark size={24} color="var(--color-driftwood)" />
      </div>
      <h3 class="empty-title">No Chapters Generated</h3>
      <p class="empty-desc">
        AI chapters for this lecture have not been indexed yet.
      </p>
    </div>
  {/if}
</div>

<style>
  .chapter-breakdown-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-10);
    width: 100%;
  }
  .chapter-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--spacing-8);
  }
  .chapter-title-group {
    display: flex;
    align-items: center;
    gap: var(--spacing-8);
  }
  .section-title {
    font-size: 15px;
    font-weight: 700;
    color: var(--color-warm-cream);
    margin: 0;
    letter-spacing: -0.01em;
  }
  .chapter-count-pill {
    font-size: 11px;
    font-weight: 600;
    color: var(--color-warm-cream-dim);
    background: rgba(255, 237, 215, 0.05);
    border: 1px solid var(--color-cork-border);
    padding: 2px 8px;
    border-radius: 999px;
  }
  .chapters-list {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-8);
    overscroll-behavior-y: contain;
    max-height: 520px;
    overflow-y: auto;
  }
  .chapter-card {
    display: flex;
    gap: var(--spacing-12);
    padding: var(--spacing-14);
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards, 8px);
    align-items: flex-start;
    transition: border-color 0.15s ease, background 0.15s ease;
  }
  .chapter-card:hover {
    border-color: rgba(255, 237, 215, 0.25);
    background: rgba(255, 237, 215, 0.02);
  }
  .time-col {
    flex-shrink: 0;
  }
  .time-badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    font-weight: 700;
    color: var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.1);
    border: 1px solid rgba(220, 80, 0, 0.25);
    padding: 4px 8px;
    border-radius: 4px;
    white-space: nowrap;
  }
  .chapter-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }
  .chapter-title {
    font-size: 16px;
    font-weight: 600;
    margin: 0;
    color: var(--color-warm-cream);
    line-height: 1.35;
  }
  .chapter-desc {
    font-size: 14px;
    color: var(--color-warm-cream-dim);
    line-height: 1.6;
    margin: 0;
  }
  .empty-chapters-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: var(--spacing-24) var(--spacing-16);
    background: rgba(16, 9, 4, 0.3);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards, 8px);
    gap: var(--spacing-6);
  }
  .empty-icon-wrap {
    width: 44px;
    height: 44px;
    border-radius: 50%;
    background: rgba(255, 237, 215, 0.04);
    border: 1px solid var(--color-cork-border);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .empty-title {
    font-family: var(--font-display);
    font-size: 17px;
    color: var(--color-warm-cream);
    margin: 0;
  }
  .empty-desc {
    font-size: 13px;
    color: var(--color-warm-cream-dim);
    margin: 0;
  }
  @media (max-width: 580px) {
    .chapter-card {
      flex-direction: column;
      gap: 8px;
    }
  }
</style>
