<script lang="ts">
  import { lookupSessionByCode } from "$lib/api/sessions";
  import PublicVisualPanel from "$lib/components/shared/PublicVisualPanel.svelte";

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

  let sessionTitle = $state("");
  let sessionStatus = $state<"idle" | "live" | "ended">("idle");

  async function checkCode() {
    if (!sessionCode.trim() || sessionCode.trim().length < 4) return;
    try {
      const info = await lookupSessionByCode(sessionCode.trim());
      sessionTitle = info.session.title;
      sessionStatus = info.session.status;
    } catch {
      sessionTitle = "";
      sessionStatus = "idle";
    }
  }
</script>

<section class="join-wrap">
  <div class="join-left-content">
    <p class="eyebrow">GUEST ACCESS / NO ACCOUNT REQUIRED</p>
    <h1>Enter the room.</h1>
    <p class="lede">
      Follow real-time captions and record your attendance seamlessly without creating an account.
    </p>
    <div class="join-card panel">
      <label>
        Session short code
        <input
          bind:value={sessionCode}
          onblur={checkCode}
          placeholder="e.g. A4K9QZ"
          maxlength="8"
        />
      </label>

      {#if sessionTitle}
        <div class="session-info-badge">
          <p class="eyebrow"><span class="eyebrow-accent">●</span> {sessionStatus.toUpperCase()}</p>
          <h2>{sessionTitle}</h2>
        </div>
      {/if}

      <label>
        Matric / Student ID
        <input bind:value={matric} placeholder="MAT/2023/001" />
      </label>

      <label>
        Full name <span>(optional if on roster)</span>
        <input bind:value={displayName} placeholder="Ada Okafor" />
      </label>

      {#if joinError}
        <p class="error">{joinError}</p>
      {/if}

      <button
        class="primary full"
        onclick={onJoinSession}
        disabled={sessionStatus === "ended" || !matric.trim()}
      >
        {sessionStatus === "ended" ? "Session Ended" : "Join live lecture"}
      </button>
      <p class="hint">
        Verification status will be confirmed against your course roster on entry.
      </p>
    </div>
  </div>

  <PublicVisualPanel title="INSTANT GUEST ACCESS" subtitle="Zero barriers · Real-time captions · Fair attendance" />
</section>
