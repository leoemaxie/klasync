<script lang="ts">
  import type { Session } from "$lib/types";

  let {
    session,
    apiNotice = "",
    isSaving = false,
    copied = false,
    lecturerName = "",
    lecturerEmail = "",
    onCopyInvite,
    onEndSession,
    onStartSession,
  }: {
    session: Session | null;
    apiNotice?: string;
    isSaving?: boolean;
    copied?: boolean;
    lecturerName?: string;
    lecturerEmail?: string;
    onCopyInvite: () => void;
    onEndSession: () => void;
    onStartSession: () => void;
  } = $props();

  const inviteUrl = $derived(
    session?.code
      ? `${typeof location !== "undefined" ? location.origin : ""}/?join=${session.code}`
      : ""
  );
</script>

<div class="panel session-panel">
  <p class="eyebrow">LIVE SESSION</p>
  {#if session?.live}
    <h2>{session.title}</h2>
    <div class="code">{session.code}</div>
    <p>
      Students can enter this code, scan your QR invite, or use the link below.
    </p>
    <div class="invite">
      <input readonly value={inviteUrl} />
      <button class="outline" onclick={onCopyInvite}>
        {copied ? "Copied" : "Copy"}
      </button>
    </div>
    <div class="qr" aria-label="QR-code placeholder">{session.code}</div>
    <p class="hint">
      The API now issues the session; QR image generation is the remaining
      invite-format task.
    </p>
    {#if apiNotice}<p class="error">{apiNotice}</p>{/if}
    <button class="danger" onclick={onEndSession}>End session</button>
  {:else}
    <h2>Ready when you are.</h2>
    <p>
      Create a live session through the local KLASYNC API to generate a code
      and invite link.
    </p>
    {#if apiNotice}<p class="error">{apiNotice}</p>{/if}
    <button
      class="primary full"
      onclick={onStartSession}
      disabled={!lecturerName.trim() || !lecturerEmail.trim() || isSaving}
    >
      {isSaving ? "Starting session" : "Start live session"}
    </button>
  {/if}
</div>
