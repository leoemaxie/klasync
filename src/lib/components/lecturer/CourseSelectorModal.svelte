<script lang="ts">
  import type { Course } from '$lib/types';
  import { createCourse, getCourses, getCourseRoster } from '$lib/api/courses';
  import { triggerHaptic } from '$lib/native/haptics';
  import {
    Plus,
    BookOpen,
    Calendar,
    Check,
    X,
    Filter,
    Layers,
  } from '@lucide/svelte';

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
  let isSubmitting = $state(false);
  let formError = $state('');

  let newCode = $state('');
  let newTitle = $state('');
  let newSession = $state('2025/2026');
  let newSemester = $state('Second Semester');

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

  async function handleSelectCourse(course: Course) {
    triggerHaptic('selection');
    activeCourse = course;
    courseCode = course.code;
    courseTitle = course.title;
    academicSession = course.academic_session;
    semester = course.semester;
    isOpen = false;
    onCourseSelected?.(course);
  }

  async function handleCreateOffering(e: SubmitEvent) {
    e.preventDefault();
    if (!newCode.trim() || !newTitle.trim() || !newSession.trim() || !newSemester.trim()) {
      formError = 'All fields (Code, Title, Academic Session, Semester) are required.';
      triggerHaptic('error');
      return;
    }

    isSubmitting = true;
    formError = '';

    try {
      const created = await createCourse({
        code: newCode.trim().toUpperCase(),
        title: newTitle.trim(),
        academic_session: newSession.trim(),
        semester: newSemester.trim(),
      });

      triggerHaptic('success');
      courses = [created, ...courses];
      await handleSelectCourse(created);
      isCreating = false;
      newCode = '';
      newTitle = '';
    } catch (err: any) {
      formError = err?.message || 'Could not create course offering. It may already exist.';
      triggerHaptic('error');
    } finally {
      isSubmitting = false;
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      isOpen = false;
    }
  }
</script>

