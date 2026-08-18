<script lang="ts">
  import { onMount } from 'svelte';
  import type { StudentEnrolledCourse } from '$lib/types';
  import { getStudentCourses } from '$lib/api/courses';
  import { GraduationCap, Search, BookOpen } from '@lucide/svelte';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import StudentCourseCard from './StudentCourseCard.svelte';

  let {
    onSelectCourseFilter,
  }: { onSelectCourseFilter?: (courseCode: string) => void } = $props();

  let courses = $state<StudentEnrolledCourse[]>([]);
  let isLoading = $state(true);
  let searchQuery = $state('');

  onMount(async () => {
    try {
      courses = await getStudentCourses();
    } catch {
      courses = [];
    } finally {
      isLoading = false;
    }
  });

  const filteredCourses = $derived(
    courses.filter(
      (c) =>
        c.code.toLowerCase().includes(searchQuery.toLowerCase()) ||
        c.title.toLowerCase().includes(searchQuery.toLowerCase())
    )
  );
</script>

<div class="student-courses-hub">
  <div class="hub-header">
    <div>
      <p class="eyebrow">MY COURSES</p>
      <h2 class="hub-title">Courses</h2>
    </div>

    <div class="search-wrap">
      <Search size={13} class="search-icon" />
      <input
        type="search"
        bind:value={searchQuery}
        placeholder="Search courses..."
        class="course-search-input"
        aria-label="Search enrolled courses"
      />
    </div>
  </div>

  {#if isLoading}
    <SkeletonCard lines={2} label="Loading courses..." />
  {:else if filteredCourses.length === 0}
    <div class="empty-hub-box">
      <BookOpen size={28} color="var(--color-driftwood)" />
      <p class="empty-title">No courses found</p>
      <p class="hint">
        {#if searchQuery}
          Try adjusting your search terms.
        {:else}
          Join a live lecture to view your enrolled courses.
        {/if}
      </p>
    </div>
  {:else}
    <div class="courses-grid">
      {#each filteredCourses as course (course.id)}
        <StudentCourseCard {course} onSelect={onSelectCourseFilter} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .student-courses-hub {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-12);
    margin-bottom: var(--spacing-14);
  }
  .hub-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--spacing-10);
    border-bottom: 1px solid var(--color-cork-border);
    padding-bottom: var(--spacing-10);
  }
  .hub-title {
    font-family: var(--font-display);
    font-size: 22px;
    color: var(--color-warm-cream);
    margin: 2px 0 0 0;
  }
  .search-wrap {
    position: relative;
    display: flex;
    align-items: center;
  }
  :global(.search-wrap .search-icon) {
    position: absolute;
    left: 10px;
    color: var(--color-driftwood);
  }
  .course-search-input {
    padding-left: 28px !important;
    font-size: 12px;
    min-width: 160px;
    margin: 0;
  }
  .courses-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: var(--spacing-10);
  }
  .empty-hub-box {
    padding: var(--spacing-20);
    text-align: center;
    border: 1px dashed var(--color-cork-border);
    border-radius: var(--radius-cards);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }
  .empty-title {
    font-size: 14px;
    color: var(--color-warm-cream);
    margin: 0;
  }
  @media (max-width: 540px) {
    .hub-header {
      flex-direction: column;
      align-items: stretch;
    }
    .search-wrap {
      width: 100%;
    }
    .course-search-input {
      width: 100%;
    }
    .courses-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
