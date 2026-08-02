<script lang="ts">
  let { level = 0, isActive = false }: { level: number; isActive: boolean } =
    $props();
</script>

<div class="audio-level-meter" aria-hidden="true">
  <div class="meter-bar-track">
    {#each Array(12) as _, i}
      <div
        class="meter-segment"
        class:active={isActive && level > (i / 12) * 100}
        class:peak={i >= 10}
      ></div>
    {/each}
  </div>
  <span class="meter-readout"
    >{isActive ? `${Math.round(level)} dB` : 'MUTED'}</span
  >
</div>

<style>
  .audio-level-meter {
    display: flex;
    align-items: center;
    gap: var(--spacing-12);
    width: 100%;
  }

  .meter-bar-track {
    display: flex;
    gap: 3px;
    flex: 1;
    height: 14px;
    background: rgba(16, 9, 4, 0.6);
    padding: 2px;
    border-radius: 4px;
    border: 1px solid var(--color-cork-border);
  }

  .meter-segment {
    flex: 1;
    background: rgba(255, 237, 215, 0.1);
    border-radius: 1px;
    transition: background 0.1s ease;
  }

  .meter-segment.active {
    background: #4ab772;
  }

  .meter-segment.active.peak {
    background: var(--color-ember-accent);
  }

  .meter-readout {
    font-size: 10px;
    font-family: var(--font-body);
    letter-spacing: 0.1em;
    color: var(--color-driftwood);
    min-width: 50px;
    text-align: right;
  }
</style>
