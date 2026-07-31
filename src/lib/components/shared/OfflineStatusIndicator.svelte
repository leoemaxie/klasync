<script lang="ts">
  import { onMount } from "svelte";

  let isOnline = $state(true);

  onMount(() => {
    isOnline = typeof navigator !== "undefined" ? navigator.onLine : true;

    function handleOnline() {
      isOnline = true;
    }
    function handleOffline() {
      isOnline = false;
    }

    window.addEventListener("online", handleOnline);
    window.addEventListener("offline", handleOffline);

    return () => {
      window.removeEventListener("online", handleOnline);
      window.removeEventListener("offline", handleOffline);
    };
  });
</script>

{#if !isOnline}
  <div class="offline-banner" role="status" aria-live="polite">
    <span class="offline-dot">●</span>
    <span>OFFLINE MODE ACTIVE · Local buffer queueing heartbeats &amp; captions</span>
  </div>
{/if}

<style>
  .offline-banner {
    position: fixed;
    bottom: 16px;
    right: 16px;
    z-index: 150;
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(56, 36, 22, 0.95);
    border: 1px solid var(--color-ember-accent);
    color: var(--color-warm-cream);
    font-size: 11px;
    letter-spacing: 0.08em;
    padding: 8px 16px;
    border-radius: 999px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.6);
  }
  .offline-dot {
    color: var(--color-ember-accent);
    animation: blink 1s infinite alternate;
  }
  @keyframes blink {
    from { opacity: 0.3; }
    to { opacity: 1; }
  }
</style>
