<script lang="ts">
  import MicStatusPanel from './MicStatusPanel.svelte';
  import { Bluetooth } from '@lucide/svelte';
  import type { IngestedCaption } from '$lib/api/audio';

  let {
    sessionCode = '',
    onCaptionIngested,
  }: {
    sessionCode?: string;
    onCaptionIngested?: (caption: IngestedCaption) => void;
  } = $props();

  function handlePairBluetooth() {
    alert('Bluetooth Web API integration coming soon!');
  }
</script>

<div class="device-setup-tab">
  <MicStatusPanel {sessionCode} {onCaptionIngested} />

  <div class="panel bluetooth-panel">
    <div class="bluetooth-header">
      <div class="icon-circle">
        <Bluetooth
          size={20}
          color="var(--color-ember-accent)"
          aria-hidden="true"
        />
      </div>
      <div class="header-text">
        <p class="eyebrow">CONNECT KLASYNC MIC</p>
      </div>
    </div>
    <p class="hint">
      Pair your dedicated Klasync lapel microphone via Bluetooth for optimal
      studio audio quality, ultra-low latency, and enhanced noise suppression.
    </p>
    <div class="bt-actions">
      <button type="button" class="primary full" onclick={handlePairBluetooth}>
        Pair New Device
      </button>
    </div>
  </div>
</div>

<style>
  .device-setup-tab {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-24);
    align-items: stretch;
  }
  .bluetooth-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-16);
  }
  .bluetooth-header {
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
  .bt-actions {
    margin-top: auto;
  }
  @media (max-width: 900px) {
    .device-setup-tab {
      grid-template-columns: 1fr;
    }
  }
</style>
