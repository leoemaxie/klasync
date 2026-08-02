<script lang="ts">
  import MicStatusPanel from "./MicStatusPanel.svelte";
</script>

<div class="device-setup-tab">
  <MicStatusPanel />
  <div class="panel device-help-panel">
    <p class="eyebrow">AUDIO &amp; HARDWARE CONFIGURATION</p>
    <h3>Klasync Audio Pipeline</h3>
    <p class="hint">
      Connect your Klasync ESP32-S3 hardware microphone or enable browser WebAudio for live caption streaming. Audio levels are processed locally before streaming.
    </p>
    <div class="pipeline-features">
      <div class="feature-chip">✓ Automatic Gain Control</div>
      <div class="feature-chip">✓ Noise Suppression</div>
      <div class="feature-chip">✓ 16-bit 48kHz PCM</div>
    </div>
  </div>
</div>

<style>
  .device-setup-tab {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-24);
    align-items: start;
  }
  .device-help-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-12);
  }
  .pipeline-features {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 8px;
  }
  .feature-chip {
    font-size: 10px;
    letter-spacing: 0.06em;
    padding: 4px 10px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--color-cork-border);
    border-radius: 9999px;
    color: var(--color-warm-cream);
  }
  @media (max-width: 900px) {
    .device-setup-tab {
      grid-template-columns: 1fr;
    }
  }
</style>
