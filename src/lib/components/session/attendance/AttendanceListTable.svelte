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

<div class="attendance-table-wrap">
  <table class="attendance-table">
    <thead>
      <tr>
        <th>STUDENT / GUEST</th>
        <th>MATRICULATION</th>
        <th>VERIFICATION</th>
        <th>JOINED</th>
        <th>HEARTBEATS</th>
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

<style>
  .attendance-table-wrap {
    overflow-x: auto;
    background: rgba(16, 9, 4, 0.45);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
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
    color: var(--color-driftwood);
    border-bottom: 1px solid var(--color-cork-border);
    background: rgba(8, 4, 2, 0.4);
  }
  .attendance-table td {
    padding: 12px 16px;
    border-bottom: 1px solid rgba(64, 55, 46, 0.4);
    color: var(--color-warm-cream);
  }
  .name-cell {
    display: flex;
    align-items: center;
    gap: 10px;
    font-weight: 500;
  }
  .avatar {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: rgba(255, 237, 215, 0.12);
    color: var(--color-warm-cream);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    font-weight: 700;
    flex-shrink: 0;
  }
  .matric-cell {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    color: var(--color-driftwood);
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
  }
  .status-pill.verified {
    background: rgba(40, 167, 69, 0.15);
    color: #4ade80;
  }
  .status-pill.provisional {
    background: rgba(220, 80, 0, 0.15);
    color: var(--color-ember-accent);
  }
  .time-cell,
  .heartbeats-cell {
    color: var(--color-driftwood);
    font-size: 11px;
  }
</style>
