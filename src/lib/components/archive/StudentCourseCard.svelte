<script lang="ts">
  import type { StudentEnrolledCourse } from '$lib/types';
  import { User, BookOpen, ArrowRight } from '@lucide/svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { triggerHaptic } from '$lib/native/haptics';

  let {
    course,
    onSelect,
  }: {
    course: StudentEnrolledCourse;
    onSelect?: (code: string) => void;
  } = $props();

  const n = $derived(course.session_count ?? 0);
</script>

<article class="course">
  <header>
    <span class="code">{course.code}</span>
    <span class="term">{course.semester || 'Semester'}</span>
  </header>

  <h3>{course.title}</h3>

  <p class="lecturer">
    <User size={14} />
    {course.lecturer_name || 'Faculty member'}
  </p>

  <div class="foot">
    <span class="count"
      ><BookOpen size={14} /> {n} {n === 1 ? 'lecture' : 'lectures'}</span
    >
    <Button
      variant="soft"
      size="sm"
      onclick={() => {
        triggerHaptic('selection');
        onSelect?.(course.code);
      }}
      aria-label="View lectures for {course.code}"
    >
      View <ArrowRight size={14} />
    </Button>
  </div>
</article>

<style>
  .course {
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: 100%;
    padding: 16px;
    background: var(--surface-inset);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    transition:
      border-color var(--motion-fast) var(--ease-out),
      background var(--motion-fast) var(--ease-out);
  }
  .course:hover {
    border-color: var(--border-strong);
    background: var(--surface-1);
  }
  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
  }
  .code {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 12.5px;
    font-weight: 600;
    letter-spacing: 0.08em;
    color: var(--text-accent);
  }
  .term {
    font-size: 12px;
    color: var(--text-secondary);
  }
  h3 {
    margin: 0;
    font-size: 20px;
    line-height: 1.2;
    text-wrap: balance;
  }
  .lecturer {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 0;
    font-size: 13px;
    color: var(--text-secondary);
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-top: auto;
    padding-top: 6px;
  }
  .count {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--text-secondary);
  }
</style>
