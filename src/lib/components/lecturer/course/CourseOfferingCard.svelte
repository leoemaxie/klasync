<script lang="ts">
  import type { Course } from '$lib/types';
  import { Check } from '@lucide/svelte';

  let {
    course,
    isActive = false,
    onSelect,
  }: {
    course: Course;
    isActive: boolean;
    onSelect: (c: Course) => void;
  } = $props();
</script>

<div
  class="course-card"
  class:active={isActive}
  onclick={() => onSelect(course)}
  role="button"
  tabindex="0"
  onkeydown={(e) => e.key === 'Enter' && onSelect(course)}
>
  <div class="card-top-row">
    <span class="course-badge">{course.code}</span>
    <div class="session-tags">
      <span class="tag session-tag"
        >{course.academic_session || '2025/2026'}</span
      >
      <span class="tag semester-tag"
        >{course.semester || 'Second Semester'}</span
      >
    </div>
  </div>

  <h3 class="course-card-title">{course.title}</h3>

  <div class="card-footer-row">
    {#if isActive}
      <span class="active-badge"><Check size={12} /> Active Offering</span>
    {:else}
      <span class="select-hint">Click to switch</span>
    {/if}
  </div>
</div>

<style>
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
  }
  .select-hint {
    color: var(--color-driftwood);
    font-size: 10px;
    text-transform: uppercase;
  }
</style>
