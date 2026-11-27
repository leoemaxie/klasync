<script lang="ts">
  import { tick } from 'svelte';
  import { Ellipsis } from '@lucide/svelte';
  import { triggerHaptic } from '$lib/native/haptics';
  import { TOOLS, toolById, tabId, type ToolId } from './tools';

  let {
    pane,
    active,
    tools = TOOLS.map((t) => t.id),
    primary,
    labels = false,
    onSelect,
    label,
  }: {
    /** Which pane this switcher drives; namespaces tab/panel ids. */
    pane: string;
    active: ToolId;
    tools?: ToolId[];
    /** When set, only these show as tabs; the rest live behind "More". */
    primary?: ToolId[];
    /** Keep every tab's label even when the pane is narrow. */
    labels?: boolean;
    onSelect: (id: ToolId) => void;
    label: string;
  } = $props();

  const visible = $derived(primary ?? tools);
  const overflow = $derived(tools.filter((t) => !visible.includes(t)));
  const overflowActive = $derived(overflow.includes(active));
  let moreOpen = $state(false);
  let listEl: HTMLDivElement | undefined = $state();
  let moreEl: HTMLDivElement | undefined = $state();

  function select(id: ToolId) {
    triggerHaptic('selection');
    moreOpen = false;
    onSelect(id);
  }

  async function onKeydown(event: KeyboardEvent) {
    const keys = ['ArrowRight', 'ArrowLeft', 'Home', 'End'];
    if (!keys.includes(event.key)) return;
    const order = visible;
    const current = order.indexOf(active);
    let next = current < 0 ? 0 : current;
    if (event.key === 'ArrowRight') next = (current + 1) % order.length;
    if (event.key === 'ArrowLeft')
      next = (current - 1 + order.length) % order.length;
    if (event.key === 'Home') next = 0;
    if (event.key === 'End') next = order.length - 1;
    event.preventDefault();
    select(order[next]);
    await tick();
    listEl?.querySelector<HTMLElement>(`#${tabId(pane, order[next])}`)?.focus();
  }

  function onWindowPointer(event: PointerEvent) {
    if (moreOpen && moreEl && !moreEl.contains(event.target as Node))
      moreOpen = false;
  }
</script>

<svelte:window onpointerdown={onWindowPointer} />

<div class="tool-tabs" class:all-labels={labels}>
  <div
    class="tablist"
    role="tablist"
    aria-label={label}
    tabindex="-1"
    bind:this={listEl}
    onkeydown={onKeydown}
  >
    {#each visible as id (id)}
      {@const def = toolById(id)}
      {@const Icon = def.icon}
      <button
        type="button"
        role="tab"
        id={tabId(pane, id)}
        class="tab"
        class:active={active === id}
        aria-selected={active === id}
        aria-label={def.label}
        title="{def.label} ({def.key})"
        aria-controls="{pane}-panel"
        tabindex={active === id || (overflowActive && id === visible[0])
          ? 0
          : -1}
        onclick={() => select(id)}
      >
        <Icon size={15} />
        <span class="tab-label">{def.label}</span>
      </button>
    {/each}
  </div>

  {#if overflow.length}
    <div class="more" bind:this={moreEl}>
      <button
        type="button"
        class="tab more-btn"
        class:active={overflowActive}
        aria-haspopup="menu"
        aria-expanded={moreOpen}
        onclick={() => (moreOpen = !moreOpen)}
        onkeydown={(e) => e.key === 'Escape' && (moreOpen = false)}
      >
        <Ellipsis size={15} />
        <span class="tab-label">
          {overflowActive ? toolById(active).label : 'More'}
        </span>
      </button>
      {#if moreOpen}
        <div class="menu" role="menu" aria-label="More tools">
          {#each overflow as id (id)}
            {@const def = toolById(id)}
            {@const Icon = def.icon}
            <button
              type="button"
              role="menuitemradio"
              aria-checked={active === id}
              class="menu-item"
              class:active={active === id}
              onclick={() => select(id)}
              onkeydown={(e) => e.key === 'Escape' && (moreOpen = false)}
            >
              <Icon size={15} /><span>{def.label}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .tool-tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
    flex: 1;
    container-type: inline-size;
    container-name: tooltabs;
  }
  /* Narrow panes: only the active tab keeps its label. */
  @container tooltabs (max-width: 540px) {
    .all-labels .tab:not(.active) .tab-label {
      display: inline;
    }
    .tool-tabs:not(.all-labels) .tab:not(.active) .tab-label {
      display: none;
    }
    .tool-tabs:not(.all-labels) .tab:not(.active) {
      padding: 0 10px;
    }
  }
  .tablist {
    display: flex;
    gap: 2px;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .tablist::-webkit-scrollbar {
    display: none;
  }
  .tab {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    height: var(--control-h);
    min-height: var(--control-h);
    min-width: 0;
    padding: 0 12px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--font-body);
    font-size: 13px;
    font-weight: 500;
    letter-spacing: 0.01em;
    white-space: nowrap;
    flex-shrink: 0;
    transition:
      color var(--motion-fast) var(--ease-out),
      background var(--motion-fast) var(--ease-out);
  }
  .tab:hover {
    color: var(--text-primary);
    background: var(--surface-hover);
  }
  .tab.active {
    color: var(--text-primary);
    background: var(--surface-accent);
  }
  .tab.active::after {
    content: '';
    position: absolute;
    left: 10px;
    right: 10px;
    bottom: 0;
    height: 2px;
    border-radius: 2px 2px 0 0;
    background: var(--color-ember-accent);
  }
  .tab:focus-visible,
  .menu-item:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
  .tab.active :global(svg) {
    color: var(--text-accent);
  }
  .more {
    position: relative;
    flex-shrink: 0;
  }
  .menu {
    position: absolute;
    right: 0;
    top: calc(100% + 6px);
    z-index: 30;
    min-width: 168px;
    padding: 6px;
    background: var(--surface-3);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    box-shadow: var(--elev-3);
    backdrop-filter: blur(14px);
    animation: pop var(--motion-base) var(--ease-out);
  }
  .menu-item {
    display: flex;
    width: 100%;
    align-items: center;
    gap: 10px;
    height: var(--control-h);
    min-height: var(--control-h);
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--font-body);
    font-size: 13.5px;
    text-align: left;
  }
  .menu-item:hover,
  .menu-item.active {
    background: var(--surface-hover);
    color: var(--text-primary);
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
  }
  @media (max-width: 720px) {
    .tool-tabs {
      width: 100%;
    }
    .tablist {
      flex: 1;
    }
    .tab {
      flex: 1 1 auto;
      gap: 5px;
      padding: 0 4px;
      font-size: 12.5px;
    }
    /* Phones: plain text labels, no icons, so nothing truncates. */
    .all-labels .tablist .tab :global(svg) {
      display: none;
    }
  }
</style>
