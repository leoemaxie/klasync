<script lang="ts">
  import type { Participant } from '$lib/types';
  import { exportSessionAttendanceCsv, getAttendanceCsvUrl } from '$lib/api';
  import SkeletonTable from '$lib/components/shared/SkeletonTable.svelte';
  import ButtonSpinner from '$lib/components/shared/ButtonSpinner.svelte';
  import {
    Users,
    ShieldCheck,
    UserX,
    Search,
    Download,
    RefreshCw,
    X,
    Clock,
    Activity,
  } from '@lucide/svelte';

  let {
    sessionCode = '',
    participants = [],
    isLoading = false,
    onRefreshAttendance,
  }: {
    sessionCode?: string;
    participants: Participant[];
    isLoading?: boolean;
    onRefreshAttendance: () => Promise<void> | void;
  } = $props();

  let isExporting = $state(false);
  let isRefreshing = $state(false);

  let searchQuery = $state('');
  let statusFilter = $state<'all' | 'verified' | 'provisional'>('all');
  let sortBy = $state<'joined' | 'name' | 'heartbeats'>('joined');

  // Summary Metrics
  let verifiedCount = $derived(participants.filter((p) => p.verified).length);
  let provisionalCount = $derived(
    participants.filter((p) => !p.verified).length
  );
  let totalHeartbeats = $derived(
    participants.reduce((acc, p) => acc + (p.heartbeats || 0), 0)
  );
  let matchRate = $derived(
    participants.length > 0
      ? Math.round((verifiedCount / participants.length) * 100)
      : 0
  );

  // Filtered and sorted participant records
  let filteredParticipants = $derived(
    participants
      .filter((p) => {
        if (statusFilter === 'verified' && !p.verified) return false;
        if (statusFilter === 'provisional' && p.verified) return false;

        if (!searchQuery.trim()) return true;
        const q = searchQuery.toLowerCase().trim();
        return (
          p.name.toLowerCase().includes(q) ||
          p.matric.toLowerCase().includes(q)
        );
      })
      .sort((a, b) => {
        if (sortBy === 'name') {
          return a.name.localeCompare(b.name);
        } else if (sortBy === 'heartbeats') {
          return (b.heartbeats || 0) - (a.heartbeats || 0);
        } else {
          const timeA = a.joinedAt ? new Date(a.joinedAt).getTime() : 0;
          const timeB = b.joinedAt ? new Date(b.joinedAt).getTime() : 0;
          return timeB - timeA;
        }
      })
  );

  function getInitials(name: string): string {
    if (!name) return '?';
    const parts = name.trim().split(/\s+/);
    if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
    return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
  }

  function formatJoinedTime(timestamp?: string): string {
    if (!timestamp) return 'Recent';
    try {
      const date = new Date(timestamp);
      if (isNaN(date.getTime())) return 'Recent';
      return date.toLocaleTimeString([], {
        hour: '2-digit',
        minute: '2-digit',
      });
    } catch {
      return 'Recent';
    }
  }

  async function handleExportCsv() {
    if (!sessionCode) return;
    isExporting = true;
    try {
      const csv = await exportSessionAttendanceCsv(sessionCode);
      const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.setAttribute('download', `attendance_${sessionCode}.csv`);
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
    } catch {
      const directUrl = getAttendanceCsvUrl(sessionCode);
      window.open(directUrl, '_blank');
    } finally {
      isExporting = false;
    }
  }

  async function handleRefresh() {
    isRefreshing = true;
    try {
      await onRefreshAttendance();
    } finally {
      isRefreshing = false;
    }
  }
</script>

