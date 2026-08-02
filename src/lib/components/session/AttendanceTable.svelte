<script lang="ts">
  import type { Participant } from "$lib/types";
  import { ShieldCheck, UserX, Clock } from "@lucide/svelte";

  let {
    filteredParticipants = [],
    totalCount = 0
  }: {
    filteredParticipants: Participant[];
    totalCount: number;
  } = $props();

  function getInitials(name: string): string {
    if (!name) return "?";
    const parts = name.trim().split(/\s+/);
    if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
    return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
  }

  function formatJoinedTime(timestamp?: string): string {
    if (!timestamp) return "Recent";
    try {
      const date = new Date(timestamp);
      return isNaN(date.getTime()) ? "Recent" : date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    } catch { return "Recent"; }
  }
</script>

<div class="table-card">
  <table class="attendance-table">
    <thead>
      <tr>
        <th>STUDENT / PARTICIPANT</th>
        <th>MATRIC NUMBER</th>
        <th>ROSTER STATUS</th>
        <th>CHECK-INS</th>
        <th>JOINED</th>
      </tr>
    </thead>
    <tbody>
      {#each filteredParticipants as participant (participant.id || participant.matric)}
        <tr class="table-row">
          <td class="student-cell">
            <div class="avatar" class:verified-avatar={participant.verified}>{getInitials(participant.name)}</div>
            <span class="student-name">{participant.name}</span>
          </td>
          <td><code class="matric-tag">{participant.matric}</code></td>
          <td>
            {#if participant.verified}
              <span class="badge verified"><ShieldCheck size={12} /> VERIFIED MATCH</span>
            {:else}
              <span class="badge provisional"><UserX size={12} /> PROVISIONAL GUEST</span>
            {/if}
          </td>
          <td>
            <span class="pulse-tag">
              <span class="dot" class:active={participant.heartbeats > 0}></span>
              {participant.heartbeats} pulse{participant.heartbeats === 1 ? "" : "s"}
            </span>
          </td>
          <td class="time-cell"><Clock size={11} /> {formatJoinedTime(participant.joinedAt)}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
<div class="footer-count">Showing {filteredParticipants.length} of {totalCount} participant record{totalCount === 1 ? "" : "s"}</div>

<style>
  .table-card { background: rgba(16, 9, 4, 0.6); border: 1px solid var(--color-cork-border); border-radius: 8px; overflow-x: auto; margin-bottom: 6px; }
  .attendance-table { width: 100%; border-collapse: collapse; text-align: left; font-size: 12px; }
  .attendance-table th { background: rgba(24, 14, 7, 0.9); color: var(--color-warm-cream-dim); font-size: 10px; letter-spacing: 0.1em; text-transform: uppercase; padding: 12px 16px; border-bottom: 1px solid var(--color-cork-border); font-weight: 700; }
  .table-row { border-bottom: 1px solid rgba(64, 55, 46, 0.4); transition: background 0.15s ease; }
  .table-row:hover { background: rgba(56, 36, 22, 0.35); }
  .attendance-table td { padding: 12px 16px; vertical-align: middle; color: var(--color-warm-cream); }
  .student-cell { display: flex; align-items: center; gap: 10px; }
  .avatar { width: 30px; height: 30px; border-radius: 6px; background: rgba(56, 36, 22, 0.8); border: 1px solid var(--color-cork-border); color: var(--color-warm-cream); font-size: 11px; font-weight: 700; display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
  .avatar.verified-avatar { background: rgba(74, 183, 114, 0.15); border-color: #4ab772; color: #4ab772; }
  .student-name { font-size: 13px; font-weight: 600; color: var(--color-warm-cream); }
  .matric-tag { font-family: monospace; font-size: 12px; background: rgba(10, 5, 2, 0.8); border: 1px solid var(--color-cork-border); padding: 4px 8px; border-radius: 4px; color: var(--color-warm-cream); }
  .badge { display: inline-flex; align-items: center; gap: 5px; font-size: 10px; font-weight: 700; text-transform: uppercase; padding: 4px 10px; border-radius: 4px; letter-spacing: 0.05em; }
  .badge.verified { background: rgba(74, 183, 114, 0.15); color: #4ab772; border: 1px solid #4ab772; }
  .badge.provisional { background: rgba(229, 169, 60, 0.15); color: #e5a93c; border: 1px solid #e5a93c; }
  .pulse-tag { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; color: var(--color-warm-cream); }
  .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--color-driftwood); }
  .dot.active { background: #38bdf8; box-shadow: 0 0 8px #38bdf8; }
  .time-cell { font-size: 11px; color: var(--color-warm-cream-dim); }
  .footer-count { font-size: 11px; color: var(--color-warm-cream-dim); text-align: right; }
</style>
