<script lang="ts">
  import type { Participant, Session } from "$lib/types";
  import type { AuthUser } from "$lib/api/auth";

  let {
    session,
    joinedParticipant,
    wsConnected = false,
    currentUser = null,
    onLecturerView
  }: {
    session: Session | null;
    joinedParticipant: Participant | null;
    wsConnected?: boolean;
    currentUser?: AuthUser | null;
    onLecturerView: () => void;
  } = $props();
</script>

<section class="live-header-bar" aria-label="Live session header">
  <div class="header-main">
    <div class="live-status-row">
      <span class="live-pill" class:connected={wsConnected}>
        <span class="pulse-dot" aria-hidden="true">●</span>
        {wsConnected ? "LIVE STREAM ACTIVE" : "CAPTIONS CONNECTED"}
      </span>
      {#if joinedParticipant?.verified}
        <span class="verified-badge">VERIFIED STUDENT</span>
      {:else}
        <span class="provisional-badge">PROVISIONAL ATTENDEE</span>
      {/if}
    </div>
    <h1 class="live-title">{session?.title ?? "Live Lecture Session"}</h1>
    <p class="live-subtitle">
      {joinedParticipant?.verified
        ? "Your presence is verified on the course roster."
        : "Provisional attendance recorded. Verification pending."}
    </p>
  </div>
  {#if !currentUser || currentUser.role === 'lecturer' || currentUser.role === 'admin'}
    <button type="button" class="outline lecturer-switch-btn" onclick={onLecturerView}>
      Lecturer View
    </button>
  {/if}
</section>

<style>
  .live-header-bar { display: flex; justify-content: space-between; align-items: center; gap: var(--spacing-18); margin-bottom: var(--spacing-20); padding-bottom: var(--spacing-18); border-bottom: 1px solid var(--color-cork-border); }
  .live-status-row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-bottom: 6px; }
  .live-pill { font-size: 9px; font-weight: 700; letter-spacing: 0.12em; color: var(--color-ember-accent); background: rgba(220, 80, 0, 0.12); border: 1px solid var(--color-ember-accent); padding: 4px 10px; border-radius: 4px; display: inline-flex; align-items: center; gap: 4px; }
  .live-pill.connected { color: #4ab772; background: rgba(74, 183, 114, 0.12); border-color: #4ab772; }
  .verified-badge { font-size: 9px; font-weight: 700; letter-spacing: 0.12em; color: #4ab772; background: rgba(74, 183, 114, 0.15); border: 1px solid #4ab772; padding: 4px 10px; border-radius: 4px; }
  .provisional-badge { font-size: 9px; font-weight: 700; letter-spacing: 0.12em; color: #e5a93c; background: rgba(229, 169, 60, 0.12); border: 1px solid #e5a93c; padding: 4px 10px; border-radius: 4px; }
  .pulse-dot { animation: blink 1s infinite alternate; }
  @keyframes blink { 0% { opacity: 0.3; } 100% { opacity: 1; } }
  .live-title { font-size: 26px; font-weight: 700; margin: 4px 0; color: var(--color-warm-cream); font-family: var(--font-display); }
  .live-subtitle { font-size: 13px; color: var(--color-warm-cream-dim); }
  @media (max-width: 640px) { .live-header-bar { flex-direction: column; align-items: stretch; gap: 12px; } .lecturer-switch-btn { width: 100%; text-align: center; } }
</style>
