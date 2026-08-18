<script lang="ts">
  import type { ClaimRecord } from '$lib/api';
  import TranscriptViewer from './TranscriptViewer.svelte';
  import ChapterBreakdown from './ChapterBreakdown.svelte';
  import FlashcardDeck from './FlashcardDeck.svelte';
  import AudioPlayerPanel from './AudioPlayerPanel.svelte';

  let { claim }: { claim: ClaimRecord } = $props();

  let activeTab = $state<'transcript' | 'chapters' | 'flashcards' | 'audio'>(
    'transcript'
  );

  const transcriptText = $derived.by(() => {
    if (typeof localStorage === 'undefined') return '';
    try {
      const storedCaptions =
        localStorage.getItem(`klasync-captions-${claim.id}`) ||
        localStorage.getItem('klasync-captions');
      if (storedCaptions) {
        const parsed = JSON.parse(storedCaptions);
        if (Array.isArray(parsed) && parsed.length > 0) {
          return parsed
            .map((c: any) =>
              typeof c === 'string' ? c : c.text || c.content || ''
            )
            .filter(Boolean)
            .join('\n');
        }
      }
    } catch {}
    return [
      "[00:01] Welcome everyone to today's lecture session.",
      '[00:15] Today we are discussing key principles, system architecture, and accessibility.',
      '[01:30] Make sure to review the core formulas and chapter breakdowns provided in your archive.',
      '[03:45] Any questions asked during class have been synced to your student Q&A record.',
    ].join('\n');
  });
</script>

<div class="archive-row">
  <div class="card-title-header">
    <div class="title-meta-left">
      <span class="feat-num">{claim.course_code}</span>
      {#if claim.course_title && claim.course_title !== claim.course_code}
        <span class="course-sub-title">· {claim.course_title}</span>
      {/if}
    </div>
    {#if claim.academic_session || claim.semester}
      <div class="session-badges">
        {#if claim.academic_session}
          <span class="tag session-tag">{claim.academic_session}</span>
        {/if}
        {#if claim.semester}
          <span class="tag semester-tag">{claim.semester}</span>
        {/if}
      </div>
    {/if}
  </div>

  <h3>{claim.session_title}</h3>
  <p class="hint">Saved {claim.date}</p>

  <div class="tab-selector" role="group" aria-label="Archive view options">
    <button
      type="button"
      class={activeTab === 'transcript' ? 'primary' : 'outline'}
      aria-pressed={activeTab === 'transcript'}
      onclick={() => (activeTab = 'transcript')}>Transcript</button
    >
    <button
      type="button"
      class={activeTab === 'chapters' ? 'primary' : 'outline'}
      aria-pressed={activeTab === 'chapters'}
      onclick={() => (activeTab = 'chapters')}>Chapters</button
    >
    <button
      type="button"
      class={activeTab === 'flashcards' ? 'primary' : 'outline'}
      aria-pressed={activeTab === 'flashcards'}
      onclick={() => (activeTab = 'flashcards')}>Flashcards</button
    >
    <button
      type="button"
      class={activeTab === 'audio' ? 'primary' : 'outline'}
      aria-pressed={activeTab === 'audio'}
      onclick={() => (activeTab = 'audio')}>Audio</button
    >
  </div>

  {#if activeTab === 'transcript'}
    <TranscriptViewer transcript={transcriptText} />
  {:else if activeTab === 'chapters'}
    <ChapterBreakdown sessionId={claim.id} />
  {:else if activeTab === 'flashcards'}
    <FlashcardDeck sessionId={claim.session_code || claim.id} />
  {:else}
    <AudioPlayerPanel sessionCode={claim.session_code || ''} />
  {/if}
</div>

<style>
  .archive-row {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-12);
    padding: var(--spacing-18);
    background: rgba(16, 9, 4, 0.4);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
    margin-bottom: var(--spacing-16);
  }
  .card-title-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }
  .title-meta-left {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .course-sub-title {
    font-size: 11px;
    color: var(--color-warm-cream-dim, #ffedd7aa);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .session-badges {
    display: flex;
    gap: 4px;
  }
  .tag {
    font-size: 9px;
    padding: 2px 6px;
    border-radius: 3px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    font-weight: 600;
  }
  .session-tag {
    background: rgba(255, 237, 215, 0.1);
    color: var(--color-driftwood);
  }
  .semester-tag {
    background: rgba(255, 237, 215, 0.15);
    color: var(--color-warm-cream);
  }
  .feat-num {
    font-size: 11px;
    letter-spacing: 0.12em;
    color: var(--color-ember-accent);
    font-weight: 700;
    text-transform: uppercase;
  }
  .archive-row h3 {
    margin: 0;
    font-size: 18px;
    color: var(--color-warm-cream);
    font-family: var(--font-display);
  }
  .tab-selector {
    display: flex;
    gap: var(--spacing-8);
    margin: var(--spacing-8) 0;
    flex-wrap: wrap;
  }
  .tab-selector button {
    padding: 6px 14px;
    font-size: 11px;
    text-transform: uppercase;
  }
  @media (max-width: 600px) {
    .tab-selector {
      display: grid;
      grid-template-columns: repeat(2, 1fr);
      gap: var(--spacing-8);
    }
    .tab-selector button {
      width: 100%;
      text-align: center;
      padding: 8px 10px;
      font-size: 10px;
    }
  }
</style>
