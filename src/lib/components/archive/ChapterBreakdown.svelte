<script lang="ts">
  import { onMount } from "svelte";
  import { fetchSessionChapters, type SessionChapter } from "$lib/api";
  import SkeletonCard from "$lib/components/shared/SkeletonCard.svelte";

  let { sessionId = "sess-demo-312" }: { sessionId?: string } = $props();

  let chapters = $state<SessionChapter[]>([]);
  let isLoading = $state(true);

  onMount(async () => {
    try {
      chapters = await fetchSessionChapters(sessionId);
    } catch {
      // Fallback
    } finally {
      isLoading = false;
    }
  });

  function formatTime(seconds: number): string {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  }
</script>

<div class="chapter-breakdown-panel">
  <div class="chapter-header">
    <p class="eyebrow">AI AUTOMATED LECTURE CHAPTERS</p>
    <span class="chapter-count">{chapters.length} Topic Sections</span>
  </div>

  {#if isLoading}
    <SkeletonCard lines={3} label="Generating AI chapter breakdown..." />
  {:else if chapters.length}
    <div class="chapters-list">
      {#each chapters as ch}
        <div class="chapter-card">
          <div class="chapter-time-badge">
            {formatTime(ch.start_timestamp_sec)} - {formatTime(ch.end_timestamp_sec)}
          </div>
          <div class="chapter-info">
            <h3>{ch.title}</h3>
            <p>{ch.summary}</p>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <p class="hint">No chapters generated yet for this session.</p>
  {/if}
</div>

<style>
  .chapter-breakdown-panel { display: flex; flex-direction: column; gap: var(--spacing-14); margin-top: var(--spacing-14); }
  .chapter-header { display: flex; justify-content: space-between; align-items: center; }
  .chapter-count { font-size: 10px; letter-spacing: 0.1em; color: var(--color-driftwood); }
  .chapters-list { display: flex; flex-direction: column; gap: var(--spacing-12); }
  .chapter-card { display: flex; gap: var(--spacing-14); padding: var(--spacing-14); background: rgba(16, 9, 4, 0.4); border: 1px solid var(--color-cork-border); border-radius: var(--radius-cards); }
  .chapter-time-badge { font-family: var(--font-display); font-size: 11px; color: var(--color-ember-accent); background: rgba(220, 80, 0, 0.1); border: 1px solid var(--color-ember-accent); padding: 4px 8px; border-radius: 4px; height: fit-content; white-space: nowrap; }
  .chapter-info h3 { font-size: 16px; margin-bottom: 4px; color: var(--color-warm-cream); }
  .chapter-info p { font-size: 13px; color: var(--color-warm-cream-dim); line-height: 1.5; }
</style>
