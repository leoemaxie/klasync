<script lang="ts">
  import type { Screen } from '$lib/types';
  import { Archive, Radio, Home } from '@lucide/svelte';

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
    <h1 class="workspace-title">Lecture Archive</h1>
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
      title="Join a live session"
    >
      <Radio size={13} color="var(--color-ember-accent)" />
      <span>Join Session</span>
    </button>
  </div>
</header>

<style>
  .workspace-top-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--spacing-10);
    padding-bottom: var(--spacing-10);
    border-bottom: 1px solid var(--color-cork-border);
  }
  .workspace-title {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--color-warm-cream);
    margin: 0;
  }
  .top-bar-right {
    display: flex;
    align-items: center;
    gap: var(--spacing-8);
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
    padding: 6px 10px;
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
  @media (max-width: 640px) {
    .workspace-title {
      font-size: 19px;
    }
    .top-bar-right {
      width: 100%;
      justify-content: space-between;
    }
  }
</style>
