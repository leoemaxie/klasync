<script lang="ts">
  import { Users, ShieldCheck, UserX, Activity } from '@lucide/svelte';

  let {
    totalCount = 0,
    verifiedCount = 0,
    provisionalCount = 0,
    matchRate = 0,
  }: {
    totalCount: number;
    verifiedCount: number;
    provisionalCount: number;
    matchRate: number;
  } = $props();
</script>

<div class="attendance-summary-cards">
  <div class="summary-card">
    <div class="card-icon total"><Users size={16} /></div>
    <div class="card-content">
      <span class="card-label">TOTAL ATTENDEES</span>
      <strong class="card-val">{totalCount}</strong>
    </div>
  </div>

  <div class="summary-card">
    <div class="card-icon verified"><ShieldCheck size={16} /></div>
    <div class="card-content">
      <span class="card-label">ROSTER VERIFIED</span>
      <strong class="card-val text-success">{verifiedCount}</strong>
    </div>
  </div>

  <div class="summary-card">
    <div class="card-icon provisional"><UserX size={16} /></div>
    <div class="card-content">
      <span class="card-label">PROVISIONAL GUESTS</span>
      <strong class="card-val text-warning">{provisionalCount}</strong>
    </div>
  </div>

  <div class="summary-card">
    <div class="card-icon rate"><Activity size={16} /></div>
    <div class="card-content">
      <span class="card-label">VERIFICATION RATE</span>
      <strong class="card-val">{matchRate}%</strong>
    </div>
  </div>
</div>

<style>
  .attendance-summary-cards {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--spacing-14);
  }
  .summary-card {
    display: flex;
    align-items: center;
    gap: var(--spacing-12);
    padding: var(--spacing-14) var(--spacing-16);
    background: rgba(16, 9, 4, 0.45);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
  }
  .card-icon {
    width: 34px;
    height: 34px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 237, 215, 0.08);
    color: var(--color-warm-cream);
    flex-shrink: 0;
  }
  .card-icon.verified {
    background: rgba(40, 167, 69, 0.15);
    color: #4ade80;
  }
  .card-icon.provisional {
    background: rgba(220, 80, 0, 0.15);
    color: var(--color-ember-accent);
  }
  .card-icon.rate {
    background: rgba(255, 237, 215, 0.12);
    color: var(--color-warm-cream);
  }
  .card-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .card-label {
    font-size: 10px;
    letter-spacing: 0.1em;
    color: var(--color-driftwood);
    text-transform: uppercase;
  }
  .card-val {
    font-size: 18px;
    font-family: var(--font-display);
    color: var(--color-warm-cream);
  }
  .text-success { color: #4ade80; }
  .text-warning { color: var(--color-ember-accent); }

  @media (max-width: 900px) {
    .attendance-summary-cards {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>
