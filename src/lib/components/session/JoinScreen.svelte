<script lang="ts">
  import { lookupSessionByCode } from '$lib/api/sessions';
  import PublicVisualPanel from '$lib/components/shared/PublicVisualPanel.svelte';
  import Skeleton from '$lib/components/shared/Skeleton.svelte';
  import ButtonSpinner from '$lib/components/shared/ButtonSpinner.svelte';
  import { triggerHaptic } from '$lib/native/haptics';

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
    triggerHaptic('light');
    isCheckingCode = true;
    try {
      const info = await lookupSessionByCode(sessionCode.trim().toUpperCase());
      sessionTitle = info.session.title;
      sessionStatus = info.session.status;
      triggerHaptic('success');
    } catch {
      sessionTitle = '';
      sessionStatus = 'idle';
    } finally {
      isCheckingCode = false;
    }
  }

  async function handleJoin() {
    triggerHaptic('medium');
    isJoining = true;
    try {
      await onJoinSession();
    } finally {
      isJoining = false;
    }
  }
</script>

<svelte:head><title>Join Session — Klasync</title></svelte:head>

<section class="join-wrap">
  <div class="join-left-content">
    <p class="eyebrow">GUEST ACCESS</p>
    <h1>Join Session</h1>
    <div class="join-card panel">
      <label for="session-code-input">
        Session short code
        <input
          id="session-code-input"
          type="text"
          inputmode="text"
          autocapitalize="characters"
          autocorrect="off"
          spellcheck="false"
          bind:value={sessionCode}
          onblur={checkCode}
          placeholder="e.g. A4K9QZ"
          maxlength="8"
        />
      </label>

      {#if isCheckingCode}
        <Skeleton height="36px" label="Verifying session code..." />
      {:else if sessionTitle}
        <div class="session-info-badge">
          <p class="eyebrow">
            <span class="eyebrow-accent">●</span>
            {sessionStatus.toUpperCase()}
          </p>
          <h2>{sessionTitle}</h2>
        </div>
      {/if}

      <label for="matric-input">
        Matric / Student ID
        <input
          id="matric-input"
          type="text"
          autocapitalize="characters"
          bind:value={matric}
          placeholder="MAT/2023/001"
        />
      </label>

      <label for="display-name-input">
        Full name <span>(optional if on roster)</span>
        <input
          id="display-name-input"
          type="text"
          bind:value={displayName}
          placeholder="Ada Okafor"
        />
      </label>

      {#if joinError}<p class="error" role="alert">{joinError}</p>{/if}

      <button
        class="primary full"
        onclick={handleJoin}
        disabled={sessionStatus === 'ended' || !matric.trim() || isJoining}
      >
        {#if isJoining}<ButtonSpinner label="Verifying..." /> Entering...{:else if sessionStatus === 'ended'}Session
          Ended{:else}Join lecture{/if}
      </button>
      <p class="hint">Matric number verified against course roster.</p>
    </div>
  </div>
  <PublicVisualPanel
    title="INSTANT GUEST ACCESS"
    subtitle="Zero barriers · Real-time captions · Fair attendance"
  />
</section>
