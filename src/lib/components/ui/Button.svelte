<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type Props = HTMLButtonAttributes & {
    /** solid = cream, accent = ember fill (AA), outline, ghost, soft = tinted ember */
    variant?: 'solid' | 'accent' | 'outline' | 'ghost' | 'soft';
    size?: 'sm' | 'md';
    /** Square/round icon-only button; give it an aria-label. */
    icon?: boolean;
    pressed?: boolean;
    children?: Snippet;
  };

  let {
    variant = 'ghost',
    size = 'md',
    icon = false,
    pressed,
    type = 'button',
    children,
    class: className = '',
    ...rest
  }: Props = $props();
</script>

<button
  {type}
  class="ui-btn {className}"
  data-variant={variant}
  data-size={size}
  data-icon={icon || undefined}
  data-pressed={pressed || undefined}
  aria-pressed={pressed}
  {...rest}
>
  {@render children?.()}
</button>

<style>
  .ui-btn {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    flex: none;
    height: var(--control-h);
    min-height: var(--control-h);
    padding: 0 14px;
    border: 1px solid transparent;
    border-radius: 999px;
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--font-body);
    font-size: 13px;
    font-weight: 500;
    letter-spacing: 0;
    text-transform: none;
    white-space: nowrap;
    transition:
      background var(--motion-fast) var(--ease-out),
      border-color var(--motion-fast) var(--ease-out),
      color var(--motion-fast) var(--ease-out),
      transform var(--motion-fast) var(--ease-out);
  }
  .ui-btn[data-size='sm'] {
    height: calc(var(--control-h) - 8px);
    min-height: calc(var(--control-h) - 8px);
    padding: 0 11px;
    font-size: 12.5px;
  }
  .ui-btn[data-icon] {
    width: var(--control-h);
    min-width: var(--control-h);
    padding: 0;
  }
  .ui-btn[data-icon][data-size='sm'] {
    width: calc(var(--control-h) - 8px);
    min-width: calc(var(--control-h) - 8px);
  }

  /* ghost */
  .ui-btn:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text-primary);
  }
  .ui-btn[data-pressed] {
    background: var(--surface-accent);
    color: var(--text-primary);
  }
  .ui-btn[data-pressed] :global(svg) {
    color: var(--text-accent);
  }

  /* outline */
  .ui-btn[data-variant='outline'] {
    border-color: var(--border-default);
  }
  .ui-btn[data-variant='outline']:hover:not(:disabled) {
    border-color: var(--border-strong);
  }
  .ui-btn[data-variant='outline'][data-pressed] {
    border-color: var(--border-accent);
  }

  /* soft (tinted ember) */
  .ui-btn[data-variant='soft'] {
    border-color: var(--border-accent);
    background: var(--surface-accent);
    color: var(--text-primary);
  }
  .ui-btn[data-variant='soft']:hover:not(:disabled) {
    background: rgba(220, 80, 0, 0.26);
  }
  .ui-btn[data-variant='soft'] :global(svg) {
    color: var(--text-accent);
  }

  /* accent (filled ember, AA with --text-on-accent) */
  .ui-btn[data-variant='accent'] {
    background: var(--fill-accent);
    color: var(--text-on-accent);
    font-weight: 600;
  }
  .ui-btn[data-variant='accent']:hover:not(:disabled) {
    background: var(--fill-accent-hover);
    color: var(--text-on-accent);
  }

  /* solid (cream) */
  .ui-btn[data-variant='solid'] {
    background: var(--text-primary);
    color: var(--color-walnut-shadow);
    font-weight: 600;
    box-shadow: var(--elev-2);
  }
  .ui-btn[data-variant='solid']:hover:not(:disabled) {
    background: var(--text-primary);
    color: var(--color-walnut-shadow);
    transform: scale(1.04);
  }
  .ui-btn[data-variant='solid']:active:not(:disabled) {
    transform: scale(0.97);
  }
  .ui-btn[data-variant='solid']:disabled {
    background: var(--surface-3);
    color: var(--text-tertiary);
    box-shadow: none;
    opacity: 1;
  }

  .ui-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .ui-btn:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
</style>
