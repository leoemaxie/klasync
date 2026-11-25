<script lang="ts">
  let {
    value = $bindable(),
    min,
    max,
    label = 'Resize panel',
  }: {
    /** Width in px of the panel on the right of the handle. */
    value: number;
    min: number;
    max: number;
    label?: string;
  } = $props();

  let dragging = $state(false);
  let startX = 0;
  let startValue = 0;

  const clamp = (v: number) => Math.min(Math.max(v, min), max);

  function down(event: PointerEvent) {
    dragging = true;
    startX = event.clientX;
    startValue = value;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }
  function move(event: PointerEvent) {
    if (!dragging) return;
    value = clamp(startValue - (event.clientX - startX));
  }
  function up() {
    dragging = false;
  }
  function key(event: KeyboardEvent) {
    const step = event.shiftKey ? 64 : 16;
    if (event.key === 'ArrowLeft') value = clamp(value + step);
    else if (event.key === 'ArrowRight') value = clamp(value - step);
    else if (event.key === 'Home') value = max;
    else if (event.key === 'End') value = min;
    else return;
    event.preventDefault();
  }
</script>

<!-- A focusable separator is an interactive widget per WAI-ARIA. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="splitter"
  class:dragging
  role="separator"
  aria-orientation="vertical"
  aria-label={label}
  aria-valuemin={min}
  aria-valuemax={max}
  aria-valuenow={Math.round(value)}
  tabindex="0"
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
  onkeydown={key}
  ondblclick={() => (value = clamp(400))}
>
  <span class="grip" aria-hidden="true"></span>
</div>

<style>
  .splitter {
    position: relative;
    width: 14px;
    margin: 0 -2px;
    cursor: col-resize;
    touch-action: none;
    display: grid;
    place-items: center;
    outline: none;
  }
  .grip {
    width: 3px;
    height: 36px;
    border-radius: 3px;
    background: var(--border-default);
    transition:
      background var(--motion-fast) var(--ease-out),
      height var(--motion-base) var(--ease-out);
  }
  .splitter:hover .grip,
  .splitter.dragging .grip {
    background: var(--color-ember-accent);
    height: 56px;
  }
  .splitter:focus-visible .grip {
    background: var(--text-primary);
    height: 56px;
    box-shadow: 0 0 0 3px var(--surface-page);
  }
</style>