<section class="attendance-container" aria-label="Live session attendance">
  <!-- Clean Header with Summary Stats -->
  <div class="panel-header">
    <div class="header-titles">
      <p class="eyebrow">LIVE SESSION ATTENDANCE</p>
      <h2 class="panel-title">Session Roster</h2>
      <div class="summary-inline-pills">
        <span class="inline-pill total">
          <Users size={12} aria-hidden="true" /> {participants.length} Registered
        </span>
        <span class="inline-pill verified">
          <ShieldCheck size={12} aria-hidden="true" /> {verifiedCount} Verified ({matchRate}%)
        </span>
        <span class="inline-pill provisional">
          <UserX size={12} aria-hidden="true" /> {provisionalCount} Provisional
        </span>
        <span class="inline-pill pulses">
          <Activity size={12} aria-hidden="true" /> {totalHeartbeats} Pulses
        </span>
      </div>
    </div>

    <div class="header-actions">
      {#if sessionCode}
        <button
          type="button"
          class="outline action-btn"
          onclick={handleExportCsv}
          disabled={isExporting || participants.length === 0}
        >
          {#if isExporting}
            <ButtonSpinner label="Exporting..." /> Exporting...
          {:else}
            <Download size={13} aria-hidden="true" /> Export CSV
          {/if}
        </button>
      {/if}
      <button
        type="button"
        class="text action-btn"
        onclick={handleRefresh}
        disabled={isRefreshing || isLoading}
      >
        <RefreshCw size={13} aria-hidden="true" class={isRefreshing ? 'spin-icon' : ''} />
        {#if isRefreshing}Refreshing...{:else}Refresh{/if}
      </button>
    </div>
  </div>

  <!-- Single Streamlined Controls Bar -->
  <div class="controls-bar">
    <div class="search-wrap">
      <Search size={14} aria-hidden="true" class="search-icon" />
      <input
        type="text"
        placeholder="Filter by student name or matric..."
        aria-label="Filter by student name or matric"
        bind:value={searchQuery}
        class="search-input"
      />
      {#if searchQuery}
        <button
          type="button"
          class="clear-search-btn"
          onclick={() => (searchQuery = '')}
          aria-label="Clear filter text"
        >
          <X size={12} aria-hidden="true" />
        </button>
      {/if}
    </div>

    <div class="filter-group" role="group" aria-label="Status filter">
      <button
        type="button"
        class="pill-tab"
        class:active={statusFilter === 'all'}
        aria-pressed={statusFilter === 'all'}
        onclick={() => (statusFilter = 'all')}
      >
        All ({participants.length})
      </button>
      <button
        type="button"
        class="pill-tab"
        class:active={statusFilter === 'verified'}
        aria-pressed={statusFilter === 'verified'}
        onclick={() => (statusFilter = 'verified')}
      >
        Verified ({verifiedCount})
      </button>
      <button
        type="button"
        class="pill-tab"
        class:active={statusFilter === 'provisional'}
        aria-pressed={statusFilter === 'provisional'}
        onclick={() => (statusFilter = 'provisional')}
      >
        Provisional ({provisionalCount})
      </button>
    </div>

    <div class="sort-group">
      <select bind:value={sortBy} class="sort-select" aria-label="Sort attendance list">
        <option value="joined">Latest Joined</option>
        <option value="name">Name (A-Z)</option>
        <option value="heartbeats">Check-in Pulses</option>
      </select>
    </div>
  </div>

  <!-- Attendance Table & States -->
  {#if isLoading || isRefreshing}
    <div class="loading-wrap">
      <SkeletonTable
        rows={4}
        cols={4}
        label="Fetching live attendance records..."
      />
    </div>
  {:else if filteredParticipants.length > 0}
    <div class="table-card">
      <table class="attendance-table">
        <thead>
          <tr>
            <th scope="col">STUDENT / PARTICIPANT</th>
            <th scope="col">MATRIC NUMBER</th>
            <th scope="col">ROSTER STATUS</th>
            <th scope="col">PULSE</th>
            <th scope="col">JOINED</th>
          </tr>
        </thead>
        <tbody>
          {#each filteredParticipants as participant (participant.id || participant.matric)}
            <tr class="table-row">
              <td class="student-cell">
                <div class="avatar" class:verified-avatar={participant.verified}>
                  {getInitials(participant.name)}
                </div>
                <span class="student-name">{participant.name}</span>
              </td>
              <td>
                <code class="matric-tag">{participant.matric}</code>
              </td>
              <td>
                {#if participant.verified}
                  <span class="badge verified">
                    <ShieldCheck size={12} aria-hidden="true" /> Verified Match
                  </span>
                {:else}
                  <span class="badge provisional">
                    <UserX size={12} aria-hidden="true" /> Provisional
                  </span>
                {/if}
              </td>
              <td>
                <span class="pulse-tag">
                  <span class="dot" class:active={participant.heartbeats > 0} aria-hidden="true"></span>
                  {participant.heartbeats} pulse{participant.heartbeats === 1 ? '' : 's'}
                </span>
              </td>
              <td class="time-cell">
                <Clock size={11} aria-hidden="true" /> {formatJoinedTime(participant.joinedAt)}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    <div class="footer-count">
      Showing {filteredParticipants.length} of {participants.length} record{participants.length === 1 ? '' : 's'}
    </div>
  {:else if searchQuery || statusFilter !== 'all'}
    <div class="empty-state">
      <Search size={24} class="empty-icon" />
      <p class="empty-title">No matching participants</p>
      <p class="empty-desc">
        No students matched your search criteria.
      </p>
      <button
        type="button"
        class="outline reset-btn"
        onclick={() => {
          searchQuery = '';
          statusFilter = 'all';
        }}
      >
        Clear Filters
      </button>
    </div>
  {:else}
    <div class="empty-state">
      <Users size={28} class="empty-icon" />
      <p class="empty-title">No participants joined yet</p>
      <p class="empty-desc">
        Students joining live with code <strong>{sessionCode}</strong> will appear here automatically.
      </p>
    </div>
  {/if}
</section>

<style>
  .attendance-container {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-16);
  }

  /* Header */
  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: var(--spacing-14);
    flex-wrap: wrap;
    padding-bottom: 4px;
  }

  .header-titles {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .eyebrow {
    font-size: 10px;
    letter-spacing: 0.1em;
    color: var(--color-driftwood);
    margin: 0;
    line-height: 1;
    font-weight: 600;
  }

  .panel-title {
    font-size: 22px;
    font-weight: 500;
    color: var(--color-warm-cream);
    margin: 2px 0 6px 0;
    font-family: var(--font-display);
    line-height: 1.2;
  }

  .summary-inline-pills {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .inline-pill {
    font-size: 10px;
    font-weight: 500;
    padding: 3px 8px;
    border-radius: 4px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    letter-spacing: 0.03em;
  }

  .inline-pill.total {
    background: rgba(56, 36, 22, 0.6);
    color: var(--color-warm-cream);
    border: 1px solid var(--color-cork-border);
  }

  .inline-pill.verified {
    background: rgba(74, 183, 114, 0.12);
    color: #4ab772;
    border: 1px solid rgba(74, 183, 114, 0.25);
  }

  .inline-pill.provisional {
    background: rgba(220, 80, 0, 0.12);
    color: var(--color-ember-accent);
    border: 1px solid rgba(220, 80, 0, 0.25);
  }

  .inline-pill.pulses {
    background: rgba(56, 189, 248, 0.1);
    color: #38bdf8;
    border: 1px solid rgba(56, 189, 248, 0.25);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .action-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    padding: 6px 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  :global(.spin-icon) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  /* Controls Bar */
  .controls-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
    background: rgba(16, 9, 4, 0.4);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    padding: 8px 12px;
  }

  .search-wrap {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 200px;
    max-width: 320px;
  }

  :global(.search-icon) {
    position: absolute;
    left: 8px;
    color: var(--color-driftwood);
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    padding: 6px 26px 6px 28px;
    background: rgba(10, 5, 2, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 4px;
    color: var(--color-warm-cream);
    font-size: 11px;
  }

  .search-input:focus {
    outline: none;
    border-color: var(--color-warm-cream);
  }

  .clear-search-btn {
    position: absolute;
    right: 6px;
    background: none;
    border: none;
    color: var(--color-driftwood);
    cursor: pointer;
    padding: 2px;
    display: flex;
  }

  .filter-group {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .pill-tab {
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    color: var(--color-driftwood);
    font-size: 11px;
    padding: 4px 10px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .pill-tab:hover {
    color: var(--color-warm-cream);
  }

  .pill-tab.active {
    background: var(--color-bark-brown);
    color: var(--color-warm-cream);
    border-color: var(--color-cork-border);
  }

  .sort-select {
    background: rgba(10, 5, 2, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 4px;
    color: var(--color-warm-cream);
    font-size: 11px;
    padding: 5px 8px;
    cursor: pointer;
  }

  /* Table */
  .table-card {
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    overflow-x: auto;
  }

  .attendance-table {
    width: 100%;
    border-collapse: collapse;
    text-align: left;
    font-size: 12px;
  }

  .attendance-table th {
    background: rgba(24, 14, 7, 0.8);
    color: var(--color-driftwood);
    font-size: 9px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding: 10px 14px;
    border-bottom: 1px solid var(--color-cork-border);
    font-weight: 500;
  }

  .table-row {
    border-bottom: 1px solid rgba(64, 55, 46, 0.3);
    transition: background 0.15s ease;
  }

  .table-row:last-child {
    border-bottom: none;
  }

  .table-row:hover {
    background: rgba(56, 36, 22, 0.25);
  }

  .attendance-table td {
    padding: 10px 14px;
    vertical-align: middle;
    color: var(--color-warm-cream);
  }

  .student-cell {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .avatar {
    width: 28px;
    height: 28px;
    border-radius: 4px;
    background: rgba(56, 36, 22, 0.6);
    border: 1px solid var(--color-cork-border);
    color: var(--color-driftwood);
    font-size: 10px;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .avatar.verified-avatar {
    background: rgba(74, 183, 114, 0.1);
    border-color: rgba(74, 183, 114, 0.3);
    color: #4ab772;
  }

  .student-name {
    font-size: 12px;
    font-weight: 500;
  }

  .matric-tag {
    font-family: monospace;
    font-size: 11px;
    background: rgba(10, 5, 2, 0.5);
    border: 1px solid var(--color-cork-border);
    padding: 2px 6px;
    border-radius: 3px;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    padding: 3px 8px;
    border-radius: 3px;
  }

  .badge.verified {
    background: rgba(74, 183, 114, 0.1);
    color: #4ab772;
    border: 1px solid rgba(74, 183, 114, 0.25);
  }

  .badge.provisional {
    background: rgba(220, 80, 0, 0.1);
    color: var(--color-ember-accent);
    border: 1px solid rgba(220, 80, 0, 0.25);
  }

  .pulse-tag {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--color-driftwood);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-driftwood);
  }

  .dot.active {
    background: #38bdf8;
    box-shadow: 0 0 6px rgba(56, 189, 248, 0.6);
  }

  .time-cell {
    font-size: 11px;
    color: var(--color-driftwood);
    display: table-cell;
  }

  .footer-count {
    font-size: 11px;
    color: var(--color-driftwood);
    text-align: right;
  }

  /* Empty state */
  .empty-state {
    background: rgba(16, 9, 4, 0.4);
    border: 1px dashed var(--color-cork-border);
    border-radius: 6px;
    padding: var(--spacing-32);
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }

  :global(.empty-icon) {
    color: var(--color-driftwood);
  }

  .empty-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--color-warm-cream);
    margin: 0;
  }

  .empty-desc {
    font-size: 12px;
    color: var(--color-driftwood);
    margin: 0;
  }

  .reset-btn {
    font-size: 11px;
    margin-top: 6px;
    padding: 4px 12px;
  }
</style>

