<script lang="ts">
  import { onMount } from 'svelte';
  import type { Screen } from '$lib/types';
  import ArchiveTopBar from './ArchiveTopBar.svelte';
  import ArchiveSidebar from './ArchiveSidebar.svelte';
  import ArchiveWelcomeView from './ArchiveWelcomeView.svelte';
  import StudentLectureStudio from './StudentLectureStudio.svelte';
  import StudentCoursesHub from './StudentCoursesHub.svelte';
  import { getStudentArchive, type ClaimRecord } from '$lib/api';

  let { screen = $bindable() }: { screen: Screen } = $props();

  let activeViewMode = $state<'courses' | 'claims'>('claims');
  let searchQuery = $state('');
  let selectedClaim = $state<ClaimRecord | null>(null);
  let claims = $state<ClaimRecord[]>([]);
  let isLoading = $state(true);
  /** Focus mode hides the page chrome so the lecture gets the whole screen. */
  let focusMode = $state(false);
  /** Whether the lecture list is open beside an open lecture (desktop). */
  let listOpen = $state(false);

  onMount(async () => {
    try {
      claims = await getStudentArchive().catch(() => []);
      // Share links look like #/archive?lecture=<id>.
      const linked = new URLSearchParams(
        window.location.hash.split('?')[1] ?? ''
      ).get('lecture');
      const target = linked ? claims.find((c) => c.id === linked) : undefined;
      if (target) selectedClaim = target;
      else if (claims.length > 0 && window.innerWidth > 960)
        selectedClaim = claims[0];
    } finally {
      isLoading = false;
    }
  });

  function openClaim(claim: ClaimRecord) {
    selectedClaim = claim;
    // Give the lecture the room back once one is chosen.
    listOpen = false;
  }

  function handleSelectCourse(code: string) {
    searchQuery = code;
    activeViewMode = 'claims';
    const first = claims.find(
      (c) => c.course_code.toLowerCase() === code.toLowerCase()
    );
    if (first) openClaim(first);
  }
</script>

<svelte:head>
  <title>Student Archive Studio — Klasync</title>
</svelte:head>

<div
  class="student-studio-workspace"
  class:mobile-detail-open={!!selectedClaim}
  class:focus-mode={focusMode && !!selectedClaim}
  class:has-claim={!!selectedClaim}
>
  {#if !selectedClaim}
    <ArchiveTopBar
      claimsCount={claims.length}
      onNavigate={(s) => (screen = s)}
    />
  {/if}

  <div class="studio-layout" class:list-closed={!!selectedClaim && !listOpen}>
    <div
      class="sidebar-slot"
      class:hidden-on-mobile={!!selectedClaim || activeViewMode === 'courses'}
      class:collapsed={!!selectedClaim && !listOpen}
    >
      <ArchiveSidebar
        bind:activeViewMode
        bind:searchQuery
        {claims}
        {selectedClaim}
        {isLoading}
        onSelectClaim={openClaim}
      />
    </div>

    <section
      class="studio-detail-canvas panel"
      class:immersive={!!selectedClaim}
      class:visible-on-mobile={!!selectedClaim || activeViewMode === 'courses'}
    >
      {#if activeViewMode === 'courses' && !selectedClaim}
        <StudentCoursesHub onSelectCourseFilter={handleSelectCourse} />
      {:else if selectedClaim}
        <StudentLectureStudio
          claim={selectedClaim}
          onBack={() => (selectedClaim = null)}
          bind:focus={focusMode}
          bind:listOpen
          showListToggle
        />
      {:else}
        <ArchiveWelcomeView {claims} onOpenLatest={openClaim} />
      {/if}
    </section>
  </div>
</div>

<style>
  .student-studio-workspace {
    padding: calc(var(--app-top-offset, var(--nav-height)) + 16px)
      var(--card-padding)
      calc(var(--spacing-68) + env(safe-area-inset-bottom, 0px));
    max-width: 1440px;
    width: 100%;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14);
    box-sizing: border-box;
    overflow-x: hidden;
  }
  .studio-layout {
    display: grid;
    grid-template-columns: 350px minmax(0, 1fr);
    gap: var(--spacing-16);
    align-items: start;
    min-height: 600px;
    width: 100%;
    min-width: 0;
  }
  .studio-layout.list-closed {
    grid-template-columns: minmax(0, 1fr);
  }
  .sidebar-slot {
    min-width: 0;
    width: 100%;
  }
  .sidebar-slot.collapsed {
    display: none;
  }
  .has-claim {
    padding-bottom: 20px;
  }
  .focus-mode {
    padding-top: calc(var(--app-top-offset, var(--nav-height)) + 10px);
    padding-bottom: 16px;
  }
  .studio-detail-canvas.immersive {
    background: transparent;
    border: 0;
    padding: 0;
    min-height: 0;
    overflow: visible;
  }
  .studio-detail-canvas {
    background: rgba(16, 9, 4, 0.5);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
    padding: var(--spacing-16);
    min-height: 600px;
    min-width: 0;
    width: 100%;
    max-width: 100%;
    box-sizing: border-box;
    overflow-x: hidden;
  }
  @media (max-width: 960px) {
    .mobile-detail-open :global(.workspace-top-bar) {
      display: none;
    }
    .studio-layout {
      grid-template-columns: minmax(0, 1fr);
      min-height: auto;
      gap: var(--spacing-10);
    }
    .studio-detail-canvas {
      min-height: auto;
      padding: var(--spacing-12);
    }
    .hidden-on-mobile {
      display: none;
    }
    .studio-detail-canvas:not(.visible-on-mobile) {
      display: none;
    }
  }
  @media (max-width: 640px) {
    .student-studio-workspace {
      padding: calc(var(--nav-height) + 10px) 10px
        calc(90px + env(safe-area-inset-bottom, 0px));
      gap: var(--spacing-10);
    }
    .studio-detail-canvas {
      padding: 8px;
      border-radius: 6px;
    }
    .studio-detail-canvas.immersive {
      padding: 0;
    }
  }
</style>
