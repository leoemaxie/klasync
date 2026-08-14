<script lang="ts">
  import { onMount } from 'svelte';
  import type { Screen } from '$lib/types';
  import PublicVisualPanel from '$lib/components/shared/PublicVisualPanel.svelte';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import ClaimCard from './ClaimCard.svelte';
  import {
    getArchiveResources,
    getStudentArchive,
    type ApiResource,
    type ClaimRecord,
  } from '$lib/api';

  let { screen = $bindable() }: { screen: Screen } = $props();

  let searchQuery = $state('');
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
          {#each filteredClaims as claim (claim.id)}
            <ClaimCard {claim} />
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

      <button
        type="button"
        class="primary full"
        style="text-align: center; margin-top: var(--spacing-12);"
        onclick={() => (screen = 'home')}
      >
        Return to Home
      </button>
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
  }
</style>
