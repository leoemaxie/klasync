<script lang="ts">
  import type { Screen } from '$lib/types';
  import { Archive, Radio } from '@lucide/svelte';

  let {
    claimsCount = 0,
    onNavigate,
  }: {
    claimsCount: number;
    onNavigate: (screen: Screen) => void;
  } = $props();
</script>

<header class="workspace-top-bar">
  <div class="top-bar-left">
    <div class="eyebrow-brand">
      <span class="brand-pill">KLASYNC</span>
      <span class="divider">/</span>
      <span class="workspace-label">STUDENT ARCHIVE STUDIO</span>
    </div>
    <h1 class="workspace-title">Lecture Archive & Study Studio</h1>
  </div>

  <div class="top-bar-right">
    <div class="stat-badge">
      <Archive size={13} color="var(--color-ember-accent)" />
      <span>{claimsCount} {claimsCount === 1 ? 'Lecture' : 'Lectures'}</span>
    </div>

    <button
      type="button"
      class="outline join-cta-btn"
      onclick={() => onNavigate('join')}
      title="Join a live lecture session"
    >
      <Radio size={13} color="var(--color-ember-accent)" />
      <span>Join Live Session</span>
    </button>

    <button
      type="button"
      class="text home-link-btn"
      onclick={() => onNavigate('home')}
    >
      Return Home
    </button>
  </div>
</header>

<style>
  .workspace-top-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--spacing-14);
    padding-bottom: var(--spacing-14);
    border-bottom: 1px solid var(--color-cork-border);
  }
  .eyebrow-brand {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    margin-bottom: 4px;
  }
  .brand-pill {
    color: var(--color-ember-accent);
    font-weight: 700;
    font-family: var(--font-mono, monospace);
  }
  .divider {
    color: var(--color-cork-border);
  }
  .workspace-label {
    color: var(--color-driftwood);
  }
  .workspace-title {
    font-family: var(--font-display);
    font-size: 26px;
    color: var(--color-warm-cream);
    margin: 0;
  }
  .top-bar-right {
    display: flex;
    align-items: center;
    gap: var(--spacing-12);
    flex-wrap: wrap;
  }
  .stat-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--color-warm-cream);
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    padding: 6px 12px;
    border-radius: 4px;
  }
  .join-cta-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    padding: 6px 12px;
    text-transform: uppercase;
  }
  .home-link-btn {
    font-size: 11px;
    text-transform: uppercase;
    color: var(--color-driftwood);
  }
  @media (max-width: 640px) {
    .workspace-title {
      font-size: 22px;
    }
    .top-bar-right {
      width: 100%;
      justify-content: space-between;
    }
  }
</style>
