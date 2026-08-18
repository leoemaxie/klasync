<script lang="ts">
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
    <span class="m-val">{totalCount}</span>
    <span class="m-lbl">Total Attendees</span>
  </div>

  <div class="summary-card">
    <span class="m-val">{matchRate}%</span>
    <span class="m-lbl">Roster Match Rate</span>
  </div>

  <div class="summary-card">
    <span class="m-val">{verifiedCount}</span>
    <span class="m-lbl">Verified Matches</span>
  </div>

  <div class="summary-card">
    <span class="m-val warning">{provisionalCount}</span>
    <span class="m-lbl">Provisional Guests</span>
  </div>
</div>

<style>
  .attendance-summary-cards {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--spacing-12);
  }
  .summary-card {
    display: flex;
    flex-direction: column;
    padding: var(--spacing-14);
    background: rgba(16, 9, 4, 0.5);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
    text-align: center;
  }
  .m-val {
    font-family: var(--font-display);
    font-size: 28px;
    color: var(--color-warm-cream);
  }
  .m-val.warning {
    color: var(--color-ember-accent);
  }
  .m-lbl {
    font-size: 10px;
    letter-spacing: 0.1em;
    color: var(--color-driftwood);
    margin-top: 4px;
    text-transform: uppercase;
  }
  @media (max-width: 768px) {
    .attendance-summary-cards {
      grid-template-columns: 1fr 1fr;
    }
  }
</style>
