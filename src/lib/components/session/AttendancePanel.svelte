<script lang="ts">
  import type { Participant } from "$lib/types";
  import { exportSessionAttendanceCsv, getAttendanceCsvUrl } from "$lib/api";

  let {
    sessionCode = "",
    participants = [],
    onRefreshAttendance,
  }: {
    sessionCode?: string;
    participants: Participant[];
    onRefreshAttendance: () => void;
  } = $props();

  let isExporting = $state(false);

  async function handleExportCsv() {
    if (!sessionCode) return;
    isExporting = true;
    try {
      const csv = await exportSessionAttendanceCsv(sessionCode);
      const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.setAttribute("download", `attendance_${sessionCode}.csv`);
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
    } catch {
      // Fallback to direct URL download
      const directUrl = getAttendanceCsvUrl(sessionCode);
      window.open(directUrl, "_blank");
    } finally {
      isExporting = false;
    }
  }
</script>

<section class="attendance">
  <div class="header-row">
    <div>
      <p class="eyebrow">LIVE ATTENDANCE</p>
      <h2>
        {participants.length} participant{participants.length === 1 ? "" : "s"}
      </h2>
    </div>
    <div class="actions">
      {#if sessionCode}
        <button class="outline" onclick={handleExportCsv} disabled={isExporting}>
          {isExporting ? "Exporting..." : "Export CSV"}
        </button>
      {/if}
      <button class="text" onclick={onRefreshAttendance}>
        Refresh attendance
      </button>
    </div>
  </div>

  {#if participants.length}
    <div class="participant-list">
      {#each participants as participant}
        <p>
          <span>{participant.name}</span>
          <small>
            {participant.matric} · {participant.verified
              ? "Verified roster match"
              : "Provisional"} · {participant.heartbeats} check-ins
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
