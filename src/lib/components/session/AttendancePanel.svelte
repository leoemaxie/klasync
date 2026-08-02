<script lang="ts">
  import type { Participant } from '$lib/types';
  import { exportSessionAttendanceCsv, getAttendanceCsvUrl } from '$lib/api';
  import SkeletonTable from '$lib/components/shared/SkeletonTable.svelte';
  import ButtonSpinner from '$lib/components/shared/ButtonSpinner.svelte';
  import {
    Users,
    ShieldCheck,
    UserX,
    Activity,
    Search,
    Download,
    RefreshCw,
    X,
    Clock,
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
          // Default: latest joined first
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
      const blob = new Blob([csv], { type: 'type/csv;charset=utf-8;' });
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

<section class="attendance-container">
  <!-- Top Header Section -->
  <div class="panel-header">
    <div>
      <p class="eyebrow">SESSION ATTENDANCE MANAGEMENT</p>
      <h2 class="panel-title">
        {participants.length} Participant{participants.length === 1 ? '' : 's'}
        Registered
      </h2>
    </div>
    <div class="header-actions">
      {#if sessionCode}
        <button
          type="button"
          class="outline action-btn"
          onclick={handleExportCsv}
          disabled={isExporting || participants.length === 0}
          title="Export current session attendance as CSV"
        >
          {#if isExporting}
            <ButtonSpinner label="Exporting CSV..." /> Exporting...
          {:else}
            <Download size={14} /> Export CSV
          {/if}
        </button>
      {/if}
      <button
        type="button"
        class="text action-btn refresh-btn"
        onclick={handleRefresh}
        disabled={isRefreshing || isLoading}
        title="Sync live attendance with server"
      >
        <RefreshCw
          size={14}
          class={isRefreshing ? 'spin-icon' : ''}
        />
        {#if isRefreshing}Refreshing...{:else}Refresh Attendance{/if}
      </button>
    </div>
  </div>

  <!-- Summary Metric Grid Cards -->
  <div class="metrics-grid">
    <div class="metric-card">
      <div class="metric-icon-wrap">
        <Users size={18} />
      </div>
      <div class="metric-data">
        <span class="metric-label">TOTAL JOINED</span>
        <span class="metric-val">{participants.length}</span>
      </div>
    </div>

    <div class="metric-card">
      <div class="metric-icon-wrap verified-accent">
        <ShieldCheck size={18} />
      </div>
      <div class="metric-data">
        <div class="metric-label-row">
          <span class="metric-label">ROSTER VERIFIED</span>
          {#if participants.length > 0}
            <span class="rate-badge">{matchRate}% match</span>
          {/if}
        </div>
        <span class="metric-val verified-color">{verifiedCount}</span>
      </div>
    </div>

    <div class="metric-card">
      <div class="metric-icon-wrap provisional-accent">
        <UserX size={18} />
      </div>
      <div class="metric-data">
        <span class="metric-label">PROVISIONAL GUESTS</span>
        <span class="metric-val provisional-color">{provisionalCount}</span>
      </div>
    </div>

    <div class="metric-card">
      <div class="metric-icon-wrap activity-accent">
        <Activity size={18} />
      </div>
      <div class="metric-data">
        <span class="metric-label">ACTIVE CHECK-IN PULSES</span>
        <span class="metric-val pulse-color">{totalHeartbeats}</span>
      </div>
    </div>
  </div>

  <!-- Search, Filter & Sort Controls Toolbar -->
  <div class="toolbar">
    <div class="search-box">
      <Search size={14} class="search-icon" />
      <input
        type="text"
        placeholder="Search student name or matric number..."
        bind:value={searchQuery}
        class="search-input"
      />
      {#if searchQuery}
        <button
          type="button"
          class="clear-search-btn"
          onclick={() => (searchQuery = '')}
          aria-label="Clear search"
        >
          <X size={13} />
        </button>
      {/if}
    </div>

    <div class="filter-pills">
      <button
        type="button"
        class="pill-btn"
        class:active={statusFilter === 'all'}
        onclick={() => (statusFilter = 'all')}
      >
        All <span class="pill-count">{participants.length}</span>
      </button>
      <button
        type="button"
        class="pill-btn verified-pill"
        class:active={statusFilter === 'verified'}
        onclick={() => (statusFilter = 'verified')}
      >
        Verified <span class="pill-count">{verifiedCount}</span>
      </button>
      <button
        type="button"
        class="pill-btn provisional-pill"
        class:active={statusFilter === 'provisional'}
        onclick={() => (statusFilter = 'provisional')}
      >
        Provisional <span class="pill-count">{provisionalCount}</span>
      </button>
    </div>

    <div class="sort-selector">
      <label for="sort-select" class="sort-label">Sort:</label>
      <select id="sort-select" bind:value={sortBy} class="sort-select">
        <option value="joined">Latest Joined</option>
        <option value="name">Name (A-Z)</option>
        <option value="heartbeats">Check-in Pulses (High to Low)</option>
      </select>
    </div>
  </div>

  <!-- Content Section: Loading, Table, or Empty State -->
  {#if isLoading || isRefreshing}
    <div class="loading-wrap">
      <SkeletonTable
        rows={4}
        cols={4}
        label="Fetching live session attendance records..."
      />
    </div>
  {:else if filteredParticipants.length > 0}
    <div class="table-container">
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
                <div class="avatar-badge" class:verified-avatar={participant.verified}>
                  {getInitials(participant.name)}
                </div>
                <div class="student-info">
                  <span class="student-name">{participant.name}</span>
                </div>
              </td>
              <td>
                <code class="matric-tag">{participant.matric}</code>
              </td>
              <td>
                {#if participant.verified}
                  <span class="status-badge verified">
                    <ShieldCheck size={12} />
                    Verified Roster Match
                  </span>
                {:else}
                  <span class="status-badge provisional">
                    <UserX size={12} />
                    Provisional Guest
                  </span>
                {/if}
              </td>
              <td>
                <div class="pulse-count-wrap">
                  <span class="pulse-dot" class:active={participant.heartbeats > 0}></span>
                  <span class="pulse-count">{participant.heartbeats} pulse{participant.heartbeats === 1 ? '' : 's'}</span>
                </div>
              </td>
              <td class="joined-time-cell">
                <span class="joined-time">
                  <Clock size={11} />
                  {formatJoinedTime(participant.joinedAt)}
                </span>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <div class="table-footer-info">
      Showing {filteredParticipants.length} of {participants.length} participant record{participants.length === 1 ? '' : 's'}
    </div>
  {:else if searchQuery || statusFilter !== 'all'}
    <div class="empty-search-state">
      <Search size={28} class="empty-icon" />
      <p class="empty-title">No matching participants found</p>
      <p class="empty-desc">
        No students matched your filter criteria
        {#if searchQuery} for "<strong>{searchQuery}</strong>"{/if}.
      </p>
      <button
        type="button"
        class="outline reset-filter-btn"
        onclick={() => {
          searchQuery = '';
          statusFilter = 'all';
        }}
      >
        Reset Filters & Search
      </button>
    </div>
  {:else}
    <div class="empty-attendance-state">
      <div class="empty-icon-circle">
        <Users size={32} />
      </div>
      <h3>NO PARTICIPANTS YET</h3>
      <p class="empty-desc">
        Students joining with the session code will appear here automatically.
      </p>
      {#if sessionCode}
        <div class="code-callout">
          <span>Active Session Code:</span>
          <code class="active-code">{sessionCode}</code>
        </div>
      {/if}
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
    align-items: center;
    gap: var(--spacing-14);
    flex-wrap: wrap;
  }

  .panel-title {
    font-size: 22px;
    font-weight: 500;
    color: var(--color-warm-cream);
    margin: 4px 0 0 0;
    letter-spacing: -0.01em;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: var(--spacing-10);
  }

  .action-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    padding: 8px 14px;
  }

  :global(.spin-icon) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  /* Metrics Grid */
  .metrics-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
    gap: var(--spacing-12);
  }

  .metric-card {
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 8px;
    padding: 12px 14px;
    display: flex;
    align-items: center;
    gap: var(--spacing-12);
    transition: border-color 0.2s ease, background-color 0.2s ease;
  }

  .metric-card:hover {
    border-color: rgba(108, 95, 81, 0.6);
    background: rgba(24, 14, 7, 0.8);
  }

  .metric-icon-wrap {
    width: 38px;
    height: 38px;
    border-radius: 6px;
    background: rgba(56, 36, 22, 0.6);
    border: 1px solid var(--color-cork-border);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-driftwood);
    flex-shrink: 0;
  }

  .metric-icon-wrap.verified-accent {
    color: #4ab772;
    background: rgba(74, 183, 114, 0.1);
    border-color: rgba(74, 183, 114, 0.3);
  }

  .metric-icon-wrap.provisional-accent {
    color: var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.1);
    border-color: rgba(220, 80, 0, 0.3);
  }

  .metric-icon-wrap.activity-accent {
    color: #38bdf8;
    background: rgba(56, 189, 248, 0.1);
    border-color: rgba(56, 189, 248, 0.3);
  }

  .metric-data {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }

  .metric-label-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }

  .metric-label {
    font-size: 9px;
    letter-spacing: 0.08em;
    color: var(--color-driftwood);
    font-weight: 500;
    text-transform: uppercase;
  }

  .rate-badge {
    font-size: 9px;
    color: #4ab772;
    background: rgba(74, 183, 114, 0.15);
    padding: 1px 5px;
    border-radius: 3px;
    font-weight: 600;
  }

  .metric-val {
    font-size: 20px;
    font-weight: 600;
    color: var(--color-warm-cream);
    line-height: 1.1;
  }

  .metric-val.verified-color {
    color: #4ab772;
  }

  .metric-val.provisional-color {
    color: var(--color-ember-accent);
  }

  .metric-val.pulse-color {
    color: #38bdf8;
  }

  /* Toolbar */
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--spacing-12);
    flex-wrap: wrap;
    background: rgba(16, 9, 4, 0.4);
    border: 1px solid var(--color-cork-border);
    border-radius: 8px;
    padding: 10px 14px;
  }

  .search-box {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 220px;
    max-width: 380px;
  }

  :global(.search-icon) {
    position: absolute;
    left: 10px;
    color: var(--color-driftwood);
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    padding: 7px 30px 7px 32px;
    background: rgba(10, 5, 2, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    color: var(--color-warm-cream);
    font-size: 12px;
    transition: border-color 0.2s ease;
  }

  .search-input:focus {
    outline: none;
    border-color: var(--color-warm-cream);
  }

  .clear-search-btn {
    position: absolute;
    right: 8px;
    background: none;
    border: none;
    color: var(--color-driftwood);
    cursor: pointer;
    padding: 2px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .clear-search-btn:hover {
    color: var(--color-warm-cream);
  }

  .filter-pills {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .pill-btn {
    background: transparent;
    border: 1px solid var(--color-cork-border);
    border-radius: 20px;
    color: var(--color-driftwood);
    font-size: 11px;
    padding: 5px 12px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    transition: all 0.2s ease;
  }

  .pill-btn:hover {
    color: var(--color-warm-cream);
    border-color: rgba(108, 95, 81, 0.6);
  }

  .pill-btn.active {
    background: var(--color-bark-brown);
    color: var(--color-warm-cream);
    border-color: var(--color-warm-cream);
  }

  .pill-count {
    font-size: 10px;
    background: rgba(255, 237, 215, 0.12);
    padding: 1px 6px;
    border-radius: 10px;
  }

  .sort-selector {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .sort-label {
    font-size: 11px;
    color: var(--color-driftwood);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .sort-select {
    background: rgba(10, 5, 2, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    color: var(--color-warm-cream);
    font-size: 11px;
    padding: 6px 10px;
    cursor: pointer;
  }

  .sort-select:focus {
    outline: none;
    border-color: var(--color-warm-cream);
  }

  /* Table */
  .table-container {
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 8px;
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
    letter-spacing: 0.1em;
    text-transform: uppercase;
    padding: 12px 16px;
    border-bottom: 1px solid var(--color-cork-border);
    font-weight: 500;
  }

  .table-row {
    border-bottom: 1px solid rgba(64, 55, 46, 0.4);
    transition: background-color 0.15s ease;
  }

  .table-row:last-child {
    border-bottom: none;
  }

  .table-row:hover {
    background: rgba(56, 36, 22, 0.3);
  }

  .attendance-table td {
    padding: 12px 16px;
    vertical-align: middle;
    color: var(--color-warm-cream);
  }

  /* Student Cell */
  .student-cell {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .avatar-badge {
    width: 32px;
    height: 32px;
    border-radius: 6px;
    background: rgba(56, 36, 22, 0.8);
    border: 1px solid var(--color-cork-border);
    color: var(--color-driftwood);
    font-size: 11px;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    letter-spacing: 0.05em;
  }

  .avatar-badge.verified-avatar {
    background: rgba(74, 183, 114, 0.12);
    border-color: rgba(74, 183, 114, 0.4);
    color: #4ab772;
  }

  .student-info {
    display: flex;
    flex-direction: column;
  }

  .student-name {
    font-weight: 500;
    font-size: 13px;
    color: var(--color-warm-cream);
  }

  .matric-tag {
    font-family: monospace;
    font-size: 12px;
    background: rgba(10, 5, 2, 0.6);
    border: 1px solid var(--color-cork-border);
    padding: 3px 8px;
    border-radius: 4px;
    color: var(--color-warm-cream);
  }

  /* Status Badges */
  .status-badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 4px 10px;
    border-radius: 4px;
  }

  .status-badge.verified {
    background: rgba(74, 183, 114, 0.12);
    color: #4ab772;
    border: 1px solid rgba(74, 183, 114, 0.3);
  }

  .status-badge.provisional {
    background: rgba(220, 80, 0, 0.12);
    color: var(--color-ember-accent);
    border: 1px solid rgba(220, 80, 0, 0.3);
  }

  /* Pulse count */
  .pulse-count-wrap {
    display: inline-flex;
    align-items: center;
    gap: 7px;
  }

  .pulse-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--color-driftwood);
  }

  .pulse-dot.active {
    background: #38bdf8;
    box-shadow: 0 0 6px rgba(56, 189, 248, 0.6);
  }

  .pulse-count {
    font-size: 12px;
    color: var(--color-warm-cream);
  }

  /* Joined time */
  .joined-time-cell {
    color: var(--color-driftwood);
  }

  .joined-time {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--color-driftwood);
  }

  .table-footer-info {
    font-size: 11px;
    color: var(--color-driftwood);
    text-align: right;
    padding-right: 4px;
  }

  /* Empty States */
  .loading-wrap {
    margin-top: var(--spacing-12);
  }

  .empty-search-state,
  .empty-attendance-state {
    background: rgba(16, 9, 4, 0.4);
    border: 1px dashed var(--color-cork-border);
    border-radius: 8px;
    padding: var(--spacing-40) var(--spacing-20);
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--spacing-12);
  }

  .empty-icon-circle {
    width: 60px;
    height: 60px;
    border-radius: 50%;
    background: rgba(56, 36, 22, 0.6);
    border: 1px solid var(--color-cork-border);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-driftwood);
  }

  .empty-title {
    font-size: 15px;
    font-weight: 500;
    color: var(--color-warm-cream);
    margin: 0;
  }

  .empty-desc {
    font-size: 12px;
    color: var(--color-driftwood);
    max-width: 420px;
    margin: 0;
    line-height: 1.5;
  }

  .reset-filter-btn {
    font-size: 11px;
    margin-top: 8px;
    padding: 6px 14px;
  }

  .code-callout {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    background: rgba(10, 5, 2, 0.6);
    border: 1px solid var(--color-cork-border);
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 12px;
    color: var(--color-driftwood);
    margin-top: 8px;
  }

  .active-code {
    font-family: monospace;
    font-size: 14px;
    font-weight: 700;
    color: var(--color-warm-cream);
    letter-spacing: 0.1em;
  }
</style>

