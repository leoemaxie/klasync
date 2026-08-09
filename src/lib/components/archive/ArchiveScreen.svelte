<script lang="ts">
  import { onMount } from 'svelte';
  import type { Screen } from '$lib/types';
  import PublicVisualPanel from '$lib/components/shared/PublicVisualPanel.svelte';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import TranscriptViewer from './TranscriptViewer.svelte';
  import FlashcardDeck from './FlashcardDeck.svelte';
  import AudioPlayerPanel from './AudioPlayerPanel.svelte';
  import ChapterBreakdown from './ChapterBreakdown.svelte';
  import {
    getArchiveResources,
    getStudentArchive,
    type ApiResource,
    type ClaimRecord,
  } from '$lib/api';

  let { screen = $bindable() }: { screen: Screen } = $props();

  let searchQuery = $state('');
  let activeTab = $state<'transcript' | 'chapters' | 'flashcards' | 'audio'>(
    'transcript'
  );
  let apiResources = $state<ApiResource[]>([]);
  let claims = $state<ClaimRecord[]>([]);
  let isLoading = $state(true);

  onMount(async () => {
    try {
      const [res, userClaims] = await Promise.all([
        getArchiveResources().catch(() => []),
        getStudentArchive().catch(() => []),
      ]);
      apiResources = res;
      claims = userClaims;
    } catch {
      claims = [];
    } finally {
      isLoading = false;
    }
  });

  const filteredClaims = $derived(
    claims.filter(
      (c) =>
        c.course_code.toLowerCase().includes(searchQuery.toLowerCase()) ||
        c.session_title.toLowerCase().includes(searchQuery.toLowerCase())
    )
  );
  function getClaimTranscript(claimId: string): string {
    if (typeof localStorage === 'undefined') return '';
    try {
      const storedCaptions =
        localStorage.getItem(`klasync-captions-${claimId}`) ||
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
  }
</script>

<svelte:head>
  <title>Student Archive — Klasync</title>
</svelte:head>

<section class="join-wrap">
  <div class="join-left-content">
    <p class="eyebrow">STUDENT ARCHIVE / CLAIMED LECTURES</p>
    <h1>Lecture Archive</h1>

    <div class="join-card panel">
      <label for="archive-search">
        Search archive
        <input
          id="archive-search"
          type="search"
          bind:value={searchQuery}
          placeholder="Search by course code or session title..."
        />
      </label>

      {#if isLoading}
        <div style="margin: var(--spacing-18) 0;">
          <SkeletonCard
            lines={3}
            label="Fetching student lecture archives from API..."
          />
        </div>
      {:else if filteredClaims.length > 0}
        <div class="archive-list">
          {#each filteredClaims as claim}
            <div class="archive-row">
              <span class="feat-num">{claim.course_code}</span>
              <h3>{claim.session_title}</h3>
              <p class="hint">
                Claimed on {claim.date} · Verified Student Access
              </p>

              <div
                class="tab-selector"
                role="group"
                aria-label="Archive view options"
              >
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
                  onclick={() => (activeTab = 'chapters')}>AI Chapters</button
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
                  onclick={() => (activeTab = 'audio')}>Audio Stream</button
                >
              </div>

              {#if activeTab === 'transcript'}
                <TranscriptViewer transcript={getClaimTranscript(claim.id)} />
              {:else if activeTab === 'chapters'}
                <ChapterBreakdown />
              {:else if activeTab === 'flashcards'}
                <FlashcardDeck />
              {:else}
                <AudioPlayerPanel />
              {/if}
            </div>
          {/each}
        </div>
      {:else}
        <div class="empty-archive-box">
          <p class="empty-title">No claimed lectures found</p>
          <p class="hint">
            Join an active lecture session with a short code and matric number
            to claim course materials.
          </p>
        </div>
      {/if}

      <a
        href="#/"
        class="primary full"
        style="text-align: center; text-decoration: none;"
        onclick={() => (screen = 'home')}
      >
        Return to Home
      </a>
    </div>
  </div>

  <PublicVisualPanel
    title="SEARCHABLE LECTURE ARCHIVE"
    subtitle="Full-text Search · AI Summaries · Audio Stream Replays"
  />
</section>

<style>
  .empty-archive-box {
    padding: var(--spacing-24);
    background: rgba(16, 9, 4, 0.4);
    border: 1px dashed var(--color-cork-border);
    border-radius: var(--radius-cards);
    text-align: center;
    margin: var(--spacing-14) 0;
  }
  .empty-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--color-warm-cream);
    margin-bottom: 4px;
  }
  @media (max-width: 600px) {
    :global(.join-card.panel) {
      padding: var(--spacing-18) !important;
    }
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
