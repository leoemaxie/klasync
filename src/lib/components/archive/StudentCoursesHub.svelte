<script lang="ts">
  import { onMount } from 'svelte';
  import type { StudentEnrolledCourse } from '$lib/types';
  import { getStudentCourses } from '$lib/api/courses';
  import { Search, BookOpen, X } from '@lucide/svelte';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
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

  const q = $derived(searchQuery.trim().toLowerCase());
  const filteredCourses = $derived(
    courses.filter(
      (c) =>
        !q ||
        c.code.toLowerCase().includes(q) ||
        c.title.toLowerCase().includes(q)
    )
  );
</script>

<div class="hub">
  <header class="head">
    <div>
      <p class="eyebrow-line">My courses</p>
      <h2>Courses</h2>
    </div>

    <div class="search">
      <Search size={15} />
      <input
        type="search"
        bind:value={searchQuery}
        placeholder="Search courses"
        aria-label="Search enrolled courses"
        autocomplete="off"
      />
      {#if searchQuery}
        <Button
          icon
          size="sm"
          class="clear"
          aria-label="Clear search"
          onclick={() => (searchQuery = '')}><X size={14} /></Button
        >
      {/if}
    </div>
  </header>

  {#if isLoading}
    <SkeletonCard lines={2} label="Loading courses…" />
  {:else if filteredCourses.length === 0}
    <EmptyState
      icon={BookOpen}
      title="No courses found"
      description={searchQuery
        ? 'Try different words, or clear the search.'
        : 'Join a live lecture to see your enrolled courses here.'}
    />
  {:else}
    <ul class="grid">
      {#each filteredCourses as course (course.id)}
        <li><StudentCourseCard {course} onSelect={onSelectCourseFilter} /></li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .hub {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 12px;
    padding-bottom: 14px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .eyebrow-line {
    margin: 0 0 2px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--text-accent);
  }
  .head h2 {
    margin: 0;
    font-size: 28px;
    font-weight: 500;
  }
  .search {
    position: relative;
    display: flex;
    align-items: center;
    width: min(320px, 100%);
  }
  .search :global(svg:first-child) {
    position: absolute;
    left: 13px;
    color: var(--text-tertiary);
    pointer-events: none;
  }
  .search input {
    width: 100%;
    height: var(--control-h);
    min-height: var(--control-h);
    margin: 0;
    padding: 0 36px 0 38px;
    border: 1px solid var(--border-default);
    border-radius: 999px;
    background: var(--surface-inset);
    color: var(--text-primary);
    font-family: var(--font-body);
    font-size: 13.5px;
    letter-spacing: 0;
    text-transform: none;
  }
  .search input::placeholder {
    color: var(--text-tertiary);
    font-style: normal;
  }
  .search input::-webkit-search-cancel-button {
    display: none;
  }
  .search input:focus {
    outline: none;
    border-color: var(--border-strong);
    box-shadow: var(--focus-ring);
  }
  .search :global(.clear) {
    position: absolute;
    right: 4px;
  }
  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    gap: 12px;
  }
</style>
