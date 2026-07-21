<script lang="ts">
  import AudioLevelMeter from "./AudioLevelMeter.svelte";

  let isHardwareConnected = $state(false);
  let isUsingDeviceMic = $state(false);
  let audioLevel = $state(0);
  let micError = $state("");
  let stream: MediaStream | null = null;
  let animFrameId: number | null = null;

  async function enableDeviceMic() {
    micError = "";
    try {
      stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      isUsingDeviceMic = true;
      startLevelMeter(stream);
    } catch (err) {
      micError = "Could not access browser microphone. Check device permissions.";
      isUsingDeviceMic = false;
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
      const avg = data.reduce((a, b) => a + b, 0) / data.length;
      audioLevel = Math.min((avg / 128) * 100, 100);
      animFrameId = requestAnimationFrame(tick);
    }
    tick();
  }

  function stopDeviceMic() {
    if (stream) {
      stream.getTracks().forEach((t) => t.stop());
      stream = null;
    }
    if (animFrameId) cancelAnimationFrame(animFrameId);
    isUsingDeviceMic = false;
    audioLevel = 0;
  }
</script>

<div class="panel mic-status-panel">
  <div class="mic-header">
    <p class="eyebrow">
      <span class="eyebrow-accent">●</span> AUDIO INPUT SOURCE &amp; HARDWARE HEALTH
    </p>
    <span class="status-badge" class:connected={isHardwareConnected || isUsingDeviceMic}>
      {isHardwareConnected ? "KLASYNC MIC ACTIVE" : isUsingDeviceMic ? "DEVICE MIC FALLBACK" : "HARDWARE STANDBY"}
    </span>
  </div>

  <div class="mic-info-grid">
    <div class="mic-stat">
      <span class="stat-label">HARDWARE DEVICE</span>
      <strong>Klasync Mic (ESP32-S3)</strong>
    </div>
    <div class="mic-stat">
      <span class="stat-label">TRANSPORT</span>
      <strong>{isHardwareConnected ? "2.4GHz Direct Stream" : isUsingDeviceMic ? "WebAudio Fallback" : "Idle"}</strong>
    </div>
    <div class="mic-stat">
      <span class="stat-label">SAMPLE RATE</span>
      <strong>48 kHz · 16-bit PCM</strong>
    </div>
  </div>

  <AudioLevelMeter level={audioLevel} isActive={isHardwareConnected || isUsingDeviceMic} />

  {#if micError}
    <p class="error">{micError}</p>
  {/if}

  <div class="mic-actions">
    {#if !isUsingDeviceMic}
      <button type="button" class="outline full" onclick={enableDeviceMic}>
        Fallback to Device Microphone
      </button>
    {:else}
      <button type="button" class="danger full" onclick={stopDeviceMic}>
        Disconnect Device Microphone
      </button>
    {/if}
  </div>

  <p class="hint">
    Hardware Philosophy: When powered on, the Klasync wireless mic connects automatically without pairing or configuration.
  </p>
</div>

<style>
  .mic-status-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-18);
  }
  .mic-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--spacing-12);
  }
  .status-badge {
    font-size: 9px;
    letter-spacing: 0.14em;
    padding: 3px 8px;
    border-radius: 4px;
    border: 1px solid var(--color-cork-border);
    color: var(--color-driftwood);
  }
  .status-badge.connected {
    color: #4ab772;
    border-color: #4ab772;
  }
  .mic-info-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--spacing-12);
    border-top: 1px dashed var(--color-cork-border);
    border-bottom: 1px dashed var(--color-cork-border);
    padding: var(--spacing-12) 0;
  }
  .mic-stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .stat-label {
    font-size: 9px;
    letter-spacing: 0.1em;
    color: var(--color-driftwood);
  }
  .mic-stat strong {
    font-size: 12px;
    color: var(--color-warm-cream);
  }
  .mic-actions {
    display: flex;
    gap: var(--spacing-12);
  }
  @media (max-width: 600px) {
    .mic-info-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
