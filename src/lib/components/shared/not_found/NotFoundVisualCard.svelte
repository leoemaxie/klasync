<script lang="ts">
  import { Archive } from '@lucide/svelte';
</script>

<div class="panel not-found-visual-card">
  <div class="visual-header">
    <span class="telemetry-tag">DIAGNOSTICS</span>
    <div class="big-404-wrap">
      <h2 class="big-404-text">404</h2>
      <p class="big-404-sub">RESOURCE_UNRESOLVED</p>
    </div>
  </div>

  <div class="telemetry-grid">
    <div class="telemetry-item">
      <span class="tel-label">ROUTER</span>
      <span class="tel-val">SPA_DYNAMIC_RESOLVER</span>
    </div>
    <div class="telemetry-item">
      <span class="tel-label">HTTP STATUS</span>
      <span class="tel-val status-404">404 NOT FOUND</span>
    </div>
    <div class="telemetry-item">
      <span class="tel-label">LOCATION</span>
      <span class="tel-val">klasync://workspace</span>
    </div>
  </div>

  <div class="visual-footer-box">
    <div class="footer-box-header">
      <Archive size={16} color="var(--color-ember-accent)" />
      <strong>Hosting a Lecture?</strong>
    </div>
    <p class="hint">
      Sign in to your lecturer account to initialize a new live room and invite students.
    </p>
  </div>
</div>

<style>
  .not-found-visual-card {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: var(--spacing-20);
    padding: var(--spacing-28);
    background: rgba(16, 9, 4, 0.45);
    border: 1px solid var(--color-cork-border);
    border-radius: 12px;
  }
  .visual-header { display: flex; flex-direction: column; gap: 8px; }
  .telemetry-tag { font-size: 10px; letter-spacing: 0.14em; color: var(--color-ember-accent); font-weight: 700; text-transform: uppercase; }
  .big-404-wrap { display: flex; flex-direction: column; align-items: flex-start; }
  .big-404-text { font-family: var(--font-display); font-size: clamp(64px, 8vw, 84px); line-height: 0.9; margin: 0; color: var(--color-warm-cream); opacity: 0.9; }
  .big-404-sub { font-size: 11px; letter-spacing: 0.16em; color: var(--color-driftwood); margin: 6px 0 0 0; }
  .telemetry-grid { display: grid; grid-template-columns: 1fr; gap: var(--spacing-8); padding: var(--spacing-14); background: rgba(8, 4, 2, 0.6); border-radius: 8px; border: 1px solid var(--color-cork-border); }
  .telemetry-item { display: flex; justify-content: space-between; font-size: 11px; }
  .tel-label { color: var(--color-driftwood); letter-spacing: 0.08em; }
  .tel-val { color: var(--color-warm-cream); font-family: var(--font-mono, monospace); }
  .status-404 { color: var(--color-ember-accent); }
  .visual-footer-box { padding: var(--spacing-14); background: rgba(16, 9, 4, 0.35); border: 1px dashed var(--color-cork-border); border-radius: 8px; display: flex; flex-direction: column; gap: 4px; }
  .footer-box-header { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--color-warm-cream); }
  .hint { margin: 0; font-size: 11px; color: var(--color-driftwood); line-height: 1.4; }
</style>
