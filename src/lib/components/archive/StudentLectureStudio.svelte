<script lang="ts">
  import type { ClaimRecord } from '$lib/api';
  import StudioHeader from './StudioHeader.svelte';
  import StudioNavTabs, { type StudioTab } from './StudioNavTabs.svelte';
  import TranscriptViewer from './TranscriptViewer.svelte';
  import ChapterBreakdown from './ChapterBreakdown.svelte';
  import FlashcardDeck from './FlashcardDeck.svelte';
  import AudioPlayerPanel from './AudioPlayerPanel.svelte';
  import LectureStudyNotes from './LectureStudyNotes.svelte';

  let {
    claim,
    onBack,
  }: {
    claim: ClaimRecord;
    onBack?: () => void;
  } = $props();

  let activeTab = $state<StudioTab>('transcript');

  const transcriptText = $derived.by(() => {
    if (typeof localStorage === 'undefined') return '';
    try {
      const stored =
        localStorage.getItem(`klasync-captions-${claim.id}`) ||
        localStorage.getItem('klasync-captions');
      if (stored) {
        const parsed = JSON.parse(stored);
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
      '[01:30] Make sure to review core formulas and chapter breakdowns provided in your archive.',
      '[03:45] Any questions asked during class have been synced to your student record.',
    ].join('\n');
  });
</script>

<div class="lecture-studio-pane">
  <StudioHeader {claim} {onBack} />
  <StudioNavTabs bind:activeTab />

  <div class="studio-content-body">
    {#if activeTab === 'transcript'}
      <TranscriptViewer transcript={transcriptText} />
    {:else if activeTab === 'chapters'}
      <ChapterBreakdown sessionId={claim.session_code || claim.id} />
    {:else if activeTab === 'flashcards'}
      <FlashcardDeck sessionId={claim.session_code || claim.id} />
    {:else if activeTab === 'audio'}
      <AudioPlayerPanel sessionCode={claim.session_code || claim.id} />
    {:else if activeTab === 'notes'}
      <LectureStudyNotes
        sessionId={claim.id}
        sessionTitle={claim.session_title}
      />
    {/if}
  </div>
</div>

<style>
  .lecture-studio-pane {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14);
    width: 100%;
    max-width: 100%;
    min-width: 0;
    box-sizing: border-box;
  }
  .studio-content-body {
    padding-top: var(--spacing-4);
    width: 100%;
    max-width: 100%;
    min-width: 0;
    box-sizing: border-box;
  }
  @media (max-width: 640px) {
    .lecture-studio-pane {
      gap: var(--spacing-10);
    }
  }
</style>
