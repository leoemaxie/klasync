<script lang="ts">
  import type { ClaimRecord } from '$lib/api';
  import { CheckCircle2, ChevronRight } from '@lucide/svelte';
  import { triggerHaptic } from '$lib/native/haptics';

  let {
    claim,
    isSelected = false,
    onSelect,
  }: {
    claim: ClaimRecord;
    isSelected?: boolean;
    onSelect: (claim: ClaimRecord) => void;
  } = $props();

  function handleClick() {
    triggerHaptic('light');
    onSelect(claim);
  }
</script>

<button
  type="button"
  class="claim-item-card"
  class:selected={isSelected}
  onclick={handleClick}
  aria-current={isSelected ? 'true' : undefined}
>
  <div class="item-head">
    <span class="course-code-tag">{claim.course_code}</span>
    <span class="date-tag">{claim.date}</span>
  </div>

  <h3 class="item-title">{claim.session_title}</h3>

  <div class="item-footer">
    <div class="item-badges">
      <span class="badge verified">
        <CheckCircle2 size={10} /> Verified
      </span>
      {#if claim.academic_session}
        <span class="badge session">{claim.academic_session}</span>
      {/if}
    </div>
    <ChevronRight size={14} class="item-arrow" />
  </div>
</button>

<style>
  .claim-item-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: var(--spacing-12);
    background: rgba(16, 9, 4, 0.4);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    text-align: left;
    cursor: pointer;
    transition: all 0.2s ease;
    width: 100%;
    min-height: 48px;
  }
  .claim-item-card:hover {
    border-color: var(--color-warm-cream);
    background: rgba(255, 237, 215, 0.03);
  }
  .claim-item-card.selected {
    border-color: var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.08);
  }
  .item-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .course-code-tag {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    font-weight: 700;
    color: var(--color-ember-accent);
  }
  .date-tag {
    font-size: 10px;
    color: var(--color-driftwood);
  }
  .item-title {
    font-family: var(--font-display);
    font-size: 14px;
    margin: 0;
    color: var(--color-warm-cream);
    line-height: 1.3;
  }
  .item-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 2px;
  }
  .item-badges {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .badge {
    font-size: 9px;
    padding: 2px 6px;
    border-radius: 3px;
    text-transform: uppercase;
  }
  .badge.verified {
    background: rgba(74, 222, 128, 0.1);
    color: #4ade80;
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }
  .badge.session {
    background: rgba(255, 237, 215, 0.08);
    color: var(--color-driftwood);
  }
  :global(.item-arrow) {
    color: var(--color-driftwood);
  }
  .claim-item-card.selected :global(.item-arrow) {
    color: var(--color-ember-accent);
  }
</style>
