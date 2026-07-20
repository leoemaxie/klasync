<script lang="ts">
  import type { Participant } from "$lib/types";

  let {
    participants = [],
    onRefreshAttendance,
  }: {
    participants: Participant[];
    onRefreshAttendance: () => void;
  } = $props();
</script>

<section class="attendance">
  <div>
    <p class="eyebrow">LIVE ATTENDANCE</p>
    <h2>
      {participants.length} participant{participants.length === 1 ? "" : "s"}
    </h2>
    <button class="text" onclick={onRefreshAttendance}>
      Refresh attendance
    </button>
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
