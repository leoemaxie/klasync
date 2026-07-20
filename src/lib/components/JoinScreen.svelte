<script lang="ts">
  let {
    sessionCode = $bindable(""),
    matric = $bindable(""),
    displayName = $bindable(""),
    joinError = "",
    onJoinSession,
  }: {
    sessionCode: string;
    matric: string;
    displayName: string;
    joinError?: string;
    onJoinSession: () => void;
  } = $props();
</script>

<section class="join-wrap">
  <p class="eyebrow">JOIN LIVE SESSION</p>
  <h1>Enter the room.</h1>
  <p>
    There is no account requirement for a live lecture. Your lecturer may use
    your matric number to verify attendance.
  </p>
  <div class="join-card">
    <label>
      Session code
      <input
        bind:value={sessionCode}
        placeholder="e.g. A4K9QZ"
        maxlength="6"
      />
    </label>
    <label>
      Matric / student ID
      <input bind:value={matric} placeholder="MAT/2023/001" />
    </label>
    <label>
      Full name <span>only if not on the roster</span>
      <input bind:value={displayName} placeholder="Your name" />
    </label>
    {#if joinError}
      <p class="error">{joinError}</p>
    {/if}
    <button class="primary full" onclick={onJoinSession}>Join lecture</button>
    <p class="hint">
      Need the code? Ask your lecturer or open their invite link.
    </p>
  </div>
</section>
