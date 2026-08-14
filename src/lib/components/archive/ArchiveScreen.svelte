<script lang="ts">
  import { onMount } from 'svelte';
  import type { Screen } from '$lib/types';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import StudentCoursesHub from './StudentCoursesHub.svelte';
  import StudentLectureStudio from './StudentLectureStudio.svelte';
  import {
    getArchiveResources,
    getStudentArchive,
    type ApiResource,
    type ClaimRecord,
  } from '$lib/api';
  import {
    GraduationCap,
    Archive,
    Search,
    BookOpen,
    Calendar,
    ChevronRight,
    ArrowLeft,
    Sparkles,
    Radio,
    FileText,
    CheckCircle2,
    Layers,
    Filter,
  } from '@lucide/svelte';

  let { screen = $bindable() }: { screen: Screen } = $props();

  let activeViewMode = $state<'courses' | 'claims'>('claims');
  let searchQuery = $state('');
  let selectedClaim = $state<ClaimRecord | null>(null);
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

      // Check if URL hash has a specific lecture query e.g. #archive?lecture=ID
      const hashParams = new URLSearchParams(window.location.hash.split('?')[1] || '');
      const lectureId = hashParams.get('lecture');
      if (lectureId && userClaims.length > 0) {
        const matched = userClaims.find((c) => c.id === lectureId);
        if (matched) selectedClaim = matched;
      } else if (userClaims.length > 0) {
        // Auto-select first lecture on desktop view if available
        if (window.innerWidth > 960) {
          selectedClaim = userClaims[0];
        }
      }
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
        c.session_title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        (c.course_title && c.course_title.toLowerCase().includes(searchQuery.toLowerCase()))
    )
  );

  function handleSelectCourseFilter(courseCode: string) {
    searchQuery = courseCode;
    activeViewMode = 'claims';
    const firstMatched = claims.find(
      (c) => c.course_code.toLowerCase() === courseCode.toLowerCase()
    );
    if (firstMatched) {
      selectedClaim = firstMatched;
    }
  }

  function handleSelectLecture(claim: ClaimRecord) {
    selectedClaim = claim;
  }
</script>

<svelte:head>
  <title>Student Archive Studio — Klasync</title>
</svelte:head>

