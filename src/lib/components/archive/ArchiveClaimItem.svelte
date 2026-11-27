<script lang="ts">
  import type { ClaimRecord } from '$lib/api';
  import { Headphones, Play, CircleCheck } from '@lucide/svelte';
  import { triggerHaptic } from '$lib/native/haptics';
  import { clock } from '$lib/studio/time';
  import { getResume, progressOf } from '$lib/studio/resume';

  let {
    claim,
    isSelected = false,
    resumeVersion = 0,
    onSelect,
  }: {
    claim: ClaimRecord;
    isSelected?: boolean;
    /** Bump to re-read the saved position (it changes while a lecture is open). */
    resumeVersion?: number;
    onSelect: (claim: ClaimRecord) => void;
  } = $props();

  const resume = $derived.by(() => {
    void resumeVersion;
    return getResume(claim.id);
  });
  const progress = $derived(progressOf(resume));
</script>

<button
  type="button"
  class="claim"
  class:selected={isSelected}
  aria-current={isSelected ? 'true' : undefined}
  onclick={() => {
    triggerHaptic('light');
    onSelect(claim);
  }}
>
  <span class="top">
    <span class="course">{claim.course_code}</span>
    <span class="date">{claim.date}</span>
  </span>

  <span class="title">{claim.session_title}</span>

  <span class="chips">
    {#if claim.attended === true}
      <span class="chip ok"><CircleCheck size={12} /> Attended</span>
    {:else if claim.attended === false}
      <span class="chip">Not attended</span>
    {/if}
    {#if claim.has_audio}
      <span class="chip"><Headphones size={12} /> Audio</span>
    {/if}
    {#if resume}
      <span class="chip resume">
        <Play size={11} fill="currentColor" /> Resume {clock(resume.t)}
      </span>
    {/if}
  </span>

  {#if progress != null}
    <span class="progress" aria-hidden="true">
      <i style:width="{progress * 100}%"></i>
    </span>
  {/if}
</button>

<style>
  .claim {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
    min-height: 0;
    padding: 14px 14px 12px;
    background: var(--surface-inset);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    color: var(--text-primary);
    text-align: left;
    font-family: var(--font-body);
    letter-spacing: 0;
    text-transform: none;
    white-space: normal;
    overflow: hidden;
    transition:
      border-color var(--motion-fast) var(--ease-out),
      background var(--motion-fast) var(--ease-out),
      transform var(--motion-fast) var(--ease-out);
  }
  .claim:hover {
    border-color: var(--border-strong);
    background: var(--surface-1);
  }
  .claim:active {
    transform: scale(0.992);
  }
  .claim.selected {
    border-color: var(--border-accent);
    background: var(--surface-accent);
  }
  .claim:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
  .top {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
  }
  .course {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.08em;
    color: var(--text-accent);
  }
  .date {
    font-size: 12px;
    color: var(--text-secondary);
  }
  .title {
    font-family: var(--font-display);
    font-size: 17px;
    line-height: 1.25;
    color: var(--text-primary);
    overflow-wrap: anywhere;
    text-wrap: balance;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 22px;
    padding: 0 9px;
    border: 1px solid var(--border-default);
    border-radius: 999px;
    font-size: 11.5px;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }
  .chip.ok {
    color: #7ddc9f;
    border-color: rgba(125, 220, 159, 0.3);
  }
  .chip.resume {
    color: var(--text-primary);
    border-color: var(--border-accent);
    background: var(--surface-accent);
  }
  .chip.resume :global(svg) {
    color: var(--text-accent);
  }
  .progress {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 3px;
    background: rgba(255, 237, 215, 0.08);
  }
  .progress i {
    display: block;
    height: 100%;
    background: var(--color-ember-accent);
  }
</style>
