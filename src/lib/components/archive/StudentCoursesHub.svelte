<script lang="ts">
  import { onMount } from 'svelte';
  import type { StudentEnrolledCourse } from '$lib/types';
  import { getStudentCourses, enrollStudentCourse } from '$lib/api/courses';
  import { triggerHaptic } from '$lib/native/haptics';
  import {
    GraduationCap,
    BookOpen,
    Calendar,
    User,
    Plus,
    CheckCircle2,
    Search,
  } from '@lucide/svelte';
  import SkeletonCard from '$lib/components/shared/SkeletonCard.svelte';

  let {
    onSelectCourseFilter,
  }: {
    onSelectCourseFilter?: (courseCode: string) => void;
  } = $props();

  let courses = $state<StudentEnrolledCourse[]>([]);
  let isLoading = $state(true);
  let searchQuery = $state('');
  let filterSession = $state('all');

  let isEnrollOpen = $state(false);
  let enrollCourseCode = $state('');
  let enrollError = $state('');
  let isEnrolling = $state(false);

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
    for (const c of courses) {
      if (c.academic_session) set.add(c.academic_session);
    }
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

  async function handleDirectEnroll(e: SubmitEvent) {
    e.preventDefault();
    if (!enrollCourseCode.trim()) return;

    isEnrolling = true;
    enrollError = '';

    try {
      await enrollStudentCourse(enrollCourseCode.trim());
      triggerHaptic('success');
      courses = await getStudentCourses();
      isEnrollOpen = false;
      enrollCourseCode = '';
    } catch (err: any) {
      enrollError = err?.message || 'Could not find course with that code.';
      triggerHaptic('error');
    } finally {
      isEnrolling = false;
    }
  }
</script>

<div class="student-courses-hub">
  <div class="hub-header">
    <div>
      <div class="eyebrow">
        <GraduationCap size={14} color="var(--color-ember-accent)" style="display:inline-block; vertical-align:middle;" />
        STUDENT COURSE DIRECTORY
      </div>
      <h2 class="hub-title">Enrolled Courses</h2>
      <p class="hint">Courses and lecture streams claimed across university lecturers</p>
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
          <option value="all">All Academic Sessions</option>
          {#each availableSessions as sess}
            <option value={sess}>{sess}</option>
          {/each}
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
            <span class="count-badge">{sessionCourses.length} {sessionCourses.length === 1 ? 'Course' : 'Courses'}</span>
          </div>

          <div class="courses-grid">
            {#each sessionCourses as course (course.id)}
              <div class="student-course-card panel">
                <div class="card-head">
                  <span class="course-code-badge">{course.code}</span>
                  <span class="semester-pill">{course.semester || 'Second Semester'}</span>
                </div>

                <h3 class="course-name">{course.title}</h3>

                <div class="lecturer-row">
                  <User size={13} color="var(--color-driftwood)" />
                  <span class="lecturer-name">{course.lecturer_name || 'Department Faculty'}</span>
                </div>

                <div class="stats-row">
                  <div class="stat-pill">
                    <BookOpen size={12} />
                    <span>{course.session_count} {course.session_count === 1 ? 'Lecture' : 'Lectures'}</span>
                  </div>
                  <div class="stat-pill success">
                    <CheckCircle2 size={12} />
                    <span>Enrolled</span>
                  </div>
                </div>

                <button
                  type="button"
                  class="primary outline full view-btn"
                  onclick={() => {
                    triggerHaptic('selection');
                    onSelectCourseFilter?.(course.code);
                  }}
                >
                  View Course Lectures
                </button>
              </div>
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
        Join live lectures with your matric number to auto-enroll and retain lecture archives across lecturers.
      </p>
    </div>
  {/if}
</div>

<style>
  .student-courses-hub {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-20);
    margin-bottom: var(--spacing-24);
  }

  .hub-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    flex-wrap: wrap;
    gap: var(--spacing-14);
    border-bottom: 1px solid var(--color-cork-border);
    padding-bottom: var(--spacing-16);
  }

  .hub-title {
    font-family: var(--font-display);
    font-size: 26px;
    color: var(--color-warm-cream);
    margin: 4px 0 2px 0;
  }

  .hub-controls {
    display: flex;
    align-items: center;
    gap: var(--spacing-10);
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
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 4px;
    color: var(--color-warm-cream);
    min-width: 220px;
    margin: 0;
  }

  .session-dropdown {
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    color: var(--color-warm-cream);
    padding: 6px 12px;
    font-size: 12px;
    border-radius: 4px;
    outline: none;
  }

  .sessions-stack {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-28);
  }

  .session-group-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: var(--spacing-12);
  }

  .session-heading {
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.1em;
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
    gap: var(--spacing-16);
  }

  .student-course-card {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-12);
    padding: var(--spacing-18);
    background: rgba(16, 9, 4, 0.5);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
    transition: all 0.2s ease;
  }

  .student-course-card:hover {
    border-color: var(--color-ember-accent);
    background: rgba(24, 14, 7, 0.7);
  }

  .card-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .course-code-badge {
    font-family: var(--font-mono, monospace);
    font-size: 13px;
    font-weight: 700;
    color: var(--color-ember-accent);
    letter-spacing: 0.1em;
  }

  .semester-pill {
    font-size: 10px;
    background: rgba(255, 237, 215, 0.12);
    color: var(--color-warm-cream);
    padding: 2px 8px;
    border-radius: 4px;
    text-transform: uppercase;
    font-weight: 600;
  }

  .course-name {
    font-family: var(--font-display);
    font-size: 17px;
    margin: 0;
    color: var(--color-warm-cream);
    line-height: 1.3;
  }

  .lecturer-row {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--color-driftwood);
  }

  .stats-row {
    display: flex;
    gap: 8px;
    margin-top: auto;
  }

  .stat-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--color-driftwood);
    background: rgba(16, 9, 4, 0.4);
    padding: 4px 8px;
    border-radius: 4px;
  }

  .stat-pill.success {
    color: #4ade80;
  }

  .view-btn {
    margin-top: var(--spacing-6);
    padding: 8px 12px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .empty-hub-box {
    padding: var(--spacing-41);
    text-align: center;
    border: 1px dashed var(--color-cork-border);
    border-radius: var(--radius-cards);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }

  .empty-title {
    font-size: 16px;
    color: var(--color-warm-cream);
    margin: 0;
  }
</style>
