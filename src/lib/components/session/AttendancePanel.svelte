<script lang="ts">
  import type { Participant } from '$lib/types';
  import { exportSessionAttendanceCsv, getAttendanceCsvUrl } from '$lib/api';
  import SkeletonTable from '$lib/components/shared/SkeletonTable.svelte';
  import AttendanceSummaryBar from './attendance/AttendanceSummaryBar.svelte';
  import AttendanceControls from './attendance/AttendanceControls.svelte';
  import AttendanceListTable from './attendance/AttendanceListTable.svelte';

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

  let verifiedCount = $derived(participants.filter((p) => p.verified).length);
  let provisionalCount = $derived(
    participants.filter((p) => !p.verified).length
  );
  let matchRate = $derived(
    participants.length > 0
      ? Math.round((verifiedCount / participants.length) * 100)
      : 0
  );

  let filteredParticipants = $derived(
    participants
      .filter((p) => {
        if (statusFilter === 'verified' && !p.verified) return false;
        if (statusFilter === 'provisional' && p.verified) return false;
        if (!searchQuery.trim()) return true;
        const q = searchQuery.toLowerCase().trim();
        return (
          p.name.toLowerCase().includes(q) || p.matric.toLowerCase().includes(q)
        );
      })
      .sort((a, b) => {
        if (sortBy === 'name') return a.name.localeCompare(b.name);
        if (sortBy === 'heartbeats')
          return (b.heartbeats || 0) - (a.heartbeats || 0);
        return (
          (b.joinedAt ? new Date(b.joinedAt).getTime() : 0) -
          (a.joinedAt ? new Date(a.joinedAt).getTime() : 0)
        );
      })
  );

  async function handleExportCsv() {
    if (!sessionCode) return;
    isExporting = true;
    try {
      await exportSessionAttendanceCsv(sessionCode);
    } catch {
      window.open(getAttendanceCsvUrl(sessionCode), '_blank');
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

<div class="panel attendance-panel">
  <div class="attendance-header">
    <div>
      <p class="eyebrow">LIVE ATTENDANCE &amp; ROSTER RECONCILIATION</p>
      <h2>Session Attendance Feed</h2>
    </div>
  </div>

  <AttendanceSummaryBar
    totalCount={participants.length}
    {verifiedCount}
    {provisionalCount}
    {matchRate}
  />

  <AttendanceControls
    bind:searchQuery
    bind:statusFilter
    bind:sortBy
    {isExporting}
    {isRefreshing}
    onExportCsv={handleExportCsv}
    onRefresh={handleRefresh}
  />

  {#if isLoading}
    <SkeletonTable rows={4} columns={5} />
  {:else if filteredParticipants.length > 0}
    <AttendanceListTable participants={filteredParticipants} />
  {:else}
    <div class="empty-state-box">
      <p class="empty-title">No participants match your criteria</p>
      <p class="hint">
        Waiting for students to join using session code {sessionCode || '...'}
      </p>
    </div>
  {/if}
</div>

<style>
  .attendance-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-20);
    padding: var(--spacing-24);
  }
  .attendance-header h2 {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--color-warm-cream);
    margin: 2px 0 0 0;
  }
  .empty-state-box {
    padding: var(--spacing-41);
    text-align: center;
    border: 1px dashed var(--color-cork-border);
    border-radius: var(--radius-cards);
    color: var(--color-driftwood);
  }
  .empty-title {
    font-size: 15px;
    color: var(--color-warm-cream);
    margin-bottom: 4px;
  }
</style>
