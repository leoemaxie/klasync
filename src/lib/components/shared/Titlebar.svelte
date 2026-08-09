<script lang="ts">
  import { onMount } from 'svelte';
  import { platform } from '$lib/native/platform';
  import {
    minimizeWindow,
    toggleMaximizeWindow,
    closeWindow,
  } from '$lib/native/window';

  let isTauri = $state(false);
  let isMac = $state(false);

  onMount(() => {
    isTauri = platform.isTauri;
    isMac = platform.isMacOS;
  });
</script>

{#if isTauri && !isMac}
  <div class="tauri-titlebar app-drag-region">
    <div class="titlebar-title">KLASYNC</div>
    <div class="titlebar-controls app-no-drag">
      <button
        class="titlebar-btn"
        onclick={minimizeWindow}
        aria-label="Minimize"
      >
        <svg width="10" height="1" viewBox="0 0 10 1"
          ><rect width="10" height="1" fill="currentColor" /></svg
        >
      </button>
      <button
        class="titlebar-btn"
        onclick={toggleMaximizeWindow}
        aria-label="Maximize"
      >
        <svg width="10" height="10" viewBox="0 0 10 10"
          ><rect
            width="9"
            height="9"
            x="0.5"
            y="0.5"
            fill="none"
            stroke="currentColor"
          /></svg
        >
      </button>
      <button
        class="titlebar-btn close"
        onclick={closeWindow}
        aria-label="Close"
      >
        <svg width="10" height="10" viewBox="0 0 10 10"
          ><path
            d="M1,1 L9,9 M9,1 L1,9"
            stroke="currentColor"
            stroke-width="1.2"
          /></svg
        >
      </button>
    </div>
  </div>
{/if}

<style>
  .tauri-titlebar {
    height: 32px;
    background: #0d0703;
    border-bottom: 1px solid var(--color-cork-border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-left: 14px;
    user-select: none;
    z-index: 1000;
    position: relative;
  }
  .titlebar-title {
    font-size: 11px;
    letter-spacing: 0.15em;
    color: var(--color-driftwood);
    font-weight: 500;
  }
  .titlebar-controls {
    display: flex;
    height: 100%;
  }
  .titlebar-btn {
    width: 44px;
    height: 100%;
    background: transparent;
    border: 0;
    color: var(--color-warm-cream);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s ease;
  }
  .titlebar-btn:hover {
    background: rgba(255, 237, 215, 0.1);
  }
  .titlebar-btn.close:hover {
    background: #dc3545;
    color: #fff;
  }
</style>
