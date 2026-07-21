<script lang="ts">
  let { visible = false }: { visible?: boolean } = $props();
</script>

<div class="hero-visual" class:visible aria-hidden="true">
  <div class="waveform">
    {#each [0.4, 0.7, 1, 0.85, 0.6, 0.95, 0.5, 0.75, 0.9, 0.55, 0.8, 0.65, 1, 0.45, 0.7] as h, i}
      <div class="bar" style="--h: {h}; --delay: {i * 0.08}s;"></div>
    {/each}
  </div>
  <div class="visual-label">LIVE · CAPTIONED · ARCHIVED</div>
  <div class="hero-k" aria-hidden="true">K</div>
</div>
