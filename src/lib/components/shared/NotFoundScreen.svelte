<script lang="ts">
  import type { Screen } from '$lib/types';
  import {
    Home,
    Radio,
    UserCheck,
    Archive,
    Search,
    Compass,
    AlertCircle,
    ArrowRight,
  } from '@lucide/svelte';

  let { screen = $bindable() }: { screen: Screen } = $props();

  let retryCode = $state('');

  function handleRetryJoin(e: SubmitEvent) {
    e.preventDefault();
    if (!retryCode.trim()) return;
    window.location.hash = `#/join?code=${encodeURIComponent(retryCode.trim().toUpperCase())}`;
    screen = 'join';
  }
</script>

<div class="not-found-container">
  <section class="not-found-grid">
    <!-- Left Main Card -->
    <div class="panel not-found-card">
      <div class="card-header">
        <div class="status-badge">
          <Compass size={15} color="var(--color-ember-accent)" />
          <span>404 // ROOM UNRESOLVED</span>
        </div>
        <h1 class="card-title">Room or Route Not Found</h1>
        <p class="card-desc">
          The lecture session code, invite link, or requested resource does not exist or has expired.
        </p>
      </div>

      <form class="retry-code-box" onsubmit={handleRetryJoin}>
        <label for="nf-retry-input" class="retry-label">Have a session code?</label>
        <div class="retry-input-row">
          <div class="search-input-wrap">
            <Search size={15} class="input-icon" />
            <input
              id="nf-retry-input"
              type="text"
              bind:value={retryCode}
              placeholder="e.g. KL-3942"
              maxlength="10"
              autocomplete="off"
            />
          </div>
          <button type="submit" class="retry-btn" disabled={!retryCode.trim()}>
            <span>Try Code</span>
            <ArrowRight size={14} />
          </button>
        </div>
      </form>

      <div class="diagnostic-checklist">
        <p class="checklist-title">POSSIBLE CAUSES &amp; CHECKS:</p>
        <ul class="checklist-items">
          <li>
            <AlertCircle size={13} color="var(--color-driftwood)" />
            <span>The lecturer may have ended the live session.</span>
          </li>
          <li>
            <AlertCircle size={13} color="var(--color-driftwood)" />
            <span>Check for typos in the 6-character session code.</span>
          </li>
        </ul>
      </div>

      <div class="action-buttons-group">
        <button
          type="button"
          class="nf-action-btn primary"
          onclick={() => (screen = 'home')}
        >
          <Home size={15} />
          <span>Return Home</span>
        </button>
        <button
          type="button"
          class="nf-action-btn outline"
          onclick={() => (screen = 'join')}
        >
          <Radio size={15} />
          <span>Join Session</span>
        </button>
        <button
          type="button"
          class="nf-action-btn secondary"
          onclick={() => (screen = 'lecturer-login')}
        >
          <UserCheck size={15} />
          <span>Lecturer Sign In</span>
        </button>
      </div>
    </div>

    <!-- Right Visual & Telemetry Card -->
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
  </section>
</div>

