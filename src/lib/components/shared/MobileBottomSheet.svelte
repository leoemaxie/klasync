<script lang="ts">
  import type { Snippet } from 'svelte';
  import { X } from '@lucide/svelte';
  import { triggerHaptic } from '$lib/native/haptics';

  let {
    isOpen = $bindable(false),
    title = '',
    children,
  }: {
    isOpen: boolean;
    title?: string;
    children?: Snippet;
  } = $props();

  function close() {
    triggerHaptic('light');
    isOpen = false;
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && isOpen && close()} />

{#if isOpen}
  <div class="sheet-backdrop" onclick={close} role="presentation">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="sheet-panel" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-label={title}>
      <div class="sheet-drag-handle"></div>

      <div class="sheet-header">
        <h3 class="sheet-title">{title}</h3>
        <button type="button" class="sheet-close-btn text" onclick={close} aria-label="Close">
          <X size={16} />
        </button>
      </div>

      <div class="sheet-content">
        {@render children?.()}
      </div>
    </div>
  </div>
{/if}

<style>
  .sheet-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(8px);
    z-index: 120;
    display: flex;
    align-items: flex-end;
    animation: fadeIn 0.15s ease-out;
  }
  .sheet-panel {
    width: 100%;
    max-height: 80vh;
    background: #1c1008;
    border-top: 1px solid var(--color-cork-border);
    border-radius: 16px 16px 0 0;
    padding: 12px 16px calc(24px + env(safe-area-inset-bottom, 0px));
    display: flex;
    flex-direction: column;
    gap: 12px;
    animation: slideUp 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }
  .sheet-drag-handle {
    width: 36px;
    height: 4px;
    border-radius: 2px;
    background: var(--color-cork-border);
    margin: 0 auto 4px auto;
  }
  .sheet-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--color-cork-border);
    padding-bottom: 8px;
  }
  .sheet-title { font-family: var(--font-display); font-size: 17px; color: var(--color-warm-cream); margin: 0; }
  .sheet-close-btn { padding: 4px; color: var(--color-driftwood); }
  .sheet-content { overflow-y: auto; max-height: 60vh; }
  @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
  @keyframes slideUp { from { transform: translateY(100%); } to { transform: translateY(0); } }
</style>
