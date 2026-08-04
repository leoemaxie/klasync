<script lang="ts">
  import AudioLevelMeter from './AudioLevelMeter.svelte';
  import { triggerHaptic } from '$lib/native/haptics';

  let isUsingDeviceMic = $state(false);
  let audioLevel = $state(0);
  let micError = $state('');
  let stream: MediaStream | null = null;
  let animFrameId: number | null = null;

  async function enableDeviceMic() {
    triggerHaptic('medium');
    micError = '';
    try {
      stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      isUsingDeviceMic = true;
      triggerHaptic('success');
      startLevelMeter(stream);
    } catch {
      micError = 'Could not access browser microphone. Check permissions.';
      isUsingDeviceMic = false;
      triggerHaptic('error');
    }
  }

  function startLevelMeter(mediaStream: MediaStream) {
    const ctx = new AudioContext();
    const src = ctx.createMediaStreamSource(mediaStream);
    const analyzer = ctx.createAnalyser();
    analyzer.fftSize = 64;
    src.connect(analyzer);
    const data = new Uint8Array(analyzer.frequencyBinCount);
    function tick() {
      analyzer.getByteFrequencyData(data);
      audioLevel = Math.min(
        (data.reduce((a, b) => a + b, 0) / (data.length * 128)) * 100,
        100
      );
      animFrameId = requestAnimationFrame(tick);
    }
    tick();
  }

  function stopDeviceMic() {
    triggerHaptic('warning');
    if (stream) stream.getTracks().forEach((t) => t.stop());
    if (animFrameId) cancelAnimationFrame(animFrameId);
    isUsingDeviceMic = false;
    audioLevel = 0;
  }
</script>

<div class="panel mic-status-panel">
  <div class="mic-header">
    <p class="eyebrow">
      <span class="eyebrow-accent" aria-hidden="true">●</span> DEVICE &amp; AUDIO
    </p>
    <span class="status-badge" class:connected={isUsingDeviceMic} aria-live="polite">
      {isUsingDeviceMic ? 'DEVICE MIC ACTIVE' : 'KLASNYC MIC STANDBY'}
    </span>
  </div>
  <div class="mic-info-grid">
    <div class="mic-stat">
      <span class="stat-label">SOURCE</span><strong
        >{isUsingDeviceMic ? 'WebAudio' : 'Klasync Mic'}</strong
      >
    </div>
    <div class="mic-stat">
      <span class="stat-label">SAMPLE RATE</span><strong>48 kHz · 16-bit</strong
      >
    </div>
    <div class="mic-stat">
      <span class="stat-label">LATENCY</span><strong>&lt; 15 ms</strong>
    </div>
  </div>
  <AudioLevelMeter level={audioLevel} isActive={isUsingDeviceMic} />
  {#if micError}<p class="error" role="alert">{micError}</p>{/if}
  <div class="mic-actions">
    {#if !isUsingDeviceMic}
      <button type="button" class="outline full" onclick={enableDeviceMic}
        >Test &amp; Enable Microphone</button
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
  .mic-info-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
    border-block: 1px dashed var(--color-cork-border);
    padding: 10px 0;
  }
  .mic-stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .stat-label {
    font-size: 11px;
    letter-spacing: 0.08em;
    color: var(--color-driftwood);
  }
  .mic-stat strong {
    font-size: 11px;
    color: var(--color-warm-cream);
  }
  .mic-actions {
    display: flex;
    gap: 8px;
  }
  @media (max-width: 600px) {
    .mic-info-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
