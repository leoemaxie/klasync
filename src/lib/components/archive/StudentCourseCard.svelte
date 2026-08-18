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
    <User size={13} color="var(--color-driftwood)" />
    <span class="lecturer-name">{course.lecturer_name || 'Faculty Member'}</span>
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
      onSelect?.(course.code);
    }}
  >
    View Lectures
  </button>
</div>

<style>
  .student-course-card {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-10);
    padding: var(--spacing-14);
    background: rgba(16, 9, 4, 0.55);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
    transition: border-color 0.2s ease;
  }
  .student-course-card:hover {
    border-color: var(--color-warm-cream);
  }
  .card-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .course-code-badge {
    font-family: var(--font-mono, monospace);
    font-size: 14px;
    font-weight: 700;
    color: var(--color-ember-accent);
    letter-spacing: 0.05em;
  }
  .semester-pill {
    font-size: 10px;
    font-weight: 500;
    background: rgba(255, 237, 215, 0.09);
    color: var(--color-warm-cream-dim);
    padding: 2px 7px;
    border-radius: 4px;
    text-transform: uppercase;
  }
  .course-name {
    font-size: 16px;
    font-weight: 600;
    margin: 0;
    color: var(--color-warm-cream);
    line-height: 1.35;
    word-break: break-word;
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
    flex-wrap: wrap;
  }
  .stat-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--color-driftwood);
    background: rgba(16, 9, 4, 0.4);
    border: 1px solid var(--color-cork-border);
    padding: 3px 8px;
    border-radius: 4px;
  }
  .stat-pill.success {
    color: #4ade80;
    border-color: rgba(74, 222, 128, 0.2);
    background: rgba(74, 222, 128, 0.08);
  }
  .view-btn {
    padding: 8px 12px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    min-height: 38px;
  }
</style>
