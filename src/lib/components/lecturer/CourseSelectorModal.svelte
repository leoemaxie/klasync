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
    filterSession === 'all' ? courses : courses.filter((c) => c.academic_session === filterSession)
  );

  function handleSelectCourse(course: Course) {
    triggerHaptic('selection');
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
</script>

{#if isOpen}
  <div class="modal-backdrop" onclick={(e) => e.target === e.currentTarget && (isOpen = false)} role="dialog" aria-modal="true">
    <div class="panel modal-container">
      <div class="modal-header">
        <div>
          <div class="eyebrow"><Layers size={14} style="display:inline-block; vertical-align:middle;" /> COURSE OFFERING SWITCHER</div>
          <h2>Manage Course Offerings</h2>
        </div>
        <button type="button" class="text close-btn" onclick={() => (isOpen = false)} aria-label="Close"><X size={18} /></button>
      </div>

      {#if isCreating}
        <CreateCourseOfferingForm onCreated={handleCreated} onCancel={() => (isCreating = false)} />
      {:else}
        <div class="filter-and-action-bar">
          <div class="session-filter-group">
            <Filter size={13} />
            <span>Session:</span>
            <select bind:value={filterSession} class="filter-select">
              <option value="all">All Sessions</option>
              {#each availableSessions as s}<option value={s}>{s}</option>{/each}
            </select>
          </div>
          <button type="button" class="primary-bark add-offering-btn" onclick={() => (isCreating = true)}>
            <Plus size={14} /> <span>New Course Offering</span>
          </button>
        </div>

        <div class="courses-grid-list">
          {#each filteredCourses as course (course.id)}
            <CourseOfferingCard
              {course}
              isActive={activeCourse?.id === course.id || (courseCode === course.code && academicSession === course.academic_session)}
              onSelect={handleSelectCourse}
            />
          {/each}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .modal-backdrop { position: fixed; inset: 0; z-index: 200; background: rgba(8, 4, 2, 0.82); backdrop-filter: blur(8px); display: flex; align-items: center; justify-content: center; padding: var(--spacing-18); }
  .modal-container { width: 100%; max-width: 680px; max-height: 85vh; overflow-y: auto; padding: var(--spacing-24); display: flex; flex-direction: column; gap: var(--spacing-16); }
  .modal-header { display: flex; justify-content: space-between; align-items: flex-start; border-bottom: 1px solid var(--color-cork-border); padding-bottom: var(--spacing-12); }
  .modal-header h2 { font-family: var(--font-display); font-size: 22px; color: var(--color-warm-cream); margin: 2px 0 0 0; }
  .close-btn { padding: 6px; color: var(--color-driftwood); }
  .filter-and-action-bar { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: var(--spacing-12); }
  .session-filter-group { display: flex; align-items: center; gap: 6px; font-size: 11px; color: var(--color-driftwood); text-transform: uppercase; }
  .filter-select { background: rgba(16, 9, 4, 0.6); border: 1px solid var(--color-cork-border); color: var(--color-warm-cream); padding: 5px 8px; font-size: 11px; border-radius: 4px; outline: none; }
  .add-offering-btn { display: inline-flex; align-items: center; gap: 6px; font-size: 11px; padding: 6px 14px; }
  .courses-grid-list { display: grid; grid-template-columns: 1fr 1fr; gap: var(--spacing-12); margin-top: var(--spacing-4); }
  @media (max-width: 600px) { .courses-grid-list { grid-template-columns: 1fr; } }
</style>
