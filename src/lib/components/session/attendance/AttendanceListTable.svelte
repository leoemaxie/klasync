<script lang="ts">
  import type { Participant } from '$lib/types';
  import { ShieldCheck, UserX, Clock, Activity } from '@lucide/svelte';

  let {
    participants = [],
  }: {
    participants: Participant[];
  } = $props();

  function getInitials(name: string): string {
    if (!name) return '?';
    const parts = name.trim().split(/\s+/);
    if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
    return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
  }

  function formatTime(timestamp?: string): string {
    if (!timestamp) return 'Recent';
    try {
      const date = new Date(timestamp);
      return isNaN(date.getTime())
        ? 'Recent'
        : date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    } catch {
      return 'Recent';
    }
  }
</script>

<!-- Desktop Table View -->
<div class="attendance-table-wrap desktop-only">
  <table class="attendance-table">
    <thead>
      <tr>
        <th>NAME</th>
        <th>MATRIC</th>
        <th>STATUS</th>
        <th>JOINED</th>
        <th>CHECK-INS</th>
      </tr>
    </thead>
    <tbody>
      {#each participants as p (p.id || p.matric)}
        <tr class="attendee-row" class:verified={p.verified}>
          <td class="name-cell">
            <div class="avatar">{getInitials(p.name)}</div>
            <span class="student-name">{p.name}</span>
          </td>
          <td class="matric-cell">{p.matric}</td>
          <td>
            {#if p.verified}
              <span class="status-pill verified"
                ><ShieldCheck size={12} /> Verified</span
              >
            {:else}
              <span class="status-pill provisional"
                ><UserX size={12} /> Provisional</span
              >
            {/if}
          </td>
          <td class="time-cell"><Clock size={11} /> {formatTime(p.joinedAt)}</td
          >
          <td class="heartbeats-cell"
            ><Activity size={11} /> {p.heartbeats || 0}</td
          >
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<!-- Mobile Cards List View -->
<div class="attendance-mobile-cards mobile-only">
  {#each participants as p (p.id || p.matric)}
    <div class="attendee-card" class:verified={p.verified}>
      <div class="card-header">
        <div class="attendee-info">
          <div class="avatar">{getInitials(p.name)}</div>
          <div class="text-info">
            <span class="student-name">{p.name}</span>
            <span class="student-matric">{p.matric}</span>
          </div>
        </div>
        {#if p.verified}
          <span class="status-pill verified"
            ><ShieldCheck size={11} /> Verified</span
          >
        {:else}
          <span class="status-pill provisional"
            ><UserX size={11} /> Provisional</span
          >
        {/if}
      </div>
      <div class="card-meta">
        <span class="meta-item"
          ><Clock size={11} /> {formatTime(p.joinedAt)}</span
        >
        <span class="meta-divider">·</span>
        <span class="meta-item"
          ><Activity size={11} /> {p.heartbeats || 0} check-ins</span
        >
      </div>
    </div>
  {/each}
</div>

<style>
  .desktop-only {
    display: block;
  }
  .mobile-only {
    display: none;
  }

  .attendance-table-wrap {
    overflow-x: auto;
    background: rgba(16, 9, 4, 0.45);
    border: 1px solid var(--color-cork-border, #40372e);
    border-radius: var(--radius-cards, 8px);
  }
  .attendance-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
    text-align: left;
  }
  .attendance-table th {
    padding: 12px 16px;
    font-size: 10px;
    letter-spacing: 0.1em;
    color: var(--color-driftwood, #b8a794);
    border-bottom: 1px solid var(--color-cork-border, #40372e);
    background: rgba(8, 4, 2, 0.4);
    text-transform: uppercase;
    font-weight: 600;
  }
  .attendance-table td {
    padding: 12px 16px;
    border-bottom: 1px solid rgba(64, 55, 46, 0.4);
    color: var(--color-warm-cream, #ffedd7);
  }
  .name-cell {
    display: flex;
    align-items: center;
    gap: 10px;
    font-weight: 500;
  }
  .avatar {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: rgba(255, 237, 215, 0.12);
    color: var(--color-warm-cream, #ffedd7);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    font-weight: 700;
    flex-shrink: 0;
    border: 1px solid rgba(255, 237, 215, 0.15);
  }
  .matric-cell {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    color: var(--color-driftwood, #b8a794);
  }
  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    padding: 3px 8px;
    border-radius: 999px;
    text-transform: uppercase;
    font-weight: 600;
    white-space: nowrap;
  }
  .status-pill.verified {
    background: rgba(40, 167, 69, 0.15);
    border: 1px solid rgba(74, 222, 128, 0.3);
    color: #4ade80;
  }
  .status-pill.provisional {
    background: rgba(220, 80, 0, 0.15);
    border: 1px solid rgba(220, 80, 0, 0.3);
    color: var(--color-ember-accent, #dc5000);
  }
  .time-cell,
  .heartbeats-cell {
    color: var(--color-driftwood, #b8a794);
    font-size: 11px;
  }

  /* Mobile cards view */
  .attendance-mobile-cards {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .attendee-card {
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border, #40372e);
    border-radius: 8px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .attendee-card.verified {
    border-left: 3px solid #4ade80;
  }
  .attendee-card:not(.verified) {
    border-left: 3px solid var(--color-ember-accent, #dc5000);
  }
  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .attendee-info {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .text-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .student-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--color-warm-cream, #ffedd7);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .student-matric {
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    color: var(--color-driftwood, #b8a794);
  }
  .card-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--color-driftwood, #b8a794);
    padding-top: 4px;
    border-top: 1px solid rgba(64, 55, 46, 0.3);
  }
  .meta-item {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .meta-divider {
    color: rgba(255, 237, 215, 0.2);
  }

  @media (max-width: 640px) {
    .desktop-only {
      display: none;
    }
    .mobile-only {
      display: flex;
    }
  }
</style>
