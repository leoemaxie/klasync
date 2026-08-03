<script lang="ts">
  import { onMount } from 'svelte';
  import { triggerHaptic } from '$lib/native/haptics';

  let isOnline = $state(true);
  let showReconnected = $state(false);
  let reconnectedTimer: number | undefined;

  onMount(() => {
    isOnline = typeof navigator !== 'undefined' ? navigator.onLine : true;

    function handleOnline() {
      if (!isOnline) {
        triggerHaptic('success');
        showReconnected = true;
        clearTimeout(reconnectedTimer);
        reconnectedTimer = window.setTimeout(() => {
          showReconnected = false;
        }, 3000);
      }
      isOnline = true;
    }

    function handleOffline() {
      triggerHaptic('warning');
      isOnline = false;
      showReconnected = false;
    }

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
      clearTimeout(reconnectedTimer);
    };
  });
</script>

{#if !isOnline}
  <div class="offline-banner offline" role="status" aria-live="polite">
    <span class="offline-dot">●</span>
    <span>OFFLINE MODE ACTIVE · Queueing heartbeats &amp; local cache</span>
  </div>
{:else if showReconnected}
  <div class="offline-banner online" role="status" aria-live="polite">
    <span class="online-dot">●</span>
    <span>RECONNECTED · Syncing live data</span>
  </div>
{/if}

<style>
  .offline-banner {
    position: fixed;
    bottom: calc(16px + var(--safe-bottom, 0px));
    right: 16px;
    z-index: 150;
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(16, 9, 4, 0.95);
    backdrop-filter: blur(12px);
    border: 1px solid var(--color-cork-border);
    color: var(--color-warm-cream);
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.08em;
    padding: 10px 18px;
    border-radius: 999px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.6);
  }
  .offline-banner.offline {
    border-color: var(--color-ember-accent);
  }
  .offline-banner.online {
    border-color: #28a745;
  }
  .offline-dot {
    color: var(--color-ember-accent);
    animation: blink 1s infinite alternate;
  }
  .online-dot {
    color: #28a745;
  }
  @keyframes blink {
    from {
      opacity: 0.3;
    }
    to {
      opacity: 1;
    }
  }
</style>
