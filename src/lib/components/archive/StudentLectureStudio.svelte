<script lang="ts">
  import { onMount } from 'svelte';
  import type { ClaimRecord } from '$lib/api';
  import {
    fetchSessionTranscript,
    type TranscriptLine,
  } from '$lib/api/aiStudy';
  import StudioHeader from './StudioHeader.svelte';
  import StudioNavTabs, { type StudioTab } from './StudioNavTabs.svelte';
  import TranscriptViewer from './TranscriptViewer.svelte';
  import ChapterBreakdown from './ChapterBreakdown.svelte';
  import FlashcardDeck from './FlashcardDeck.svelte';
  import AudioPlayerPanel from './AudioPlayerPanel.svelte';
  import LectureStudyNotes from './LectureStudyNotes.svelte';
  import VisualsPanel from './VisualsPanel.svelte';
  import RollingSummaryFeed from '$lib/components/shared/RollingSummaryFeed.svelte';

  let {
    claim,
    onBack,
  }: {
    claim: ClaimRecord;
    onBack?: () => void;
  } = $props();

  let activeTab = $state<StudioTab>('transcript');

  // The archive record's id is the lecture session UUID, which every study
  // endpoint is keyed by. (Guest/offline records fall back to the join code.)
  const sessionKey = $derived(
    claim.attended === undefined && claim.session_code
      ? claim.session_code
      : claim.id || claim.session_code || ''
  );

  let transcriptLines = $state<TranscriptLine[]>([]);
  let transcriptLoading = $state(true);
  let transcriptError = $state('');

  const stamp = (seconds: number | null) =>
    seconds == null
      ? ''
      : `[${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}] `;

  const transcriptText = $derived(
    transcriptLines.map((l) => `${stamp(l.offset_sec)}${l.text}`).join('\n')
  );

  async function loadTranscript() {
    transcriptLoading = true;
    transcriptError = '';
    try {
      const response = await fetchSessionTranscript(sessionKey);
      transcriptLines = response.lines;
    } catch (error) {
      transcriptLines = [];
      transcriptError =
        error instanceof Error
          ? error.message
          : 'Unable to load the transcript.';
    } finally {
      transcriptLoading = false;
    }
  }

  onMount(() => {
    void loadTranscript();
  });

  // Reload when a different lecture is picked from the list.
  let lastKey = '';
  $effect(() => {
    if (lastKey && lastKey !== sessionKey) void loadTranscript();
    lastKey = sessionKey;
  });
</script>

<div class="lecture-studio-pane">
  <StudioHeader {claim} {onBack} />
  <StudioNavTabs bind:activeTab />

  <div class="studio-content-body">
    {#key sessionKey}
      {#if activeTab === 'transcript'}
        <TranscriptViewer
          transcript={transcriptText}
          isLoading={transcriptLoading}
          errorMessage={transcriptError}
        />
      {:else if activeTab === 'summary'}
        <RollingSummaryFeed sessionId={sessionKey} title="Lecture summary" />
      {:else if activeTab === 'chapters'}
        <ChapterBreakdown sessionId={sessionKey} />
      {:else if activeTab === 'flashcards'}
        <FlashcardDeck sessionId={sessionKey} />
      {:else if activeTab === 'visuals'}
        <VisualsPanel sessionId={sessionKey} />
      {:else if activeTab === 'audio'}
        <AudioPlayerPanel sessionCode={sessionKey} />
      {:else if activeTab === 'notes'}
        <LectureStudyNotes
          sessionId={claim.id}
          sessionTitle={claim.session_title}
        />
      {/if}
    {/key}
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
