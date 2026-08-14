<script lang="ts">
  import { onMount } from 'svelte';
  import type { Screen } from '$lib/types';
  import PublicVisualPanel from '$lib/components/shared/PublicVisualPanel.svelte';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import ClaimCard from './ClaimCard.svelte';
  import StudentCoursesHub from './StudentCoursesHub.svelte';
  import {
    getArchiveResources,
    getStudentArchive,
    type ApiResource,
    type ClaimRecord,
  } from '$lib/api';
  import { GraduationCap, Archive, Search } from '@lucide/svelte';

  let { screen = $bindable() }: { screen: Screen } = $props();

  let activeView = $state<'hub' | 'claims'>('hub');
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

  function handleFilterByCourse(courseCode: string) {
    searchQuery = courseCode;
    activeView = 'claims';
  }
</script>

<svelte:head>
  <title>Student Learning Portal — Klasync</title>
</svelte:head>

<section class="join-wrap">
  <div class="join-left-content">
    <div class="header-view-switch">
      <div class="view-toggle-btns" role="group" aria-label="Student portal view">
        <button
          type="button"
          class={activeView === 'hub' ? 'primary' : 'outline'}
          onclick={() => (activeView = 'hub')}
        >
          <GraduationCap size={14} />
          <span>My Enrolled Courses</span>
        </button>
        <button
          type="button"
          class={activeView === 'claims' ? 'primary' : 'outline'}
          onclick={() => (activeView = 'claims')}
        >
          <Archive size={14} />
          <span>Claimed Lectures ({claims.length})</span>
        </button>
      </div>
    </div>

    {#if activeView === 'hub'}
      <StudentCoursesHub onSelectCourseFilter={handleFilterByCourse} />
    {:else}
      <div class="join-card panel">
        <p class="eyebrow">STUDENT ARCHIVE / CLAIMED LECTURES</p>
        <h1 style="margin-top: 4px; font-size: 28px;">Lecture Archive</h1>

        <label for="archive-search">
          Search archive
          <input
            id="archive-search"
            type="search"
            bind:value={searchQuery}
            placeholder="Search by course code or session title..."
          />
        </label>

        {#if searchQuery}
          <div class="filter-indicator-bar">
            <span>Filtered by: <strong>{searchQuery}</strong></span>
            <button type="button" class="text clear-filter-btn" onclick={() => (searchQuery = '')}>
              Clear Filter
            </button>
          </div>
        {/if}

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
      </div>
    {/if}

    <button
      type="button"
      class="outline full"
      style="text-align: center; margin-top: var(--spacing-18); max-width: 440px;"
      onclick={() => (screen = 'home')}
    >
      Return to Home
    </button>
  </div>

  <PublicVisualPanel
    title="CALENDAR-AWARE LECTURE ARCHIVE"
    subtitle="Multi-Course Directory · AI Summaries · Audio Stream Replays"
  />
</section>

<style>
  .header-view-switch {
    margin-bottom: var(--spacing-16);
  }
  .view-toggle-btns {
    display: flex;
    gap: var(--spacing-8);
    flex-wrap: wrap;
  }
  .view-toggle-btns button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .filter-indicator-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: rgba(16, 9, 4, 0.4);
    border: 1px solid var(--color-cork-border);
    padding: 6px 12px;
    border-radius: 4px;
    font-size: 11px;
    color: var(--color-driftwood);
  }
  .filter-indicator-bar strong {
    color: var(--color-warm-cream);
  }
  .clear-filter-btn {
    font-size: 10px;
    text-transform: uppercase;
    color: var(--color-ember-accent);
    padding: 0;
  }
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