<div class="student-studio-workspace">
  <!-- Desktop Workspace Navigation Bar -->
  <header class="workspace-top-bar">
    <div class="top-bar-left">
      <div class="eyebrow-brand">
        <span class="brand-pill">KLASYNC</span>
        <span class="divider">/</span>
        <span class="workspace-label">STUDENT ARCHIVE STUDIO</span>
      </div>
      <h1 class="workspace-title">Lecture Archive & Study Studio</h1>
    </div>

    <div class="top-bar-right">
      <div class="stat-badge">
        <Archive size={13} color="var(--color-ember-accent)" />
        <span>{claims.length} {claims.length === 1 ? 'Claimed Lecture' : 'Claimed Lectures'}</span>
      </div>

      <button
        type="button"
        class="outline join-cta-btn"
        onclick={() => (screen = 'join')}
        title="Join a live lecture session"
      >
        <Radio size={13} color="var(--color-ember-accent)" />
        <span>Join Live Session</span>
      </button>

      <button
        type="button"
        class="text home-link-btn"
        onclick={() => (screen = 'home')}
      >
        Return Home
      </button>
    </div>
  </header>

  <!-- Master-Detail 2-Column Split Layout -->
  <div class="studio-layout">
    <!-- Master Sidebar (Left Explorer) -->
    <aside class="studio-sidebar panel">
      <div class="sidebar-header">
        <div class="view-mode-tabs" role="tablist" aria-label="Archive navigation mode">
          <button
            type="button"
            role="tab"
            class="mode-btn"
            class:active={activeViewMode === 'claims'}
            aria-selected={activeViewMode === 'claims'}
            onclick={() => (activeViewMode = 'claims')}
          >
            <Archive size={13} />
            <span>All Lectures ({claims.length})</span>
          </button>
          <button
            type="button"
            role="tab"
            class="mode-btn"
            class:active={activeViewMode === 'courses'}
            aria-selected={activeViewMode === 'courses'}
            onclick={() => {
              activeViewMode = 'courses';
              selectedClaim = null;
            }}
          >
            <GraduationCap size={13} />
            <span>Course Directory</span>
          </button>
        </div>

        <div class="search-input-wrap">
          <Search size={13} class="search-icon" />
          <input
            type="search"
            bind:value={searchQuery}
            placeholder="Search code or lecture title..."
            class="sidebar-search-input"
            aria-label="Filter lecture archives"
          />
          {#if searchQuery}
            <button
              type="button"
              class="clear-query-btn text"
              onclick={() => (searchQuery = '')}
            >
              Clear
            </button>
          {/if}
        </div>
      </div>

      <!-- Master List Section -->
      <div class="sidebar-list-container">
        {#if isLoading}
          <div class="sidebar-skeleton-wrap">
            <SkeletonCard lines={2} label="Loading lectures..." />
            <SkeletonCard lines={2} label="Syncing student claims..." />
          </div>
        {:else if activeViewMode === 'claims'}
          {#if filteredClaims.length > 0}
            <div class="claims-stack" role="list">
              {#each filteredClaims as claim (claim.id)}
                <button
                  type="button"
                  class="claim-item-card"
                  class:selected={selectedClaim?.id === claim.id}
                  onclick={() => handleSelectLecture(claim)}
                  role="listitem"
                  aria-current={selectedClaim?.id === claim.id ? 'true' : undefined}
                >
                  <div class="item-head">
                    <span class="course-code-tag">{claim.course_code}</span>
                    <span class="date-tag">{claim.date}</span>
                  </div>

                  <h3 class="item-title">{claim.session_title}</h3>

                  <div class="item-footer">
                    <div class="item-badges">
                      <span class="badge verified">
                        <CheckCircle2 size={10} /> Verified
                      </span>
                      {#if claim.academic_session}
                        <span class="badge session">{claim.academic_session}</span>
                      {/if}
                    </div>
                    <ChevronRight size={14} class="item-arrow" />
                  </div>
                </button>
              {/each}
            </div>
          {:else}
            <div class="empty-sidebar-box">
              <Archive size={24} color="var(--color-driftwood)" />
              <p class="empty-sidebar-title">No lectures match query</p>
              <p class="hint">Try searching another course code or clear the filter.</p>
            </div>
          {/if}
        {:else}
          <!-- Course directory shortcut view -->
          <div class="course-directory-shortcut">
            <p class="shortcut-desc">
              Browse your enrolled courses. Select any course to filter and study its materials.
            </p>
          </div>
        {/if}
      </div>

      <!-- Sidebar Footer Status -->
      <div class="sidebar-footer">
        <div class="sync-status-row">
          <span class="sync-dot"></span>
          <span class="sync-label">Archive Sync Active</span>
        </div>
      </div>
    </aside>

    <!-- Detail Reading Studio (Right Pane) -->
    <main class="studio-detail-canvas panel">
      {#if activeViewMode === 'courses' && !selectedClaim}
        <StudentCoursesHub onSelectCourseFilter={handleSelectCourseFilter} />
      {:else if selectedClaim}
        <StudentLectureStudio
          claim={selectedClaim}
          onBack={() => (selectedClaim = null)}
        />
      {:else}
        <!-- Welcome / Default State -->
        <div class="welcome-studio-state">
          <div class="welcome-badge">
            <Sparkles size={14} color="var(--color-ember-accent)" />
            <span>ACCESSIBILITY-FIRST REVISION ENGINE</span>
          </div>

          <h2 class="welcome-title">Select a Lecture to Begin Studying</h2>
          <p class="welcome-desc">
            Choose any claimed lecture from the left explorer pane to open the interactive
            reading canvas, synchronized transcripts, AI topic breakdowns, audio replays, and
            active recall flashcards.
          </p>

          <div class="quick-overview-grid">
            <div class="overview-box">
              <FileText size={18} color="var(--color-ember-accent)" />
              <h4>Synchronized Transcripts</h4>
              <p>Full live transcripts with font scaling, keyboard navigation, and keyword search.</p>
            </div>

            <div class="overview-box">
              <Layers size={18} color="var(--color-ember-accent)" />
              <h4>AI Revision Flashcards</h4>
              <p>Interactive self-testing flashcard decks and custom AI flashcard generator.</p>
            </div>

            <div class="overview-box">
              <Radio size={18} color="var(--color-ember-accent)" />
              <h4>48kHz Audio Replays</h4>
              <p>Stereophonic audio stream playback with variable speed scrubber controls.</p>
            </div>
          </div>

          {#if filteredClaims.length > 0}
            <div class="start-reading-action">
              <button
                type="button"
                class="primary"
                onclick={() => (selectedClaim = filteredClaims[0])}
              >
                Open Latest Lecture ({filteredClaims[0].course_code})
              </button>
            </div>
          {/if}
        </div>
      {/if}
    </main>
  </div>
</div>

<style>
  .student-studio-workspace {
    padding: calc(var(--nav-height) + 20px) var(--card-padding) var(--spacing-68);
    max-width: 1440px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-18);
  }

  /* ── Top Bar ────────────────────────────────────────── */
  .workspace-top-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--spacing-14);
    padding-bottom: var(--spacing-14);
    border-bottom: 1px solid var(--color-cork-border);
  }

  .eyebrow-brand {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    margin-bottom: 4px;
  }

  .brand-pill {
    color: var(--color-ember-accent);
    font-weight: 700;
    font-family: var(--font-mono, monospace);
  }

  .divider {
    color: var(--color-cork-border);
  }

  .workspace-label {
    color: var(--color-driftwood);
  }

  .workspace-title {
    font-family: var(--font-display);
    font-size: 26px;
    color: var(--color-warm-cream);
    margin: 0;
  }

  .top-bar-right {
    display: flex;
    align-items: center;
    gap: var(--spacing-12);
    flex-wrap: wrap;
  }

  .stat-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--color-warm-cream);
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    padding: 6px 12px;
    border-radius: 4px;
  }

  .join-cta-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    padding: 6px 12px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .home-link-btn {
    font-size: 11px;
    text-transform: uppercase;
    color: var(--color-driftwood);
  }

  .home-link-btn:hover {
    color: var(--color-warm-cream);
  }

  /* ── Master-Detail Layout ───────────────────────────── */
  .studio-layout {
    display: grid;
    grid-template-columns: 360px 1fr;
    gap: var(--spacing-18);
    align-items: start;
    min-height: 640px;
  }

  /* ── Master Sidebar ─────────────────────────────────── */
  .studio-sidebar {
    display: flex;
    flex-direction: column;
    background: rgba(16, 9, 4, 0.5);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
    padding: var(--spacing-14);
    gap: var(--spacing-14);
    max-height: calc(100vh - var(--nav-height) - 160px);
    position: sticky;
    top: calc(var(--nav-height) + 20px);
  }

  .sidebar-header {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-10);
  }

  .view-mode-tabs {
    display: flex;
    gap: 4px;
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    padding: 3px;
    border-radius: 4px;
  }

  .mode-btn {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 6px 8px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    background: transparent;
    border: none;
    color: var(--color-driftwood);
    border-radius: 3px;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .mode-btn.active {
    background: var(--color-bark-brown);
    color: var(--color-warm-cream);
  }

  .search-input-wrap {
    position: relative;
    display: flex;
    align-items: center;
  }

  :global(.search-input-wrap .search-icon) {
    position: absolute;
    left: 10px;
    color: var(--color-driftwood);
  }

  .sidebar-search-input {
    width: 100%;
    padding-left: 30px !important;
    font-size: 12px;
    margin: 0;
  }

  .clear-query-btn {
    position: absolute;
    right: 8px;
    font-size: 10px;
    text-transform: uppercase;
    color: var(--color-ember-accent);
  }

  .sidebar-list-container {
    overflow-y: auto;
    padding-right: 4px;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-8);
  }

  .claims-stack {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-8);
  }

  .claim-item-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: var(--spacing-12);
    background: rgba(16, 9, 4, 0.4);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    text-align: left;
    cursor: pointer;
    transition: all 0.2s ease;
    width: 100%;
  }

  .claim-item-card:hover {
    border-color: var(--color-warm-cream);
    background: rgba(255, 237, 215, 0.03);
  }

  .claim-item-card.selected {
    border-color: var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.08);
  }

  .item-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .course-code-tag {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    font-weight: 700;
    color: var(--color-ember-accent);
  }

  .date-tag {
    font-size: 10px;
    color: var(--color-driftwood);
  }

  .item-title {
    font-family: var(--font-display);
    font-size: 14px;
    margin: 0;
    color: var(--color-warm-cream);
    line-height: 1.3;
  }

  .item-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 2px;
  }

  .item-badges {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .badge {
    font-size: 9px;
    padding: 2px 6px;
    border-radius: 3px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .badge.verified {
    background: rgba(74, 222, 128, 0.1);
    color: #4ade80;
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  .badge.session {
    background: rgba(255, 237, 215, 0.08);
    color: var(--color-driftwood);
  }

  :global(.item-arrow) {
    color: var(--color-driftwood);
  }

  .claim-item-card.selected :global(.item-arrow) {
    color: var(--color-ember-accent);
  }

  .empty-sidebar-box {
    padding: var(--spacing-24);
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }

  .empty-sidebar-title {
    font-size: 13px;
    color: var(--color-warm-cream);
    margin: 0;
  }

  .course-directory-shortcut {
    padding: var(--spacing-14);
    text-align: center;
  }

  .shortcut-desc {
    font-size: 12px;
    color: var(--color-driftwood);
    line-height: 1.5;
  }

  .sidebar-footer {
    border-top: 1px solid var(--color-cork-border);
    padding-top: var(--spacing-10);
    margin-top: auto;
  }

  .sync-status-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .sync-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #4ade80;
  }

  .sync-label {
    font-size: 10px;
    color: var(--color-driftwood);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  /* ── Detail Reading Studio Canvas ───────────────────── */
  .studio-detail-canvas {
    background: rgba(16, 9, 4, 0.5);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
    padding: var(--spacing-20);
    min-height: 640px;
  }

  .welcome-studio-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: var(--spacing-36) var(--spacing-18);
    gap: var(--spacing-14);
    max-width: 680px;
    margin: 0 auto;
  }

  .welcome-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    letter-spacing: 0.12em;
    color: var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.1);
    border: 1px solid rgba(220, 80, 0, 0.2);
    padding: 4px 12px;
    border-radius: 999px;
  }

  .welcome-title {
    font-family: var(--font-display);
    font-size: 28px;
    color: var(--color-warm-cream);
    margin: 0;
  }

  .welcome-desc {
    font-size: 14px;
    color: var(--color-driftwood);
    line-height: 1.6;
    margin: 0;
  }

  .quick-overview-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--spacing-12);
    width: 100%;
    margin-top: var(--spacing-12);
    text-align: left;
  }

  .overview-box {
    padding: var(--spacing-14);
    background: rgba(16, 9, 4, 0.4);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .overview-box h4 {
    font-size: 13px;
    color: var(--color-warm-cream);
    margin: 0;
  }

  .overview-box p {
    font-size: 11px;
    color: var(--color-driftwood);
    margin: 0;
    line-height: 1.4;
  }

  .start-reading-action {
    margin-top: var(--spacing-12);
  }

  .start-reading-action button {
    padding: 10px 24px;
    font-size: 12px;
    text-transform: uppercase;
  }

  /* ── Responsive Breakpoints ─────────────────────────── */
  @media (max-width: 960px) {
    .studio-layout {
      grid-template-columns: 1fr;
    }
    .studio-sidebar {
      position: static;
      max-height: none;
    }
    .quick-overview-grid {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 640px) {
    .student-studio-workspace {
      padding-bottom: 90px;
    }
    .workspace-title {
      font-size: 22px;
    }
    .top-bar-right {
      width: 100%;
      justify-content: space-between;
    }
  }
</style>
