<script lang="ts">
  import type { Course } from '$lib/types';
  import { triggerHaptic } from '$lib/native/haptics';
  import { Plus, X, Filter, Layers } from '@lucide/svelte';
  import CourseOfferingCard from './course/CourseOfferingCard.svelte';
  import CreateCourseOfferingForm from './course/CreateCourseOfferingForm.svelte';

  let {
    isOpen = $bindable(false),
    courses = $bindable([]),
    activeCourse = $bindable(null),
    courseCode = $bindable(''),
    courseTitle = $bindable(''),
    academicSession = $bindable('2025/2026'),
    semester = $bindable('Second Semester'),
    onCourseSelected,
  }: {
    isOpen: boolean;
    courses: Course[];
    activeCourse: Course | null;
    courseCode: string;
    courseTitle: string;
    academicSession: string;
    semester: string;
    onCourseSelected?: (course: Course) => void;
  } = $props();

  let isCreating = $state(false);
  let filterSession = $state('all');

  const availableSessions = $derived.by(() => {
    const set = new Set<string>(['2025/2026', '2026/2027']);
    for (const c of courses) {
      if (c.academic_session) set.add(c.academic_session);
    }
    return Array.from(set).sort();
  });

  const filteredCourses = $derived(
    filterSession === 'all'
      ? courses
      : courses.filter((c) => c.academic_session === filterSession)
  );

  function handleSelectCourse(course: Course) {
    triggerHaptic('light');
    activeCourse = course;
    courseCode = course.code;
    courseTitle = course.title;
    academicSession = course.academic_session;
    semester = course.semester;
    isOpen = false;
    onCourseSelected?.(course);
  }

  function handleCreated(created: Course) {
    courses = [created, ...courses];
    handleSelectCourse(created);
    isCreating = false;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape' && isOpen) {
      isOpen = false;
    }
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

{#if isOpen}
  <div
    class="modal-backdrop"
    onclick={(e) => e.target === e.currentTarget && (isOpen = false)}
    role="dialog"
    aria-modal="true"
    aria-labelledby="course-modal-title"
  >
    <div class="panel modal-container">
      <div class="sheet-drag-handle" aria-hidden="true"></div>

      <div class="modal-header">
        <div>
          <div class="eyebrow">
            <Layers
              size={14}
              style="display:inline-block; vertical-align:middle;"
            /> COURSE OFFERING SWITCHER
          </div>
          <h2 id="course-modal-title">Manage Course Offerings</h2>
        </div>
        <button
          type="button"
          class="text close-btn"
          onclick={() => (isOpen = false)}
          aria-label="Close modal"><X size={18} /></button
        >
      </div>

      {#if isCreating}
        <CreateCourseOfferingForm
          onCreated={handleCreated}
          onCancel={() => (isCreating = false)}
        />
      {:else}
        <div class="filter-and-action-bar">
          <div class="session-filter-group">
            <Filter size={13} />
            <span>Session:</span>
            <select bind:value={filterSession} class="filter-select">
              <option value="all">All Sessions</option>
              {#each availableSessions as s}<option value={s}>{s}</option
                >{/each}
            </select>
          </div>
          <button
            type="button"
            class="primary-bark add-offering-btn"
            onclick={() => (isCreating = true)}
          >
            <Plus size={14} /> <span>New Course Offering</span>
          </button>
        </div>

        <div class="courses-grid-list">
          {#each filteredCourses as course (course.id)}
            <CourseOfferingCard
              {course}
              isActive={activeCourse?.id === course.id ||
                (courseCode === course.code &&
                  academicSession === course.academic_session)}
              onSelect={handleSelectCourse}
            />
          {/each}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 200;
    background: rgba(8, 4, 2, 0.82);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--spacing-18, 18px);
  }
  .modal-container {
    width: 100%;
    max-width: 760px;
    max-height: 85vh;
    overflow-y: auto;
    padding: var(--spacing-24, 24px);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-16, 16px);
    border-radius: var(--radius-cards, 12px);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.8);
    position: relative;
  }
  .sheet-drag-handle {
    display: none;
    width: 36px;
    height: 4px;
    background: rgba(255, 237, 215, 0.25);
    border-radius: 9999px;
    margin: 0 auto -6px;
  }
  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    border-bottom: 1px solid var(--color-cork-border);
    padding-bottom: var(--spacing-12);
  }
  .modal-header h2 {
    font-family: var(--font-display);
    font-size: 22px;
    color: var(--color-warm-cream);
    margin: 2px 0 0 0;
  }
  .close-btn {
    padding: 6px;
    color: var(--color-driftwood);
  }
  .filter-and-action-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--spacing-12);
  }
  .session-filter-group {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--color-driftwood);
    text-transform: uppercase;
  }
  .filter-select {
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    color: var(--color-warm-cream);
    padding: 5px 8px;
    font-size: 11px;
    border-radius: 4px;
    outline: none;
  }
  .add-offering-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    padding: 6px 14px;
  }
  .courses-grid-list {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--spacing-12);
    margin-top: var(--spacing-4);
  }

  @media (max-width: 640px) {
    .modal-backdrop {
      align-items: flex-end;
      padding: 0;
    }
    .modal-container {
      border-bottom-left-radius: 0;
      border-bottom-right-radius: 0;
      border-top-left-radius: 18px;
      border-top-right-radius: 18px;
      max-height: 88vh;
      padding: 14px 16px calc(24px + env(safe-area-inset-bottom, 0px));
      gap: 14px;
      animation: slideUp 0.25s cubic-bezier(0.16, 1, 0.3, 1);
    }
    .sheet-drag-handle {
      display: block;
    }
    .modal-header h2 {
      font-size: 18px;
    }
    .courses-grid-list {
      grid-template-columns: 1fr;
      gap: 10px;
    }
    .filter-and-action-bar {
      gap: 8px;
    }
  }

  @keyframes slideUp {
    from {
      transform: translateY(100%);
    }
    to {
      transform: translateY(0);
    }
  }
</style>
