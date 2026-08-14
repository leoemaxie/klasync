<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { Session } from '$lib/types';
  import SessionTopHeader from './SessionTopHeader.svelte';
  import SessionAccessCard from './SessionAccessCard.svelte';
  import SessionControlsCard from './SessionControlsCard.svelte';
  import StartSessionCard from './StartSessionCard.svelte';
  import { pauseSession, resumeSession, toggleRecording } from '$lib/api';
  import {
    startMicrophoneAudioStream,
    type AudioStreamer,
    type IngestedCaption,
  } from '$lib/api/audio';
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
  let audioCtx: AudioContext | null = null;
  let animFrameId: number | null = null;
  let lastMeterUpdate = 0;

  const inviteUrl = $derived(
    session?.code
      ? `${typeof location !== 'undefined' ? location.origin : ''}/#/?join=${session.code}`
      : ''
  );

  function startLevelMeter(stream: MediaStream) {
    try {
      if (audioCtx) audioCtx.close().catch(() => {});
      audioCtx = new AudioContext();
      const src = audioCtx.createMediaStreamSource(stream);
      const analyzer = audioCtx.createAnalyser();
      analyzer.fftSize = 64;
      src.connect(analyzer);
      const data = new Uint8Array(analyzer.frequencyBinCount);
      function tick(timestamp: number) {
        if (!isRecording) {
          audioLevel = 0;
          return;
        }
        if (timestamp - lastMeterUpdate > 50) {
          analyzer.getByteFrequencyData(data);
          audioLevel = Math.min(
            (data.reduce((a, b) => a + b, 0) / (data.length * 128)) * 100,
            100
          );
          lastMeterUpdate = timestamp;
        }
        animFrameId = requestAnimationFrame(tick);
      }
      animFrameId = requestAnimationFrame(tick);
    } catch {
      audioLevel = 50;
    }
  }

  function stopAudioStream() {
    if (streamer) {
      streamer.stop();
      streamer = null;
    }
    if (audioCtx) {
      audioCtx.close().catch(() => {});
      audioCtx = null;
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
    } catch (err: any) {
      actionError = err?.message || 'Failed to toggle session state';
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
            (cap) => onCaptionIngested?.(cap),
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
    } catch (err: any) {
      actionError = err?.message || 'Failed to toggle recording';
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
    <StartSessionCard
      {lecturerName}
      {lecturerEmail}
      {apiNotice}
      {isSaving}
      {onStartSession}
    />
  {/if}
</div>

<style>
  .session-dashboard {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-20);
  }
</style>
