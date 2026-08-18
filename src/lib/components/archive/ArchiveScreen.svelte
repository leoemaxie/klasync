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

  onMount(async () => {
    try {
      claims = await getStudentArchive().catch(() => []);
      if (claims.length > 0 && window.innerWidth > 960)
        selectedClaim = claims[0];
    } finally {
      isLoading = false;
    }
  });

  function handleSelectCourse(code: string) {
    searchQuery = code;
    activeViewMode = 'claims';
    const first = claims.find(
      (c) => c.course_code.toLowerCase() === code.toLowerCase()
    );
    if (first) selectedClaim = first;
  }
</script>

<svelte:head>
  <title>Student Archive Studio — Klasync</title>
</svelte:head>

<div
  class="student-studio-workspace"
  class:mobile-detail-open={!!selectedClaim}
>
  <ArchiveTopBar claimsCount={claims.length} onNavigate={(s) => (screen = s)} />

  <div class="studio-layout">
    <div
      class="sidebar-slot"
      class:hidden-on-mobile={!!selectedClaim || activeViewMode === 'courses'}
    >
      <ArchiveSidebar
        bind:activeViewMode
        bind:searchQuery
        {claims}
        {selectedClaim}
        {isLoading}
        onSelectClaim={(c) => (selectedClaim = c)}
      />
    </div>

    <main
      class="studio-detail-canvas panel"
      class:visible-on-mobile={!!selectedClaim || activeViewMode === 'courses'}
    >
      {#if activeViewMode === 'courses' && !selectedClaim}
        <StudentCoursesHub onSelectCourseFilter={handleSelectCourse} />
      {:else if selectedClaim}
        <StudentLectureStudio
          claim={selectedClaim}
          onBack={() => (selectedClaim = null)}
        />
      {:else}
        <ArchiveWelcomeView
          {claims}
          onOpenLatest={(c) => (selectedClaim = c)}
        />
      {/if}
    </main>
  </div>
</div>

<style>
  .student-studio-workspace {
    padding: calc(var(--nav-height) + 16px) var(--card-padding)
      calc(var(--spacing-68) + env(safe-area-inset-bottom, 0px));
    max-width: 1440px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14);
  }
  .studio-layout {
    display: grid;
    grid-template-columns: 350px 1fr;
    gap: var(--spacing-16);
    align-items: start;
    min-height: 600px;
  }
  .studio-detail-canvas {
    background: rgba(16, 9, 4, 0.5);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
    padding: var(--spacing-16);
    min-height: 600px;
  }
  @media (max-width: 960px) {
    .studio-layout {
      grid-template-columns: 1fr;
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
      padding: calc(var(--nav-height) + 10px) 12px
        calc(90px + env(safe-area-inset-bottom, 0px));
      gap: var(--spacing-10);
    }
    .studio-detail-canvas {
      padding: 10px;
    }
  }
</style>
