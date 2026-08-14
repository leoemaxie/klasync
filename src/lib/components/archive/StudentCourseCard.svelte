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
    <span class="semester-pill">{course.semester || 'Semester'}</span>
  </div>

  <h3 class="course-name">{course.title}</h3>

  <div class="lecturer-row">
    <User size={12} color="var(--color-driftwood)" />
    <span class="lecturer-name">{course.lecturer_name || 'Faculty'}</span>
  </div>

  <div class="stats-row">
    <div class="stat-pill">
      <BookOpen size={11} /><span
        >{course.session_count}
        {course.session_count === 1 ? 'Lecture' : 'Lectures'}</span
      >
    </div>
    <div class="stat-pill success">
      <CheckCircle2 size={11} /><span>Enrolled</span>
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
    gap: var(--spacing-10);
    padding: var(--spacing-14);
    background: rgba(16, 9, 4, 0.5);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
  }
  .student-course-card:hover {
    border-color: var(--color-ember-accent);
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
    font-size: 9px;
    background: rgba(255, 237, 215, 0.1);
    color: var(--color-warm-cream);
    padding: 2px 6px;
    border-radius: 4px;
    text-transform: uppercase;
  }
  .course-name {
    font-family: var(--font-display);
    font-size: 16px;
    margin: 0;
    color: var(--color-warm-cream);
    line-height: 1.3;
  }
  .lecturer-row {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--color-driftwood);
  }
  .stats-row {
    display: flex;
    gap: 6px;
    margin-top: auto;
  }
  .stat-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    color: var(--color-driftwood);
    background: rgba(16, 9, 4, 0.4);
    padding: 3px 6px;
    border-radius: 4px;
  }
  .stat-pill.success {
    color: #4ade80;
  }
  .view-btn {
    padding: 6px 10px;
    font-size: 10px;
    text-transform: uppercase;
    min-height: 40px;
  }
</style>
