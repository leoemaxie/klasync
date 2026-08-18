<script lang="ts">
  import { onDestroy } from 'svelte';
  import { Mic } from '@lucide/svelte';
  import AudioLevelMeter from './AudioLevelMeter.svelte';
  import { triggerHaptic } from '$lib/native/haptics';
  import {
    startMicrophoneAudioStream,
    type AudioStreamer,
    type IngestedCaption,
  } from '$lib/api/audio';

  let {
    sessionCode = '',
    onCaptionIngested,
  }: {
    sessionCode?: string;
    onCaptionIngested?: (caption: IngestedCaption) => void;
  } = $props();

  let isUsingDeviceMic = $state(false);
  let isWsStreaming = $state(false);
  let audioLevel = $state(0);
  let micError = $state('');
  let stream: MediaStream | null = null;
  let animFrameId: number | null = null;
  let streamer: AudioStreamer | null = null;
  let audioCtx: AudioContext | null = null;
  let lastMeterUpdate = 0;

  async function enableDeviceMic() {
    triggerHaptic('medium');
    micError = '';
    try {
      stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      isUsingDeviceMic = true;
      triggerHaptic('success');
      startLevelMeter(stream);

      if (sessionCode) {
        streamer = startMicrophoneAudioStream(
          sessionCode,
          stream,
          (caption) => {
            onCaptionIngested?.(caption);
          },
          (err) => {
            micError = `Streaming notice: ${err.message}`;
            isWsStreaming = false;
          }
        );
        isWsStreaming = true;
      }
    } catch {
      micError = 'Could not access browser microphone. Check permissions.';
      isUsingDeviceMic = false;
      isWsStreaming = false;
      triggerHaptic('error');
    }
  }

  function startLevelMeter(mediaStream: MediaStream) {
    try {
      if (audioCtx) {
        audioCtx.close().catch(() => {});
      }
      audioCtx = new AudioContext();
      const src = audioCtx.createMediaStreamSource(mediaStream);
      const analyzer = audioCtx.createAnalyser();
      analyzer.fftSize = 64;
      src.connect(analyzer);
      const data = new Uint8Array(analyzer.frequencyBinCount);
      function tick(timestamp: number) {
        if (!isUsingDeviceMic) {
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

  function stopDeviceMic() {
    triggerHaptic('warning');
    if (streamer) {
      streamer.stop();
      streamer = null;
    }
    if (audioCtx) {
      audioCtx.close().catch(() => {});
      audioCtx = null;
    }
    if (stream) {
      stream.getTracks().forEach((t) => t.stop());
      stream = null;
    }
    if (animFrameId) {
      cancelAnimationFrame(animFrameId);
      animFrameId = null;
    }
    isUsingDeviceMic = false;
    isWsStreaming = false;
    audioLevel = 0;
  }

  onDestroy(() => {
    stopDeviceMic();
  });
</script>

<div class="panel mic-status-panel">
  <div class="mic-header">
    <div class="header-left">
      <div class="icon-circle">
        <Mic size={20} color="var(--color-ember-accent)" aria-hidden="true" />
      </div>
      <div class="header-text">
        <p class="eyebrow">DEVICE MIC</p>
      </div>
    </div>
    <span
      class="status-badge"
      class:connected={isUsingDeviceMic}
      class:streaming={isWsStreaming}
      aria-live="polite"
    >
      {#if isWsStreaming}
        STREAMING
      {:else if isUsingDeviceMic}
        MIC ON
      {:else}
        MIC OFF
      {/if}
    </span>
  </div>

  <div class="meter-container">
    <AudioLevelMeter level={audioLevel} isActive={isUsingDeviceMic} />
    {#if micError}<p class="error" role="alert">{micError}</p>{/if}
  </div>

  <div class="mic-actions">
    {#if !isUsingDeviceMic}
      <button type="button" class="outline full" onclick={enableDeviceMic}
        >{sessionCode
          ? 'Start Audio Stream'
          : 'Test Microphone'}</button
      >
    {:else}
      <button type="button" class="danger full" onclick={stopDeviceMic}
        >Stop Microphone</button
      >
    {/if}
  </div>
</div>

<style>
  .mic-status-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14);
  }
  .mic-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
  }
  .header-left {
    display: flex;
    align-items: center;
    gap: var(--spacing-12);
  }
  .icon-circle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    background: rgba(220, 80, 0, 0.1);
    border: 1px solid rgba(220, 80, 0, 0.2);
    border-radius: 50%;
  }
  .header-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .status-badge {
    font-size: 11px;
    letter-spacing: 0.12em;
    padding: 3px 8px;
    border-radius: 4px;
    border: 1px solid var(--color-cork-border);
    color: var(--color-driftwood);
  }
  .status-badge.connected {
    color: var(--color-warm-cream);
    border-color: #4ab772;
    background: rgba(74, 183, 114, 0.1);
  }
  .meter-container {
    margin-block: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .mic-actions {
    display: flex;
    gap: 8px;
  }
</style>