<style>
  .not-found-container {
    padding: calc(var(--nav-height) + 40px) var(--card-padding) var(--spacing-68);
    display: flex;
    justify-content: center;
    align-items: center;
    min-height: calc(100vh - var(--nav-height));
    box-sizing: border-box;
  }

  .not-found-grid {
    display: grid;
    grid-template-columns: 1.1fr 0.9fr;
    gap: var(--spacing-24);
    max-width: 1060px;
    width: 100%;
    align-items: stretch;
  }

  /* Left Card */
  .not-found-card {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-20);
    padding: var(--spacing-28);
    background: rgba(16, 9, 4, 0.65);
    border: 1px solid var(--color-cork-border);
    border-radius: 12px;
    height: 100%;
    box-sizing: border-box;
  }

  .card-header {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .status-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--color-driftwood);
    font-weight: 600;
  }

  .card-title {
    font-family: var(--font-display);
    font-size: clamp(26px, 3.2vw, 36px);
    font-weight: 500;
    color: var(--color-warm-cream);
    margin: 4px 0 0 0;
    line-height: 1.2;
    letter-spacing: -0.01em;
  }

  .card-desc {
    font-size: 13px;
    color: var(--color-warm-cream-dim);
    margin: 0;
    line-height: 1.5;
  }

  /* Retry Code Input */
  .retry-code-box {
    background: rgba(10, 5, 2, 0.5);
    border: 1px solid var(--color-cork-border);
    border-radius: 8px;
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .retry-label {
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-driftwood);
    font-weight: 600;
  }

  .retry-input-row {
    display: flex;
    gap: 8px;
  }

  .search-input-wrap {
    position: relative;
    flex: 1;
    display: flex;
    align-items: center;
  }

  .search-input-wrap input {
    width: 100%;
    padding: 8px 12px 8px 34px;
    font-size: 13px;
    background: rgba(16, 9, 4, 0.8);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    color: var(--color-warm-cream);
    font-family: monospace;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    margin: 0;
  }

  .search-input-wrap input:focus {
    border-color: var(--color-ember-accent);
    outline: none;
  }

  :global(.input-icon) {
    position: absolute;
    left: 10px;
    color: var(--color-driftwood);
    pointer-events: none;
  }

  .retry-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--color-ember-accent);
    color: #fff;
    border: 0;
    padding: 0 16px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.15s ease;
  }

  .retry-btn:hover:not(:disabled) {
    background: #e04a00;
  }

  .retry-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  /* Diagnostic Checklist */
  .diagnostic-checklist {
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: rgba(10, 5, 2, 0.3);
    border: 1px dashed var(--color-cork-border);
    padding: 12px 14px;
    border-radius: 8px;
  }

  .checklist-title {
    font-size: 10px;
    letter-spacing: 0.1em;
    color: var(--color-driftwood);
    font-weight: 600;
    margin: 0;
  }

  .checklist-items {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .checklist-items li {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--color-warm-cream-dim);
  }

  /* Action Buttons */
  .action-buttons-group {
    margin-top: auto;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }

  .nf-action-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 10px 12px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    cursor: pointer;
    transition: all 0.15s ease;
    border: 1px solid transparent;
  }

  .nf-action-btn.primary {
    background: var(--color-ember-accent);
    color: #ffffff;
    border-color: var(--color-ember-accent);
  }

  .nf-action-btn.primary:hover {
    background: #e04a00;
  }

  .nf-action-btn.outline {
    background: rgba(56, 36, 22, 0.5);
    color: var(--color-warm-cream);
    border-color: var(--color-cork-border);
  }

  .nf-action-btn.outline:hover {
    border-color: var(--color-warm-cream);
    background: var(--color-bark-brown);
  }

  .nf-action-btn.secondary {
    background: rgba(56, 36, 22, 0.8);
    color: var(--color-warm-cream);
    border-color: rgba(220, 80, 0, 0.4);
  }

  .nf-action-btn.secondary:hover {
    background: rgba(220, 80, 0, 0.2);
    border-color: var(--color-ember-accent);
  }

  /* Right Visual Card */
  .not-found-visual-card {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: var(--spacing-28);
    background: rgba(10, 5, 2, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 12px;
    height: 100%;
    box-sizing: border-box;
    gap: var(--spacing-20);
  }

  .visual-header {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 12px;
  }

  .telemetry-tag {
    font-size: 10px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--color-driftwood);
    font-weight: 600;
    background: rgba(255, 255, 255, 0.05);
    padding: 3px 10px;
    border-radius: 9999px;
    border: 1px solid var(--color-cork-border);
  }

  .big-404-wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-top: 8px;
  }

  .big-404-text {
    font-family: var(--font-display);
    font-size: clamp(80px, 12vw, 130px);
    line-height: 0.85;
    font-weight: 500;
    color: transparent;
    -webkit-text-stroke: 1.5px var(--color-ember-accent);
    letter-spacing: -0.04em;
    margin: 0;
    filter: drop-shadow(0 0 20px rgba(220, 80, 0, 0.25));
  }

  .big-404-sub {
    font-size: 11px;
    letter-spacing: 0.16em;
    color: var(--color-ember-accent);
    margin: 8px 0 0 0;
    font-weight: 600;
  }

  .telemetry-grid {
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: rgba(16, 9, 4, 0.4);
    border: 1px solid var(--color-cork-border);
    padding: 12px 14px;
    border-radius: 8px;
  }

  .telemetry-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 11px;
  }

  .tel-label {
    color: var(--color-driftwood);
    letter-spacing: 0.08em;
  }

  .tel-val {
    color: var(--color-warm-cream);
    font-family: monospace;
  }

  .tel-val.status-404 {
    color: var(--color-ember-accent);
    font-weight: 600;
  }

  .visual-footer-box {
    background: rgba(16, 9, 4, 0.5);
    border: 1px dashed var(--color-cork-border);
    padding: 12px 14px;
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .footer-box-header {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--color-warm-cream);
  }

  .visual-footer-box .hint {
    font-size: 11px;
    color: var(--color-driftwood);
    margin: 0;
    line-height: 1.4;
  }

  @media (max-width: 900px) {
    .not-found-grid {
      grid-template-columns: 1fr;
    }
    .not-found-visual-card {
      display: none !important;
    }
    .action-buttons-group {
      grid-template-columns: 1fr;
    }
  }
</style>
