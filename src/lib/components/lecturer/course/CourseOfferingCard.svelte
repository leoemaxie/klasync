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
      <span class="select-hint"
        >Select <ArrowRight size={11} /></span
      >
    {/if}
  </div>
</div>

<style>
  .course-card {
    background: rgba(24, 14, 8, 0.6);
    border: 1px solid var(--color-cork-border, #40372e);
    border-radius: var(--radius-cards, 10px);
    padding: 14px 16px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 8px;
    transition:
      background 0.15s ease,
      border-color 0.15s ease,
      transform 0.15s ease;
    text-align: left;
    position: relative;
  }
  .course-card:hover {
    border-color: var(--color-driftwood, #b8a794);
    background: rgba(36, 22, 12, 0.8);
    transform: translateY(-1px);
  }
  .course-card.active {
    border-color: var(--color-ember-accent, #dc5000);
    background: rgba(45, 24, 12, 0.85);
    box-shadow: 0 0 0 1px var(--color-ember-accent, #dc5000);
  }
  .card-top-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }
  .course-badge {
    font-family: var(--font-mono, monospace);
    font-size: 13px;
    font-weight: 700;
    color: var(--color-ember-accent, #dc5000);
    letter-spacing: 0.08em;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .active-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    font-family: var(--font-mono, monospace);
    font-weight: 700;
    letter-spacing: 0.06em;
    color: #e4e0d4;
    background: rgba(220, 80, 0, 0.25);
    border: 1px solid var(--color-ember-accent, #dc5000);
    padding: 2px 6px;
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
    background: rgba(255, 237, 215, 0.08);
    border: 1px solid rgba(255, 237, 215, 0.12);
    color: var(--color-driftwood, #b8a794);
  }
  .semester-tag {
    background: rgba(255, 237, 215, 0.05);
    border: 1px solid rgba(255, 237, 215, 0.1);
    color: var(--color-warm-cream-dim, rgba(255, 237, 215, 0.8));
  }
  .card-footer-row {
    margin-top: auto;
    padding-top: 4px;
    border-top: 1px solid rgba(64, 55, 46, 0.4);
    display: flex;
    align-items: center;
    min-height: 18px;
  }
  .active-status {
    color: var(--color-ember-accent, #dc5000);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.03em;
  }
  .select-hint {
    color: var(--color-driftwood, #b8a794);
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
