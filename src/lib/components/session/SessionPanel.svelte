<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { Session } from '$lib/types';
  import ButtonSpinner from '$lib/components/shared/ButtonSpinner.svelte';
  import SessionTopHeader from './SessionTopHeader.svelte';
  import SessionAccessCard from './SessionAccessCard.svelte';
  import SessionControlsCard from './SessionControlsCard.svelte';
  import { pauseSession, resumeSession, toggleRecording } from '$lib/api';
  import {
    startMicrophoneAudioStream,
    type AudioStreamer,
    type IngestedCaption,
  } from '$lib/api/audio';
  import { Radio } from '@lucide/svelte';
  import LiveQaPanel from './LiveQaPanel.svelte';

  let {
    session,
    apiNotice = '',
    isSaving = false,
    isEndingSession = false,
    copied = false,
    lecturerName = '',
    lecturerEmail = '',
    onCopyInvite,
    onEndSession,
    onStartSession,
    onCaptionIngested,
  }: {
    session: Session | null;
    apiNotice?: string;
    isSaving?: boolean;
    isEndingSession?: boolean;
    copied?: boolean;
    lecturerName?: string;
    lecturerEmail?: string;
    onCopyInvite: () => void;
    onEndSession: () => void | Promise<void>;
    onStartSession: () => void;
    onCaptionIngested?: (caption: IngestedCaption) => void;
  } = $props();

  let isPaused = $state(false);
  let isRecording = $state(false);
  let isTogglingPause = $state(false);
  let isTogglingRec = $state(false);
  let isEnding = $state(false);
  let actionError = $state('');

  let audioLevel = $state(0);
  let mediaStream: MediaStream | null = null;
  let streamer: AudioStreamer | null = null;
  let animFrameId: number | null = null;

  const inviteUrl = $derived(
    session?.code
      ? `${typeof location !== 'undefined' ? location.origin : ''}/#/?join=${session.code}`
      : ''
  );

  function startLevelMeter(stream: MediaStream) {
    try {
      const ctx = new AudioContext();
      const src = ctx.createMediaStreamSource(stream);
      const analyzer = ctx.createAnalyser();
      analyzer.fftSize = 64;
      src.connect(analyzer);
      const data = new Uint8Array(analyzer.frequencyBinCount);
      function tick() {
        if (!isRecording) {
          audioLevel = 0;
          return;
        }
        analyzer.getByteFrequencyData(data);
        audioLevel = Math.min(
          (data.reduce((a, b) => a + b, 0) / (data.length * 128)) * 100,
          100
        );
        animFrameId = requestAnimationFrame(tick);
      }
      tick();
    } catch {
      audioLevel = 50;
    }
  }

  function stopAudioStream() {
    if (streamer) {
      streamer.stop();
      streamer = null;
    }
    if (mediaStream) {
      mediaStream.getTracks().forEach((t) => t.stop());
      mediaStream = null;
    }
    if (animFrameId) {
      cancelAnimationFrame(animFrameId);
      animFrameId = null;
    }
    audioLevel = 0;
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
      if (!isRecording) {
        try {
          mediaStream = await navigator.mediaDevices.getUserMedia({
            audio: true,
          });
          startLevelMeter(mediaStream);
          streamer = startMicrophoneAudioStream(
            session.code,
            mediaStream,
            (caption) => {
              onCaptionIngested?.(caption);
            },
            (err) => {
              actionError = `Audio stream notice: ${err.message}`;
            }
          );
        } catch {
          actionError =
            'Could not access browser microphone. Check device permissions.';
          return;
        }
        await toggleRecording(session.code, true);
        isRecording = true;
      } else {
        stopAudioStream();
        await toggleRecording(session.code, false);
        isRecording = false;
      }
    } catch (err) {
      actionError =
        err instanceof Error ? err.message : 'Failed to toggle recording';
    } finally {
      isTogglingRec = false;
    }
  }

  async function handleEndSession() {
    isEnding = true;
    try {
      stopAudioStream();
      await onEndSession();
    } finally {
      isEnding = false;
    }
  }

  onDestroy(() => {
    stopAudioStream();
  });
</script>

<div class="session-dashboard">
  {#if session?.live}
    <SessionTopHeader
      title={session.title}
      {isPaused}
      {isRecording}
      {audioLevel}
    />
    <SessionAccessCard
      code={session.code}
      {inviteUrl}
      {copied}
      {onCopyInvite}
    />
    <SessionControlsCard
      {isPaused}
      {isRecording}
      {isTogglingPause}
      {isTogglingRec}
      isEndingSession={isEndingSession || isEnding || isSaving}
      {actionError}
      {apiNotice}
      onPauseToggle={handlePauseToggle}
      onRecordingToggle={handleRecordingToggle}
      onEndSession={handleEndSession}
    />
    <LiveQaPanel sessionCode={session.code} isLecturer={true} />
  {:else}
    <div class="start-session-card">
      <div class="start-header">
        <p class="eyebrow">READY TO BROADCAST</p>
        <h2 class="start-title">Start Live Lecture Session</h2>
        <p class="start-desc">
          Generate access code, invite link, and QR code for student entry.
        </p>
      </div>
      <div class="lecturer-summary-box">
        <div class="summary-item">
          <span class="sum-label">LECTURER:</span><span class="sum-val"
            >{lecturerName || 'Not Set'}</span
          >
        </div>
        <div class="summary-item">
          <span class="sum-label">EMAIL:</span><span class="sum-val"
            >{lecturerEmail || 'Not Set'}</span
          >
        </div>
      </div>
      {#if apiNotice}<p class="error-notice">{apiNotice}</p>{/if}
      <button
        type="button"
        class="primary start-btn"
        onclick={onStartSession}
        disabled={!lecturerName.trim() || !lecturerEmail.trim() || isSaving}
      >
        {#if isSaving}<ButtonSpinner
            label="Initializing live lecture room..."
          /> Initializing Live Room...{:else}<Radio size={16} /> Start Live Session
          Now{/if}
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
    font-family: var(--font-display);
  }
  .start-desc {
    font-size: 13px;
    color: var(--color-driftwood);
    margin: 0;
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
