<script lang="ts">
  import type { StudentEnrolledCourse } from '$lib/types';
  import { User, BookOpen, CheckCircle2 } from '@lucide/svelte';
  import { triggerHaptic } from '$lib/native/haptics';

  let {
    course,
    onSelect,
  }: {
    course: StudentEnrolledCourse;
    onSelect?: (code: string) => void;
  } = $props();
</script>

<div class="student-course-card panel">
  <div class="card-head">
    <span class="course-code-badge">{course.code}</span>
    <span class="semester-pill">{course.semester || 'Second Semester'}</span>
  </div>

  <h3 class="course-name">{course.title}</h3>

  <div class="lecturer-row">
    <User size={13} color="var(--color-driftwood)" />
    <span class="lecturer-name"
      >{course.lecturer_name || 'Department Faculty'}</span
    >
  </div>

  <div class="stats-row">
    <div class="stat-pill">
      <BookOpen size={12} />
      <span
        >{course.session_count}
        {course.session_count === 1 ? 'Lecture' : 'Lectures'}</span
      >
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
      onSelect?.(course.code);
    }}
  >
    View Course Lectures
  </button>
</div>

<style>
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
  }
</style>
