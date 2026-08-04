<script lang="ts">
  let { currentUser, liveCode }: { currentUser?: unknown; liveCode?: string } =
    $props();
</script>

<header class="workspace-header">
  <div class="header-badge-row">
    <span class="eyebrow">LECTURER CONTROL ROOM</span>
    {#if liveCode}
      <span class="live-status-pill"
        ><span class="pulse-dot" aria-hidden="true">●</span> SESSION LIVE ({liveCode})</span
      >
    {/if}
  </div>
</header>

<style>
  .workspace-header {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14);
    margin-bottom: var(--spacing-20);
    border-bottom: 1px solid var(--color-cork-border);
    padding-bottom: var(--spacing-20);
  }
  .header-badge-row {
    display: flex;
    align-items: center;
    gap: var(--spacing-12);
    flex-wrap: wrap;
  }
  .live-status-pill {
    font-size: 11px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.12);
    border: 1px solid var(--color-ember-accent);
    padding: 3px 8px;
    border-radius: 4px;
    font-weight: 700;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .pulse-dot {
    animation: blink 1s infinite alternate;
  }
  @keyframes blink {
    0% {
      opacity: 0.3;
    }
    100% {
      opacity: 1;
    }
  }
</style>
