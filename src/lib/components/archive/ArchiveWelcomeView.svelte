<script lang="ts">
  import type { ClaimRecord } from '$lib/api';
  import { Sparkles, FileText, Headphones, BookOpen } from '@lucide/svelte';

  let {
    claims = [],
    onOpenLatest,
  }: {
    claims: ClaimRecord[];
    onOpenLatest: (claim: ClaimRecord) => void;
  } = $props();
</script>

<div class="welcome-studio-state">
  <div class="welcome-badge">
    <Sparkles size={14} color="var(--color-ember-accent)" />
    <span>STUDY STUDIO</span>
  </div>
  <h2 class="welcome-title">Select a Lecture</h2>
  <p class="welcome-desc">
    Choose a lecture from the sidebar to review transcripts, notes, and flashcards.
  </p>

  <div class="quick-overview-grid">
    <div class="overview-box">
      <FileText size={18} color="var(--color-ember-accent)" />
      <h4>Transcripts</h4>
      <p>Searchable lecture transcripts.</p>
    </div>
    <div class="overview-box">
      <Sparkles size={18} color="var(--color-ember-accent)" />
      <h4>Flashcards</h4>
      <p>Flashcards generated from lecture content.</p>
    </div>
    <div class="overview-box">
      <Headphones size={18} color="var(--color-ember-accent)" />
      <h4>Audio Replays</h4>
      <p>Audio playback with speed controls.</p>
    </div>
  </div>

  {#if claims.length > 0}
    <div class="start-action">
      <button
        type="button"
        class="primary"
        onclick={() => onOpenLatest(claims[0])}
      >
        <BookOpen size={14} style="display:inline-block; vertical-align:middle; margin-right:4px;" />
        Open Latest ({claims[0].course_code})
      </button>
    </div>
  {/if}
</div>

<style>
  .welcome-studio-state { display: flex; flex-direction: column; align-items: center; text-align: center; padding: var(--spacing-20) var(--spacing-10); gap: var(--spacing-10); max-width: 680px; margin: 0 auto; }
  .welcome-badge { display: inline-flex; align-items: center; gap: 6px; font-size: 11px; color: var(--color-ember-accent); background: rgba(220, 80, 0, 0.1); border: 1px solid rgba(220, 80, 0, 0.2); padding: 4px 12px; border-radius: 999px; }
  .welcome-title { font-family: var(--font-display); font-size: 24px; color: var(--color-warm-cream); margin: 0; }
  .welcome-desc { font-size: 13px; color: var(--color-driftwood); line-height: 1.6; margin: 0; }
  .quick-overview-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: var(--spacing-10); width: 100%; margin-top: var(--spacing-6); text-align: left; }
  .overview-box { padding: var(--spacing-10); background: rgba(16, 9, 4, 0.4); border: 1px solid var(--color-cork-border); border-radius: 6px; display: flex; flex-direction: column; gap: 4px; }
  .overview-box h4 { font-size: 12px; color: var(--color-warm-cream); margin: 0; }
  .overview-box p { font-size: 11px; color: var(--color-driftwood); margin: 0; line-height: 1.4; }
  .start-action { margin-top: var(--spacing-4); }
  .start-action button { padding: 8px 18px; font-size: 11px; text-transform: uppercase; }
  @media (max-width: 768px) {
    .quick-overview-grid { grid-template-columns: 1fr; }
    .welcome-studio-state { padding: var(--spacing-14) var(--spacing-6); }
    .welcome-title { font-size: 20px; }
  }
</style>
