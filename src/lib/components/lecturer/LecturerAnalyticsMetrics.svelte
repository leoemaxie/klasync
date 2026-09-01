<script lang="ts">
  import type { CourseAnalyticsSummary, AttendanceAnomaly } from '$lib/api';
  import type { Participant } from '$lib/types';

  let {
    summary = null,
    anomalies = [],
    participants = [],
    courseCode = '',
  }: {
    summary?: CourseAnalyticsSummary | null;
    anomalies?: AttendanceAnomaly[];
    participants?: Participant[];
    courseCode?: string;
  } = $props();

  function formatPercent(val: number | null | undefined): string {
    if (val === null || val === undefined || isNaN(val)) return '0%';
    const num = Number(val);
    return Number.isInteger(num) ? `${num}%` : `${num.toFixed(2)}%`;
  }
</script>

<div class="metrics-grid">
  <div class="metric-card">
    <span class="m-val"
      >{summary
        ? formatPercent(summary.avg_attendance_percentage)
        : `${participants.length}`}</span
    >
    <span class="m-lbl"
      >{summary ? 'Average attendance' : 'Active attendees'}</span
    >
  </div>
  <div class="metric-card">
    <span class="m-val">
      {summary
        ? formatPercent(summary.roster_verification_match_rate)
        : `${participants.filter((p) => p.verified).length}`}
    </span>
    <span class="m-lbl">{summary ? 'Match rate' : 'Verified students'}</span>
  </div>
  <div class="metric-card">
    <span class="m-val"
      >{summary ? summary.total_sessions : courseCode ? '1' : '0'}</span
    >
    <span class="m-lbl">Total sessions</span>
  </div>
  <div class="metric-card">
    <span class="m-val warning"
      >{summary ? summary.total_anomalies_flagged : anomalies.length}</span
    >
    <span class="m-lbl">Flags</span>
  </div>
</div>

<style>
  .metrics-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--spacing-12);
  }
  .metric-card {
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
    .metrics-grid {
      grid-template-columns: 1fr 1fr;
    }
  }
</style>
