<script lang="ts">
  let { title = "ACCESSIBLE LEARNING INFRASTRUCTURE", subtitle = "Live Audio · Real-time Captions · Searchable Archive" }: { title?: string; subtitle?: string } = $props();
</script>

<div class="public-visual-panel" aria-hidden="true">
  <!-- Glowing signal arcs in background -->
  <div class="visual-arc-bg">
    <svg viewBox="0 0 400 400" preserveAspectRatio="xMidYMid meet" xmlns="http://www.w3.org/2000/svg">
      <circle cx="200" cy="200" r="180" fill="none" stroke="#dc5000" stroke-width="1.2" opacity="0.25" stroke-dasharray="6 6"/>
      <circle cx="200" cy="200" r="130" fill="none" stroke="#ffedd7" stroke-width="0.8" opacity="0.15"/>
      <circle cx="200" cy="200" r="80" stroke="#dc5000" stroke-width="1.5" opacity="0.3" fill="none"/>
    </svg>
  </div>

  <!-- Background K stroke -->
  <div class="visual-k-bg">K</div>

  <!-- Animated Waveform Bars -->
  <div class="waveform-container">
    <div class="waveform-bars">
      {#each [0.35, 0.65, 0.95, 0.75, 0.45, 0.85, 1, 0.6, 0.8, 0.5, 0.9, 0.7, 0.4, 0.8, 0.6, 0.95, 0.55, 0.75] as h, i}
        <div class="bar" style="--h: {h}; --delay: {i * 0.07}s;"></div>
      {/each}
    </div>
    <p class="visual-status-pill">
      <span class="pulse-dot">●</span> REAL-TIME AUDIO CAPTION STREAM
    </p>
  </div>

  <!-- Simulated Live Captions Card -->
  <div class="caption-preview-card">
    <div class="card-header">
      <span class="tag">KLASYNC ACCESSIBILITY ENGINE</span>
      <span class="live-tag">LIVE</span>
    </div>
    <p class="preview-text">
      "...transforming physical lectures into structured, accessible, and searchable study materials in real-time."
    </p>
    <div class="preview-metrics">
      <span>LATENCY: 120ms</span>
      <span>ACCURACY: 99.4%</span>
      <span>VOICE-TO-TEXT</span>
    </div>
  </div>

  <!-- Subtitle Label -->
  <div class="visual-footer">
    <p class="eyebrow">{title}</p>
    <p class="visual-subtitle">{subtitle}</p>
  </div>
</div>