{#if isOpen}
  <div
    class="modal-backdrop"
    onclick={handleBackdropClick}
    role="dialog"
    aria-modal="true"
    aria-labelledby="modal-title"
  >
    <div class="panel modal-container">
      <div class="modal-header">
        <div class="header-left">
          <div class="eyebrow">
            <Layers size={14} color="var(--color-ember-accent)" style="display:inline-block; vertical-align:middle;" />
            COURSE OFFERING SWITCHER
          </div>
          <h2 id="modal-title">Manage Course Offerings</h2>
        </div>
        <button
          type="button"
          class="text close-btn"
          onclick={() => (isOpen = false)}
          aria-label="Close modal"
        >
          <X size={18} />
        </button>
      </div>

      {#if isCreating}
        <!-- Create Course Offering Form -->
        <form class="create-offering-form" onsubmit={handleCreateOffering}>
          <p class="section-lead">Add a new session/semester instance of a course</p>

          {#if formError}
            <div class="error-banner" role="alert">{formError}</div>
          {/if}

          <div class="twocol">
            <label for="offering-code">
              Course Code
              <input
                id="offering-code"
                type="text"
                bind:value={newCode}
                placeholder="e.g. MEE 541"
                required
              />
            </label>

            <label for="offering-title">
              Course Title
              <input
                id="offering-title"
                type="text"
                bind:value={newTitle}
                placeholder="e.g. Advanced Fluid Dynamics"
                required
              />
            </label>
          </div>

          <div class="twocol">
            <label for="offering-session">
              Academic Session
              <select id="offering-session" bind:value={newSession} class="select-input" required>
                <option value="2025/2026">2025/2026</option>
                <option value="2026/2027">2026/2027</option>
                <option value="2027/2028">2027/2028</option>
                <option value="2024/2025">2024/2025</option>
              </select>
            </label>

            <label for="offering-semester">
              Semester (Required)
              <select id="offering-semester" bind:value={newSemester} class="select-input" required>
                <option value="Second Semester">Second Semester</option>
                <option value="First Semester">First Semester</option>
                <option value="Harmattan">Harmattan</option>
                <option value="Rain">Rain</option>
                <option value="Summer">Summer</option>
              </select>
            </label>
          </div>

          <div class="form-actions-row">
            <button
              type="button"
              class="outline"
              onclick={() => {
                isCreating = false;
                formError = '';
              }}
              disabled={isSubmitting}
            >
              Cancel
            </button>
            <button type="submit" class="primary" disabled={isSubmitting}>
              {isSubmitting ? 'Creating...' : 'Create Offering'}
            </button>
          </div>
        </form>
      {:else}
        <!-- Course List & Selector -->
        <div class="filter-and-action-bar">
          <div class="session-filter-group">
            <Filter size={13} color="var(--color-driftwood)" />
            <span class="filter-label">Session:</span>
            <select bind:value={filterSession} class="filter-select">
              <option value="all">All Sessions</option>
              {#each availableSessions as s}
                <option value={s}>{s}</option>
              {/each}
            </select>
          </div>

          <button
            type="button"
            class="primary-bark add-offering-btn"
            onclick={() => {
              isCreating = true;
              formError = '';
            }}
          >
            <Plus size={14} />
            <span>New Course Offering</span>
          </button>
        </div>

        <div class="courses-grid-list">
          {#if filteredCourses.length > 0}
            {#each filteredCourses as course (course.id)}
              <div
                class="course-card"
                class:active={activeCourse?.id === course.id || (courseCode === course.code && academicSession === course.academic_session && semester === course.semester)}
                onclick={() => handleSelectCourse(course)}
                role="button"
                tabindex="0"
                onkeydown={(e) => e.key === 'Enter' && handleSelectCourse(course)}
              >
                <div class="card-top-row">
                  <span class="course-badge">{course.code}</span>
                  <div class="session-tags">
                    <span class="tag session-tag">{course.academic_session || '2025/2026'}</span>
                    <span class="tag semester-tag">{course.semester || 'Second Semester'}</span>
                  </div>
                </div>

                <h3 class="course-card-title">{course.title}</h3>

                <div class="card-footer-row">
                  <span class="status-indicator">
                    {#if activeCourse?.id === course.id || (courseCode === course.code && academicSession === course.academic_session)}
                      <span class="active-badge"><Check size={12} /> Active Offering</span>
                    {:else}
                      <span class="select-hint">Click to switch</span>
                    {/if}
                  </span>
                </div>
              </div>
            {/each}
          {:else}
            <div class="empty-courses-state">
              <p class="empty-title">No course offerings found</p>
              <p class="hint">Click "New Course Offering" above to add your first course offering.</p>
            </div>
          {/if}
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
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--spacing-18);
  }

  .modal-container {
    width: 100%;
    max-width: 680px;
    max-height: 85vh;
    overflow-y: auto;
    background: var(--color-walnut-shadow);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
    padding: var(--spacing-28);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-18);
    box-shadow: 0 20px 48px rgba(0, 0, 0, 0.7);
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    border-bottom: 1px solid var(--color-cork-border);
    padding-bottom: var(--spacing-14);
  }

  .header-left h2 {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--color-warm-cream);
    margin: 4px 0 0 0;
  }

  .close-btn {
    padding: 6px;
    color: var(--color-driftwood);
  }
  .close-btn:hover {
    color: var(--color-warm-cream);
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

  .filter-select,
  .select-input {
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    color: var(--color-warm-cream);
    padding: 6px 10px;
    font-size: 12px;
    border-radius: 4px;
    outline: none;
  }

  .select-input {
    width: 100%;
    margin-top: var(--spacing-8);
    padding: 8px 10px;
    font-size: 14px;
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
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-14);
    margin-top: var(--spacing-8);
  }

  .course-card {
    background: rgba(16, 9, 4, 0.45);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
    padding: var(--spacing-16);
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-10);
    transition: all 0.2s ease;
  }

  .course-card:hover {
    border-color: var(--color-warm-cream);
    background: rgba(24, 14, 7, 0.7);
  }

  .course-card.active {
    border-color: var(--color-ember-accent);
    background: rgba(36, 20, 10, 0.75);
    box-shadow: 0 0 0 1px var(--color-ember-accent);
  }

  .card-top-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
  }

  .course-badge {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    font-weight: 700;
    color: var(--color-ember-accent);
    letter-spacing: 0.1em;
  }

  .session-tags {
    display: flex;
    gap: 4px;
  }

  .tag {
    font-size: 9px;
    padding: 2px 6px;
    border-radius: 3px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    font-weight: 600;
  }

  .session-tag {
    background: rgba(255, 237, 215, 0.1);
    color: var(--color-driftwood);
  }

  .semester-tag {
    background: rgba(255, 237, 215, 0.15);
    color: var(--color-warm-cream);
  }

  .course-card-title {
    font-size: 15px;
    margin: 0;
    color: var(--color-warm-cream);
    font-weight: 500;
    line-height: 1.3;
  }

  .card-footer-row {
    margin-top: auto;
    font-size: 11px;
  }

  .active-badge {
    color: var(--color-ember-accent);
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-weight: 600;
    text-transform: uppercase;
    font-size: 10px;
    letter-spacing: 0.08em;
  }

  .select-hint {
    color: var(--color-driftwood);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .empty-courses-state {
    grid-column: 1 / -1;
    padding: var(--spacing-28);
    text-align: center;
    border: 1px dashed var(--color-cork-border);
    border-radius: var(--radius-cards);
  }

  .empty-title {
    font-size: 15px;
    color: var(--color-warm-cream);
    margin-bottom: 4px;
  }

  .create-offering-form {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-16);
  }

  .section-lead {
    font-size: 13px;
    color: var(--color-driftwood);
    margin: 0;
  }

  .error-banner {
    background: rgba(220, 53, 69, 0.15);
    border: 1px solid rgba(220, 53, 69, 0.4);
    color: #ff8585;
    padding: 8px 12px;
    border-radius: 4px;
    font-size: 12px;
  }

  .form-actions-row {
    display: flex;
    justify-content: flex-end;
    gap: var(--spacing-12);
    margin-top: var(--spacing-8);
  }

  @media (max-width: 600px) {
    .courses-grid-list {
      grid-template-columns: 1fr;
    }
  }
</style>
