<script lang="ts">
  import type { Screen } from '$lib/types';
  import { Home, Radio, UserCheck } from 'lucide-svelte';

  let { screen = $bindable() }: { screen: Screen } = $props();
</script>

<section class="join-wrap not-found-wrap">
  <div class="not-found-left-column">
    <div class="not-found-badge">
      <span class="eyebrow-accent">●</span> 404 / ROOM UNRESOLVED
    </div>

    <h1 class="not-found-heading">Room not found.</h1>

    <p class="not-found-desc">
      Double-check the lecture session code or select a navigation path below to continue.
    </p>

    <div class="actions not-found-actions">
      <a href="#/" class="nf-btn primary" onclick={() => (screen = 'home')}>
        <Home size={14} aria-hidden="true" />
        <span>Return Home</span>
      </a>
      <a href="#/join" class="nf-btn outline" onclick={() => (screen = 'join')}>
        <Radio size={14} aria-hidden="true" />
        <span>Join Live Session</span>
      </a>
      <a
        href="#/lecturer-login"
        class="nf-btn secondary"
        onclick={() => (screen = 'lecturer-login')}
      >
        <UserCheck size={14} aria-hidden="true" />
        <span>Lecturer Access</span>
      </a>
    </div>
  </div>

  <div class="not-found-right-column">
    <h2 class="code-404-title">404</h2>
    <p class="lede">The session code or link is invalid or expired.</p>
  </div>
</section>

<style>
  .not-found-wrap {
    display: grid;
    grid-template-columns: 1.15fr 0.85fr;
    align-items: center;
    gap: var(--spacing-36);
    min-height: 75vh;
  }

  .not-found-left-column {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    text-align: left;
    gap: var(--spacing-16);
    padding: var(--spacing-36);
    background: rgba(16, 9, 4, 0.5);
    border: 1px solid var(--color-cork-border);
    border-radius: 12px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.35);
  }

  .not-found-badge {
    font-family: var(--font-body);
    font-size: var(--text-label);
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--color-driftwood);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin: 0;
  }

  .eyebrow-accent {
    color: var(--color-ember-accent);
  }

  .not-found-heading {
    font-size: clamp(32px, 4vw, 48px);
    font-weight: 500;
    color: var(--color-warm-cream);
    font-family: var(--font-display);
    margin: 0;
    line-height: 1.15;
    letter-spacing: -0.01em;
  }

  .not-found-desc {
    font-size: 14px;
    color: var(--color-warm-cream-dim);
    margin: 0;
    line-height: 1.5;
  }

  .not-found-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 6px;
    flex-wrap: wrap;
  }

  .nf-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 10px 18px;
    border-radius: 24px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    text-decoration: none !important;
    transition: all 0.2s ease;
  }

  .nf-btn.primary {
    background: var(--color-ember-accent);
    color: #fff;
    border: 1px solid var(--color-ember-accent);
  }

  .nf-btn.primary:hover {
    background: #e04a00;
    border-color: #e04a00;
    transform: translateY(-1px);
    box-shadow: 0 4px 14px rgba(220, 80, 0, 0.35);
  }

  .nf-btn.outline {
    background: rgba(56, 36, 22, 0.4);
    color: var(--color-warm-cream);
    border: 1px solid var(--color-cork-border);
  }

  .nf-btn.outline:hover {
    border-color: var(--color-warm-cream);
    background: var(--color-bark-brown);
    transform: translateY(-1px);
  }

  .nf-btn.secondary {
    background: rgba(56, 36, 22, 0.7);
    color: var(--color-warm-cream);
    border: 1px solid rgba(220, 80, 0, 0.35);
  }

  .nf-btn.secondary:hover {
    background: rgba(220, 80, 0, 0.2);
    border-color: var(--color-ember-accent);
    color: var(--color-warm-cream);
    transform: translateY(-1px);
  }

  .not-found-right-column {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--spacing-14);
  }

  .code-404-title {
    font-family: var(--font-display);
    font-size: clamp(96px, 16vw, 180px);
    line-height: 0.85;
    font-weight: 500;
    color: transparent;
    -webkit-text-stroke: 1.5px var(--color-ember-accent);
    letter-spacing: -0.04em;
    margin: 0;
  }

  .lede {
    font-size: 15px;
    color: var(--color-driftwood);
    margin: 0;
  }

  @media (max-width: 900px) {
    .not-found-wrap {
      grid-template-columns: 1fr;
      gap: var(--spacing-28);
    }
    .not-found-left-column {
      padding: var(--spacing-24);
    }
    .not-found-actions {
      flex-direction: column;
      width: 100%;
    }
    .nf-btn {
      width: 100%;
    }
  }
</style>
