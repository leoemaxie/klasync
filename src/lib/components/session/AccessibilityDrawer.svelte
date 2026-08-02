<script lang="ts">
  import { Settings, X } from '@lucide/svelte';

  let {
    fontSize = $bindable('18px'),
    dyslexicFont = $bindable(false),
    lineHeight = $bindable(1.6),
  }: {
    fontSize: string;
    dyslexicFont: boolean;
    lineHeight: number;
  } = $props();

  let drawerOpen = $state(false);
</script>

<div class="accessibility-drawer-wrap">
  <button
    type="button"
    class="outline drawer-toggle-btn"
    onclick={() => (drawerOpen = !drawerOpen)}
    aria-expanded={drawerOpen}
  >
    <Settings
      size={14}
      style="vertical-align: middle; display: inline-block;"
    /> Accessibility Preferences
  </button>

  {#if drawerOpen}
    <div
      class="panel drawer-content"
      role="region"
      aria-label="Accessibility Settings"
    >
      <div class="drawer-header">
        <p class="eyebrow">ACCESSIBILITY &amp; READING DISPLAY</p>
        <button type="button" class="text" onclick={() => (drawerOpen = false)}>
          <X size={14} style="vertical-align: middle; display: inline-block;" /> Close
        </button>
      </div>

      <div class="control-group">
        <label for="font-size-select">Caption &amp; Body Font Scale</label>
        <div class="button-group" id="font-size-select">
          <button
            type="button"
            class={fontSize === '16px' ? 'primary' : 'outline'}
            onclick={() => (fontSize = '16px')}
          >
            Standard (16px)
          </button>
          <button
            type="button"
            class={fontSize === '20px' ? 'primary' : 'outline'}
            onclick={() => (fontSize = '20px')}
          >
            Large (20px)
          </button>
          <button
            type="button"
            class={fontSize === '24px' ? 'primary' : 'outline'}
            onclick={() => (fontSize = '24px')}
          >
            XL (24px)
          </button>
        </div>
      </div>

      <div class="control-group">
        <label for="line-height-range">Line Spacing ({lineHeight}x)</label>
        <input
          id="line-height-range"
          type="range"
          min="1.3"
          max="2.2"
          step="0.1"
          bind:value={lineHeight}
        />
      </div>

      <div class="control-group toggle-row">
        <div>
          <p class="toggle-title">High-Contrast Dyslexia Font</p>
          <p class="hint">
            Increases character spacing and bottom-heavy letter shapes.
          </p>
        </div>
        <button
          type="button"
          class={dyslexicFont ? 'primary' : 'outline'}
          onclick={() => (dyslexicFont = !dyslexicFont)}
        >
          {dyslexicFont ? 'ON' : 'OFF'}
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  .accessibility-drawer-wrap {
    margin: var(--spacing-14) 0;
  }
  .drawer-toggle-btn {
    font-size: 11px;
    padding: 6px 14px;
  }
  .drawer-content {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14);
    margin-top: var(--spacing-12);
  }
  .drawer-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .control-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .button-group {
    display: flex;
    gap: var(--spacing-8);
  }
  .button-group button {
    flex: 1;
    font-size: 10px;
    padding: 6px;
  }
  .toggle-row {
    display: flex;
    flex-direction: row;
    justify-content: space-between;
    align-items: center;
  }
  .toggle-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--color-warm-cream);
  }
</style>
