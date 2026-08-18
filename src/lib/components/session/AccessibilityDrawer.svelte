<script lang="ts">
  import { Settings, X } from '@lucide/svelte';
  import { triggerHaptic } from '$lib/native/haptics';

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

  function toggleDrawer() {
    triggerHaptic('light');
    drawerOpen = !drawerOpen;
  }

  function setFontSize(val: string) {
    triggerHaptic('light');
    fontSize = val;
  }

  function toggleDyslexiaFont() {
    triggerHaptic('medium');
    dyslexicFont = !dyslexicFont;
  }
</script>

<div class="accessibility-drawer-wrap">
  <button
    type="button"
    class="outline drawer-toggle-btn"
    onclick={toggleDrawer}
    aria-expanded={drawerOpen}
  >
    <Settings
      size={14}
      aria-hidden="true"
      style="vertical-align: middle; display: inline-block;"
    /> Accessibility
  </button>

  {#if drawerOpen}
    <div
      class="panel drawer-content"
      role="region"
      aria-label="Accessibility Settings"
    >
      <div class="drawer-header">
        <p class="eyebrow">DISPLAY &amp; ACCESSIBILITY</p>
        <button type="button" class="text" onclick={toggleDrawer}>
          <X
            size={14}
            aria-hidden="true"
            style="vertical-align: middle; display: inline-block;"
          /> Close
        </button>
      </div>

      <div class="control-group">
        <p id="font-size-label" class="label" style="margin: 0 0 6px;">
          Text size
        </p>
        <div
          class="button-group"
          role="group"
          aria-labelledby="font-size-label"
        >
          <button
            type="button"
            class={fontSize === '16px' ? 'primary' : 'outline'}
            aria-pressed={fontSize === '16px'}
            onclick={() => setFontSize('16px')}
          >
            16px
          </button>
          <button
            type="button"
            class={fontSize === '20px' ? 'primary' : 'outline'}
            aria-pressed={fontSize === '20px'}
            onclick={() => setFontSize('20px')}
          >
            20px
          </button>
          <button
            type="button"
            class={fontSize === '24px' ? 'primary' : 'outline'}
            aria-pressed={fontSize === '24px'}
            onclick={() => setFontSize('24px')}
          >
            24px
          </button>
        </div>
      </div>

      <div class="control-group">
        <label for="line-height-range">Line spacing ({lineHeight}x)</label>
        <input
          id="line-height-range"
          type="range"
          min="1.3"
          max="2.2"
          step="0.1"
          aria-valuetext="{lineHeight}x"
          bind:value={lineHeight}
        />
      </div>

      <div class="control-group toggle-row">
        <div>
          <p class="toggle-title">Dyslexia-friendly font</p>
          <p class="hint">
            Improves readability with specialized letter shapes.
          </p>
        </div>
        <button
          type="button"
          class={dyslexicFont ? 'primary' : 'outline'}
          aria-pressed={dyslexicFont}
          onclick={toggleDyslexiaFont}
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
    flex-wrap: wrap;
    gap: var(--spacing-8);
  }
  .button-group button {
    flex: 1 1 90px;
    min-width: 80px;
    font-size: 10px;
    padding: 6px 8px;
    white-space: nowrap;
    text-align: center;
    box-sizing: border-box;
  }
  .toggle-row {
    display: flex;
    flex-direction: row;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }
  .toggle-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--color-warm-cream);
  }
</style>
