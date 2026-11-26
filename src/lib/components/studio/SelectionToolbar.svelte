<script lang="ts">
  import {
    PenLine,
    MessageCircleQuestion,
    Layers,
    BookmarkPlus,
  } from '@lucide/svelte';

  export type SelectionAction = 'note' | 'ask' | 'cards' | 'bookmark';

  let {
    x,
    y,
    placement = 'above',
    onAction,
  }: {
    x: number;
    y: number;
    placement?: 'above' | 'below';
    onAction: (action: SelectionAction) => void;
  } = $props();

  const actions: {
    id: SelectionAction;
    label: string;
    icon: typeof PenLine;
  }[] = [
    { id: 'note', label: 'Add to notes', icon: PenLine },
    { id: 'ask', label: 'Ask about this', icon: MessageCircleQuestion },
    { id: 'cards', label: 'Flashcards', icon: Layers },
    { id: 'bookmark', label: 'Save moment', icon: BookmarkPlus },
  ];
</script>

<!-- mousedown is cancelled so using the toolbar never collapses the selection -->
<div
  class="sel-toolbar"
  class:below={placement === 'below'}
  role="toolbar"
  aria-label="Selected text actions"
  style:left="{x}px"
  style:top="{y}px"
  tabindex="-1"
  onmousedown={(e) => e.preventDefault()}
  onpointerdown={(e) => e.stopPropagation()}
>
  {#each actions as a (a.id)}
    {@const Icon = a.icon}
    <button type="button" class="act" onclick={() => onAction(a.id)}>
      <Icon size={15} /><span>{a.label}</span>
    </button>
  {/each}
</div>

<style>
  .sel-toolbar {
    position: fixed;
    z-index: 60;
    display: flex;
    gap: 2px;
    padding: 4px;
    transform: translate(-50%, -100%);
    background: var(--surface-3);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    box-shadow: var(--elev-3);
    backdrop-filter: blur(14px);
    -webkit-backdrop-filter: blur(14px);
    animation: rise var(--motion-base) var(--ease-out);
  }
  .sel-toolbar.below {
    transform: translate(-50%, 0);
  }
  .act {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: var(--control-h);
    min-height: var(--control-h);
    padding: 0 11px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-primary);
    font-family: var(--font-body);
    font-size: 12.5px;
    font-weight: 500;
    white-space: nowrap;
    transition: background var(--motion-fast) var(--ease-out);
  }
  .act:hover {
    background: var(--surface-hover);
  }
  .act:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
  .act :global(svg) {
    color: var(--text-accent);
  }
  @keyframes rise {
    from {
      opacity: 0;
      margin-top: 4px;
    }
  }
  @media (max-width: 520px) {
    .act span {
      display: none;
    }
    .act {
      padding: 0 12px;
    }
  }
</style>
