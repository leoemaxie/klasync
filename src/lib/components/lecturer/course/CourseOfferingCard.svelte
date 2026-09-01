<script lang="ts">
  import type { Course } from '$lib/types';
  import { Check, ArrowRight } from '@lucide/svelte';

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
  onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && onSelect(course)}
>
  <div class="card-top-row">
    <span class="course-badge">{course.code}</span>
    {#if isActive}
      <span class="active-badge"><Check size={11} /> ACTIVE</span>
    {/if}
  </div>

  <h3 class="course-card-title">{course.title}</h3>

  <div class="card-meta-row">
    <span class="tag session-tag">{course.academic_session || '2025/2026'}</span
    >
    <span class="tag semester-tag">{course.semester || 'Second Semester'}</span>
  </div>

  <div class="card-footer-row">
    {#if isActive}
      <span class="active-status">Active course</span>
    {:else}
      <span class="select-hint">Select <ArrowRight size={11} /></span>
    {/if}
  </div>
</div>

<style>
  .course-card {
    background: rgba(16, 9, 4, 0.5);
    border: 1px solid var(--color-cork-border, #40372e);
    border-radius: var(--radius-cards, 10px);
    padding: 14px 16px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 8px;
    transition:
      background 0.15s ease,
      border-color 0.15s ease;
    text-align: left;
    position: relative;
  }
  .course-card:hover {
    border-color: var(--color-driftwood, #6c5f51);
    background: rgba(56, 36, 22, 0.25);
  }
  .course-card.active {
    border-color: var(--color-warm-cream, #ffedd7);
    background: rgba(56, 36, 22, 0.45);
  }
  .card-top-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }
  .course-badge {
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    font-weight: 600;
    color: var(--color-driftwood, #6c5f51);
    letter-spacing: 0.12em;
    text-transform: uppercase;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .course-card.active .course-badge {
    color: var(--color-warm-cream, #ffedd7);
  }
  .active-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    font-family: var(--font-mono, monospace);
    font-weight: 600;
    letter-spacing: 0.08em;
    color: var(--color-warm-cream, #ffedd7);
    background: rgba(56, 36, 22, 0.7);
    border: 1px solid var(--color-cork-border, #40372e);
    padding: 2px 7px;
    border-radius: 3px;
    white-space: nowrap;
  }
  .course-card-title {
    font-family: var(--font-body, sans-serif);
    font-size: 14px;
    font-weight: 600;
    line-height: 1.35;
    color: var(--color-warm-cream, #ffedd7);
    margin: 0;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    word-break: break-word;
  }
  .card-meta-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-top: 2px;
  }
  .tag {
    font-size: 10px;
    padding: 2px 7px;
    border-radius: 3px;
    letter-spacing: 0.03em;
    font-weight: 500;
    white-space: nowrap;
  }
  .session-tag {
    background: rgba(255, 237, 215, 0.06);
    border: 1px solid var(--color-cork-border, #40372e);
    color: var(--color-driftwood, #6c5f51);
  }
  .semester-tag {
    background: rgba(255, 237, 215, 0.04);
    border: 1px solid var(--color-cork-border, #40372e);
    color: var(--color-warm-cream-dim, rgba(255, 237, 215, 0.8));
  }
  .card-footer-row {
    margin-top: auto;
    padding-top: 4px;
    border-top: 1px solid rgba(64, 55, 46, 0.3);
    display: flex;
    align-items: center;
    min-height: 18px;
  }
  .active-status {
    color: var(--color-warm-cream, #ffedd7);
    font-size: 10px;
    font-weight: 500;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    font-family: var(--font-mono, monospace);
  }
  .select-hint {
    color: var(--color-driftwood, #6c5f51);
    font-size: 10px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    transition: color 0.15s ease;
  }
  .course-card:hover .select-hint {
    color: var(--color-warm-cream, #ffedd7);
  }
</style>
