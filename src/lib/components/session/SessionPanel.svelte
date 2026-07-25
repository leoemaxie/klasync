<script lang="ts">
  import type { Session } from "$lib/types";
  import QrCodeSvg from "$lib/components/shared/QrCodeSvg.svelte";

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
  <p class="eyebrow">LIVE SESSION CONTROL</p>
  {#if session?.live}
    <h2>{session.title}</h2>
    <div class="code">{session.code}</div>
    <p class="hint">
      Students can enter this code, scan your generated QR invite, or open the link below.
    </p>
    <div class="invite">
      <input readonly value={inviteUrl} />
      <button class="outline" onclick={onCopyInvite}>
        {copied ? "Copied" : "Copy Link"}
      </button>
    </div>
    <div class="qr-preview-box">
      <QrCodeSvg value={inviteUrl || session.code} size={140} />
    </div>
    {#if apiNotice}<p class="error">{apiNotice}</p>{/if}
    <button class="danger full" onclick={onEndSession}>End Live Session</button>
  {:else}
    <h2>Ready when you are.</h2>
    <p class="lede">
      Start a live session to generate a 6-character short code, invite link, and QR code for your students.
    </p>
    {#if apiNotice}<p class="error">{apiNotice}</p>{/if}
    <button
      class="primary full"
      onclick={onStartSession}
      disabled={!lecturerName.trim() || !lecturerEmail.trim() || isSaving}
    >
      {isSaving ? "Starting live room..." : "Start Live Session"}
    </button>
  {/if}
</div>

<style>
  .qr-preview-box {
    margin: var(--spacing-18) 0;
    display: flex;
    justify-content: center;
  }
</style>
