<script lang="ts">
  import { lookupSessionByCode } from '$lib/api/sessions';
  import PublicVisualPanel from '$lib/components/shared/PublicVisualPanel.svelte';
  import Skeleton from '$lib/components/shared/Skeleton.svelte';
  import ButtonSpinner from '$lib/components/shared/ButtonSpinner.svelte';

  let {
    sessionCode = $bindable(''),
    matric = $bindable(''),
    displayName = $bindable(''),
    joinError = '',
    onJoinSession,
  }: {
    sessionCode: string;
    matric: string;
    displayName: string;
    joinError?: string;
    onJoinSession: () => Promise<void> | void;
  } = $props();

  let sessionTitle = $state('');
  let sessionStatus = $state<'idle' | 'live' | 'ended'>('idle');
  let isCheckingCode = $state(false);
  let isJoining = $state(false);

  async function checkCode() {
    if (!sessionCode.trim() || sessionCode.trim().length < 4) return;
    isCheckingCode = true;
    try {
      const info = await lookupSessionByCode(sessionCode.trim());
      sessionTitle = info.session.title;
      sessionStatus = info.session.status;
    } catch {
      sessionTitle = '';
      sessionStatus = 'idle';
    } finally {
      isCheckingCode = false;
    }
  }

  async function handleJoin() {
    isJoining = true;
    try {
      await onJoinSession();
    } finally {
      isJoining = false;
    }
  }
</script>

<section class="join-wrap">
  <div class="join-left-content">
    <p class="eyebrow">GUEST ACCESS / NO ACCOUNT REQUIRED</p>
    <h1>Enter room.</h1>
    <p class="lede">
      Real-time captions and verified attendance. No account required.
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

      {#if isCheckingCode}
        <div style="margin: var(--spacing-12) 0;">
          <Skeleton height="36px" label="Verifying session code..." />
        </div>
      {:else if sessionTitle}
        <div class="session-info-badge">
          <p class="eyebrow">
            <span class="eyebrow-accent">●</span>
            {sessionStatus.toUpperCase()}
          </p>
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
        onclick={handleJoin}
        disabled={sessionStatus === 'ended' || !matric.trim() || isJoining}
      >
        {#if isJoining}
          <ButtonSpinner label="Verifying..." /> Entering...
        {:else if sessionStatus === 'ended'}
          Session Ended
        {:else}
          Join lecture
        {/if}
      </button>
      <p class="hint">
        Matric number verified against course roster.
      </p>
    </div>
  </div>

  <PublicVisualPanel
    title="INSTANT GUEST ACCESS"
    subtitle="Zero barriers · Real-time captions · Fair attendance"
  />
</section>
