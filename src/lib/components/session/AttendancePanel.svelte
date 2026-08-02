<script lang="ts">
  import type { Participant } from '$lib/types';
  import { exportSessionAttendanceCsv, getAttendanceCsvUrl } from '$lib/api';
  import SkeletonTable from '$lib/components/shared/SkeletonTable.svelte';
  import ButtonSpinner from '$lib/components/shared/ButtonSpinner.svelte';

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
      // Fallback to direct URL download
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

<section class="attendance">
  <div class="header-row">
    <div>
      <p class="eyebrow">LIVE ATTENDANCE</p>
      <h2>
        {participants.length} participant{participants.length === 1 ? '' : 's'}
      </h2>
    </div>
    <div class="actions">
      {#if sessionCode}
        <button
          class="outline"
          onclick={handleExportCsv}
          disabled={isExporting}
        >
          {#if isExporting}
            <ButtonSpinner label="Exporting attendance CSV..." /> Exporting...
          {:else}
            Export CSV
          {/if}
        </button>
      {/if}
      <button
        class="text"
        onclick={handleRefresh}
        disabled={isRefreshing || isLoading}
      >
        {#if isRefreshing}
          <ButtonSpinner label="Refreshing live attendance..." /> Refreshing...
        {:else}
          Refresh attendance
        {/if}
      </button>
    </div>
  </div>

  {#if isLoading || isRefreshing}
    <div style="margin-top: var(--spacing-14);">
      <SkeletonTable
        rows={3}
        cols={3}
        label="Fetching latest session attendance records..."
      />
    </div>
  {:else if participants.length}
    <div class="participant-list">
      {#each participants as participant}
        <p>
          <span>{participant.name}</span>
          <small>
            {participant.matric} · {participant.verified
              ? 'Verified roster match'
              : 'Provisional'} · {participant.heartbeats} check-ins
          </small>
        </p>
      {/each}
    </div>
  {:else}
    <p class="hint">
      Participants will appear here as they join. Refresh to retrieve the
      authoritative API record.
    </p>
  {/if}
</section>

<style>
  .header-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--spacing-12);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: var(--spacing-12);
  }
</style>
