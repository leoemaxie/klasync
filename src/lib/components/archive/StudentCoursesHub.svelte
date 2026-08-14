<script lang="ts">
  import { onMount } from 'svelte';
  import type { StudentEnrolledCourse } from '$lib/types';
  import { getStudentCourses } from '$lib/api/courses';
  import { GraduationCap, Calendar, Search } from '@lucide/svelte';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';
  import StudentCourseCard from './StudentCourseCard.svelte';

  let {
    onSelectCourseFilter,
  }: {
    onSelectCourseFilter?: (courseCode: string) => void;
  } = $props();

  let courses = $state<StudentEnrolledCourse[]>([]);
  let isLoading = $state(true);
  let searchQuery = $state('');
  let filterSession = $state('all');

  onMount(async () => {
    try {
      courses = await getStudentCourses();
    } catch {
      courses = [];
    } finally {
      isLoading = false;
    }
  });

  const availableSessions = $derived.by(() => {
    const set = new Set<string>();
    for (const c of courses)
      if (c.academic_session) set.add(c.academic_session);
    return Array.from(set).sort().reverse();
  });

  const filteredCourses = $derived(
    courses.filter((c) => {
      const matchSearch =
        c.code.toLowerCase().includes(searchQuery.toLowerCase()) ||
        c.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        c.lecturer_name.toLowerCase().includes(searchQuery.toLowerCase());
      const matchSession =
        filterSession === 'all' || c.academic_session === filterSession;
      return matchSearch && matchSession;
    })
  );

  const groupedBySession = $derived.by(() => {
    const map = new Map<string, StudentEnrolledCourse[]>();
    for (const c of filteredCourses) {
      const sess = c.academic_session || '2025/2026';
      if (!map.has(sess)) map.set(sess, []);
      map.get(sess)!.push(c);
    }
    return Array.from(map.entries());
  });
</script>

<div class="student-courses-hub">
  <div class="hub-header">
    <div>
      <div class="eyebrow">
        <GraduationCap
          size={14}
          style="display:inline-block; vertical-align:middle;"
        /> STUDENT COURSE DIRECTORY
      </div>
      <h2 class="hub-title">Enrolled Courses</h2>
    </div>

    <div class="hub-controls">
      <div class="search-wrap">
        <Search size={14} class="search-icon" />
        <input
          type="search"
          bind:value={searchQuery}
          placeholder="Filter by course or lecturer..."
          class="course-search-input"
        />
      </div>
      {#if availableSessions.length > 1}
        <select bind:value={filterSession} class="session-dropdown">
          <option value="all">All Sessions</option>
          {#each availableSessions as sess}<option value={sess}>{sess}</option
            >{/each}
        </select>
      {/if}
    </div>
  </div>

  {#if isLoading}
    <SkeletonCard lines={3} label="Loading your enrolled courses..." />
  {:else if groupedBySession.length > 0}
    <div class="sessions-stack">
      {#each groupedBySession as [sessionName, sessionCourses] (sessionName)}
        <div class="session-group">
          <div class="session-group-header">
            <Calendar size={14} color="var(--color-ember-accent)" />
            <span class="session-heading">{sessionName} Academic Session</span>
            <span class="count-badge"
              >{sessionCourses.length}
              {sessionCourses.length === 1 ? 'Course' : 'Courses'}</span
            >
          </div>

          <div class="courses-grid">
            {#each sessionCourses as course (course.id)}
              <StudentCourseCard {course} onSelect={onSelectCourseFilter} />
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <div class="empty-hub-box">
      <GraduationCap size={32} color="var(--color-driftwood)" />
      <p class="empty-title">No Enrolled Courses Found</p>
      <p class="hint">
        Join live lectures with your matric number to auto-enroll and retain
        lecture archives across lecturers.
      </p>
    </div>
  {/if}
</div>

<style>
  .student-courses-hub {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-18);
    margin-bottom: var(--spacing-20);
  }
  .hub-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    flex-wrap: wrap;
    gap: var(--spacing-12);
    border-bottom: 1px solid var(--color-cork-border);
    padding-bottom: var(--spacing-14);
  }
  .hub-title {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--color-warm-cream);
    margin: 2px 0 0 0;
  }
  .hub-controls {
    display: flex;
    align-items: center;
    gap: var(--spacing-8);
    flex-wrap: wrap;
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
    padding-left: 32px !important;
    font-size: 12px;
    min-width: 200px;
    margin: 0;
  }
  .session-dropdown {
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    color: var(--color-warm-cream);
    padding: 5px 10px;
    font-size: 11px;
    border-radius: 4px;
    outline: none;
  }
  .sessions-stack {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-24);
  }
  .session-group-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: var(--spacing-10);
  }
  .session-heading {
    font-size: 12px;
    font-weight: 700;
    color: var(--color-warm-cream);
    text-transform: uppercase;
  }
  .count-badge {
    font-size: 10px;
    background: rgba(255, 237, 215, 0.1);
    color: var(--color-driftwood);
    padding: 2px 8px;
    border-radius: 999px;
  }
  .courses-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: var(--spacing-14);
  }
  .empty-hub-box {
    padding: var(--spacing-31);
    text-align: center;
    border: 1px dashed var(--color-cork-border);
    border-radius: var(--radius-cards);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }
  .empty-title {
    font-size: 15px;
    color: var(--color-warm-cream);
    margin: 0;
  }
</style>
