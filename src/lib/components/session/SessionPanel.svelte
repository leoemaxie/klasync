<script lang="ts">
  import type { Session } from '$lib/types';
  import QrCodeSvg from '$lib/components/shared/QrCodeSvg.svelte';
  import ButtonSpinner from '$lib/components/shared/ButtonSpinner.svelte';
  import { pauseSession, resumeSession, toggleRecording } from '$lib/api';
  import {
    Play,
    Pause,
    Circle,
    Mic,
    Copy,
    Check,
    Radio,
    Link as LinkIcon,
    Power,
  } from '@lucide/svelte';

  let {
    session,
    apiNotice = '',
    isSaving = false,
    copied = false,
    lecturerName = '',
    lecturerEmail = '',
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

  let isPaused = $state(false);
  let isRecording = $state(false);
  let isTogglingPause = $state(false);
  let isTogglingRec = $state(false);
  let actionError = $state('');
  let copiedCode = $state(false);

  const inviteUrl = $derived(
    session?.code
      ? `${typeof location !== 'undefined' ? location.origin : ''}/#/?join=${session.code}`
      : ''
  );

  function handleCopyCode() {
    if (!session?.code) return;
    try {
      navigator.clipboard.writeText(session.code);
      copiedCode = true;
      setTimeout(() => (copiedCode = false), 2000);
    } catch {
      // Fallback if clipboard API unavailable
    }
  }

  async function handlePauseToggle() {
    if (!session?.code) return;
    actionError = '';
    isTogglingPause = true;
    try {
      if (isPaused) {
        await resumeSession(session.code);
        isPaused = false;
      } else {
        await pauseSession(session.code);
        isPaused = true;
      }
    } catch (err) {
      actionError =
        err instanceof Error ? err.message : 'Failed to toggle session state';
    } finally {
      isTogglingPause = false;
    }
  }

  async function handleRecordingToggle() {
    if (!session?.code) return;
    actionError = '';
    isTogglingRec = true;
    try {
      await toggleRecording(session.code, !isRecording);
      isRecording = !isRecording;
    } catch (err) {
      actionError =
        err instanceof Error ? err.message : 'Failed to toggle recording';
    } finally {
      isTogglingRec = false;
    }
  }
</script>

<div class="session-dashboard">
  {#if session?.live}
    <!-- Top Header & Live Status Bar -->
    <div class="session-top-card">
      <div class="header-main">
        <p class="eyebrow">LIVE SESSION DASHBOARD</p>
        <h2 class="session-title">{session.title}</h2>
      </div>

      <div class="live-status-pills">
        {#if isPaused}
          <span class="status-pill paused">
            <Pause size={12} /> ROOM PAUSED
          </span>
        {:else}
          <span class="status-pill broadcasting">
            <span class="live-dot"></span> LIVE BROADCASTING
          </span>
        {/if}

        {#if isRecording}
          <span class="status-pill recording">
            <span class="rec-dot"></span> RECORDING ACTIVE
          </span>
        {:else}
          <span class="status-pill idle">
            <Mic size={12} /> MIC STANDBY
          </span>
        {/if}
      </div>
    </div>

    <!-- Access & Invites 2-Column Grid -->
    <div class="access-grid">
      <!-- Left Column: Code & Link -->
      <div class="access-card code-link-card">
        <div class="card-section">
          <p class="section-label">STUDENT ACCESS CODE</p>
          <div class="code-box">
            <span class="code-display">{session.code}</span>
            <button
              type="button"
              class="copy-code-btn"
              onclick={handleCopyCode}
              title="Copy 8-character session code"
            >
              {#if copiedCode}
                <Check size={13} class="success-icon" /> Copied
              {:else}
                <Copy size={13} /> Copy Code
              {/if}
            </button>
          </div>
        </div>

        <div class="card-divider"></div>

        <div class="card-section">
          <p class="section-label">DIRECT INVITE LINK</p>
          <div class="url-input-wrap">
            <LinkIcon size={14} class="link-icon" />
            <input readonly value={inviteUrl} class="url-input" />
            <button
              type="button"
              class="outline copy-link-btn"
              onclick={onCopyInvite}
            >
              {#if copied}
                <Check size={13} class="success-icon" /> Copied
              {:else}
                <Copy size={13} /> Copy Link
              {/if}
            </button>
          </div>
        </div>

        <p class="access-hint">
          Students can join by entering this 8-character code, clicking the direct link, or scanning the QR code.
        </p>
      </div>

      <!-- Right Column: Projector QR Code -->
      <div class="access-card qr-card">
        <p class="section-label">CLASSROOM QR INVITE</p>
        <div class="qr-frame">
          <QrCodeSvg value={inviteUrl || session.code} size={145} />
        </div>
        <p class="qr-hint">Display on projector for quick mobile scan</p>
      </div>
    </div>

    <!-- Room Action Controls Toolbar -->
    <div class="controls-card">
      <p class="section-label">LECTURER BROADCAST CONTROLS</p>

      <div class="action-buttons-grid">
        <button
          type="button"
          class="control-btn pause-toggle"
          class:is-active-paused={isPaused}
          onclick={handlePauseToggle}
          disabled={isTogglingPause}
        >
          {#if isTogglingPause}
            <ButtonSpinner label="Updating status..." />
          {:else if isPaused}
            <Play size={16} /> Resume Lecture Room
          {:else}
            <Pause size={16} /> Pause Lecture Room
          {/if}
        </button>

        <button
          type="button"
          class="control-btn rec-toggle"
          class:is-active-rec={isRecording}
          onclick={handleRecordingToggle}
          disabled={isTogglingRec}
        >
          {#if isTogglingRec}
            <ButtonSpinner label="Updating recording..." />
          {:else if isRecording}
            <Circle size={13} fill="currentColor" class="rec-icon" /> Stop Recording
          {:else}
            <Mic size={16} /> Start Audio Recording
          {/if}
        </button>
      </div>
    </div>

    {#if actionError || apiNotice}
      <div class="error-notice">
        {actionError || apiNotice}
      </div>
    {/if}

    <!-- Destructive Action Footer -->
    <div class="end-session-row">
      <button
        type="button"
        class="end-session-btn"
        onclick={onEndSession}
      >
        <Power size={15} /> End Live Lecture Session
      </button>
    </div>
  {:else}
    <!-- Pre-Session Start State -->
    <div class="start-session-card">
      <div class="start-header">
        <p class="eyebrow">READY TO BROADCAST</p>
        <h2 class="start-title">Start Live Lecture Session</h2>
        <p class="start-desc">
          Generate an authoritative 8-character access code, invite link, and QR code for student presence verification and live captioning.
        </p>
      </div>

      <div class="lecturer-summary-box">
        <div class="summary-item">
          <span class="sum-label">LECTURER NAME:</span>
          <span class="sum-val">{lecturerName || 'Not Set'}</span>
        </div>
        <div class="summary-item">
          <span class="sum-label">CONTACT EMAIL:</span>
          <span class="sum-val">{lecturerEmail || 'Not Set'}</span>
        </div>
      </div>

      {#if apiNotice}
        <p class="error-notice">{apiNotice}</p>
      {/if}

      <button
        type="button"
        class="primary start-btn"
        onclick={onStartSession}
        disabled={!lecturerName.trim() || !lecturerEmail.trim() || isSaving}
      >
        {#if isSaving}
          <ButtonSpinner label="Initializing live lecture room..." /> Initializing Live Room...
        {:else}
          <Radio size={16} /> Start Live Session Now
        {/if}
      </button>
    </div>
  {/if}
</div>

<style>
  .session-dashboard {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-16);
  }

  /* Top Card */
  .session-top-card {
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 8px;
    padding: var(--spacing-16) var(--spacing-20);
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--spacing-14);
    flex-wrap: wrap;
  }

  .session-title {
    font-size: 24px;
    font-weight: 500;
    color: var(--color-warm-cream);
    margin: 4px 0 0 0;
    letter-spacing: -0.01em;
  }

  .live-status-pills {
    display: flex;
    align-items: center;
    gap: var(--spacing-8);
    flex-wrap: wrap;
  }

  .status-pill {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    padding: 6px 12px;
    border-radius: 20px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    text-transform: uppercase;
  }

  .status-pill.broadcasting {
    background: rgba(74, 183, 114, 0.12);
    color: #4ab772;
    border: 1px solid rgba(74, 183, 114, 0.3);
  }

  .live-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #4ab772;
    box-shadow: 0 0 8px #4ab772;
    animation: blink 1.5s infinite;
  }

  @keyframes blink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }

  .status-pill.paused {
    background: rgba(220, 80, 0, 0.12);
    color: var(--color-ember-accent);
    border: 1px solid rgba(220, 80, 0, 0.3);
  }

  .status-pill.recording {
    background: rgba(239, 68, 68, 0.15);
    color: #f87171;
    border: 1px solid rgba(239, 68, 68, 0.3);
  }

  .rec-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #ef4444;
    box-shadow: 0 0 8px #ef4444;
    animation: blink 1s infinite;
  }

  .status-pill.idle {
    background: rgba(56, 36, 22, 0.6);
    color: var(--color-driftwood);
    border: 1px solid var(--color-cork-border);
  }

  /* Access Grid */
  .access-grid {
    display: grid;
    grid-template-columns: 1.6fr 1fr;
    gap: var(--spacing-16);
  }

  @media (max-width: 768px) {
    .access-grid {
      grid-template-columns: 1fr;
    }
  }

  .access-card {
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 8px;
    padding: var(--spacing-18);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14);
  }

  .section-label {
    font-size: 9px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--color-driftwood);
    font-weight: 600;
    margin: 0 0 6px 0;
  }

  .code-box {
    display: flex;
    align-items: center;
    gap: var(--spacing-12);
    flex-wrap: wrap;
  }

  .code-display {
    font-family: monospace;
    font-size: 26px;
    font-weight: 700;
    letter-spacing: 0.18em;
    color: var(--color-warm-cream);
    background: rgba(10, 5, 2, 0.8);
    border: 1px solid var(--color-cork-border);
    padding: 8px 18px;
    border-radius: 6px;
    line-height: 1.2;
  }

  .copy-code-btn {
    background: var(--color-bark-brown);
    border: 1px solid var(--color-cork-border);
    color: var(--color-warm-cream);
    padding: 8px 14px;
    border-radius: 6px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    transition: all 0.2s ease;
  }

  .copy-code-btn:hover {
    border-color: var(--color-warm-cream);
    background: rgba(56, 36, 22, 0.9);
  }

  .card-divider {
    height: 1px;
    background: var(--color-cork-border);
    opacity: 0.5;
  }

  .url-input-wrap {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  :global(.link-icon) {
    position: absolute;
    left: 10px;
    color: var(--color-driftwood);
    pointer-events: none;
  }

  .url-input {
    width: 100%;
    padding: 8px 12px 8px 32px;
    background: rgba(10, 5, 2, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    color: var(--color-warm-cream);
    font-size: 11px;
    font-family: monospace;
  }

  .copy-link-btn {
    font-size: 11px;
    padding: 7px 14px;
    white-space: nowrap;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .access-hint {
    font-size: 11px;
    color: var(--color-driftwood);
    margin: 0;
    line-height: 1.4;
  }

  /* QR Card */
  .qr-card {
    align-items: center;
    text-align: center;
    justify-content: center;
  }

  .qr-frame {
    padding: 10px;
    background: var(--color-warm-cream);
    border-radius: 8px;
    display: inline-flex;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  }

  .qr-hint {
    font-size: 11px;
    color: var(--color-driftwood);
    margin: 0;
  }

  /* Broadcast Controls */
  .controls-card {
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 8px;
    padding: var(--spacing-18);
  }

  .action-buttons-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-14);
    margin-top: 8px;
  }

  @media (max-width: 600px) {
    .action-buttons-grid {
      grid-template-columns: 1fr;
    }
  }

  .control-btn {
    background: rgba(56, 36, 22, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    color: var(--color-warm-cream);
    padding: 12px 18px;
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0.04em;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    transition: all 0.2s ease;
  }

  .control-btn:hover:not(:disabled) {
    border-color: var(--color-warm-cream);
    background: var(--color-bark-brown);
  }

  .control-btn.is-active-paused {
    background: rgba(220, 80, 0, 0.15);
    border-color: var(--color-ember-accent);
    color: var(--color-warm-cream);
  }

  .control-btn.is-active-rec {
    background: rgba(239, 68, 68, 0.15);
    border-color: #ef4444;
    color: #f87171;
  }

  /* Error notice */
  .error-notice {
    font-size: 12px;
    color: var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.1);
    border: 1px solid var(--color-cork-border);
    padding: 10px 14px;
    border-radius: 6px;
    margin: 0;
  }

  /* End session */
  .end-session-row {
    display: flex;
    justify-content: flex-end;
    margin-top: 4px;
  }

  .end-session-btn {
    background: transparent;
    border: 1px solid rgba(220, 80, 0, 0.4);
    color: var(--color-ember-accent);
    padding: 9px 18px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    transition: all 0.2s ease;
  }

  .end-session-btn:hover {
    background: rgba(220, 80, 0, 0.15);
    border-color: var(--color-ember-accent);
  }

  /* Start state */
  .start-session-card {
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 8px;
    padding: var(--spacing-28);
    display: flex;
    flex-direction: column;
    gap: var(--spacing-18);
  }

  .start-title {
    font-size: 24px;
    font-weight: 500;
    color: var(--color-warm-cream);
    margin: 4px 0 8px 0;
  }

  .start-desc {
    font-size: 13px;
    color: var(--color-driftwood);
    margin: 0;
    line-height: 1.5;
  }

  .lecturer-summary-box {
    display: flex;
    gap: var(--spacing-20);
    background: rgba(10, 5, 2, 0.6);
    border: 1px solid var(--color-cork-border);
    padding: 12px 16px;
    border-radius: 6px;
    flex-wrap: wrap;
  }

  .summary-item {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }

  .sum-label {
    color: var(--color-driftwood);
    font-size: 10px;
    letter-spacing: 0.08em;
    font-weight: 600;
  }

  .sum-val {
    color: var(--color-warm-cream);
    font-weight: 500;
  }

  .start-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 12px 24px;
    font-size: 13px;
    align-self: flex-start;
  }
</style>

