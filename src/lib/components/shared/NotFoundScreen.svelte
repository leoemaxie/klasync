<script lang="ts">
  import type { Screen } from '$lib/types';
  import {
    Home,
    Radio,
    UserCheck,
    Search,
    Compass,
    AlertCircle,
    ArrowRight,
  } from '@lucide/svelte';
  import NotFoundVisualCard from './not_found/NotFoundVisualCard.svelte';

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
    <div class="panel not-found-card">
      <div class="card-header">
        <div class="status-badge">
          <Compass size={15} color="var(--color-ember-accent)" />
          <span>404 · PAGE NOT FOUND</span>
        </div>
        <h1 class="card-title">Page Not Found</h1>
        <p class="card-desc">
          The page, session code, or link you requested does not exist or has expired.
        </p>
      </div>

      <form class="retry-code-box" onsubmit={handleRetryJoin}>
        <label for="nf-retry-input" class="retry-label"
          >Have a session code?</label
        >
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
        <p class="checklist-title">POSSIBLE REASONS:</p>
        <ul class="checklist-items">
          <li>
            <AlertCircle size={13} color="var(--color-driftwood)" />
            <span>The lecture session has ended.</span>
          </li>
          <li>
            <AlertCircle size={13} color="var(--color-driftwood)" />
            <span>Check for typos in the session code.</span>
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
          <span>Home</span>
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

    <NotFoundVisualCard />
  </section>
</div>

<style>
  .not-found-container {
    padding: calc(var(--nav-height) + 24px) var(--card-padding)
      var(--spacing-68);
    max-width: 1100px;
    margin: 0 auto;
  }
  .not-found-grid {
    display: grid;
    grid-template-columns: 1.2fr 0.8fr;
    gap: var(--spacing-28);
    align-items: stretch;
  }
  .not-found-card {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-20);
    padding: var(--spacing-28);
    background: rgba(16, 9, 4, 0.45);
    border: 1px solid var(--color-cork-border);
    border-radius: 12px;
  }
  .card-header {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-10);
  }
  .status-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    letter-spacing: 0.1em;
    color: var(--color-warm-cream);
    background: rgba(220, 80, 0, 0.12);
    border: 1px solid rgba(220, 80, 0, 0.25);
    padding: 4px 10px;
    border-radius: 999px;
    align-self: flex-start;
  }
  .card-title {
    font-family: var(--font-display);
    font-size: clamp(26px, 4vw, 36px);
    color: var(--color-warm-cream);
    margin: 0;
    line-height: 1.15;
  }
  .card-desc {
    color: var(--color-driftwood);
    font-size: 14px;
    line-height: 1.5;
    margin: 0;
  }
  .retry-code-box {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-8);
  }
  .retry-label {
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-driftwood);
  }
  .retry-input-row {
    display: flex;
    gap: var(--spacing-10);
  }
  .search-input-wrap {
    position: relative;
    flex: 1;
    display: flex;
    align-items: center;
  }
  :global(.search-input-wrap .input-icon) {
    position: absolute;
    left: 12px;
    color: var(--color-driftwood);
  }
  .search-input-wrap input {
    width: 100%;
    padding: 10px 12px 10px 38px;
    background: rgba(8, 4, 2, 0.6);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    color: var(--color-warm-cream);
    font-size: 14px;
    font-family: var(--font-mono, monospace);
  }
  .search-input-wrap input:focus {
    outline: none;
    border-color: var(--color-warm-cream);
  }
  .retry-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 10px 20px;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .diagnostic-checklist {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-8);
    border-top: 1px solid var(--color-cork-border);
    border-bottom: 1px solid var(--color-cork-border);
    padding: var(--spacing-14) 0;
  }
  .checklist-title {
    font-size: 11px;
    letter-spacing: 0.1em;
    color: var(--color-driftwood);
    margin: 0;
  }
  .checklist-items {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-8);
  }
  .checklist-items li {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--color-warm-cream);
  }
  .action-buttons-group {
    display: flex;
    gap: var(--spacing-10);
    flex-wrap: wrap;
  }
  .nf-action-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 10px 18px;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  @media (max-width: 900px) {
    .not-found-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
