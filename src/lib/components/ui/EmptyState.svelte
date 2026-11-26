<script lang="ts">
  import type { Component, Snippet } from 'svelte';

  let {
    icon: Icon,
    title,
    description = '',
    tone = 'neutral',
    children,
  }: {
    icon?: Component<{ size?: number | string }>;
    title: string;
    description?: string;
    tone?: 'neutral' | 'error';
    /** Optional actions rendered under the text. */
    children?: Snippet;
  } = $props();
</script>

<div
  class="ui-empty"
  data-tone={tone}
  role={tone === 'error' ? 'alert' : undefined}
>
  {#if Icon}<span class="glyph"><Icon size={20} /></span>{/if}
  <h3>{title}</h3>
  {#if description}<p>{description}</p>{/if}
  {@render children?.()}
</div>

<style>
  .ui-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 32px 20px;
    text-align: center;
    min-height: 0;
  }
  .glyph {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 50%;
    background: var(--surface-3);
    border: 1px solid var(--border-default);
    color: var(--text-secondary);
  }
  [data-tone='error'] .glyph {
    border-color: rgba(240, 160, 48, 0.4);
    color: var(--color-error);
  }
  h3 {
    margin: 2px 0 0;
    font-size: 20px;
  }
  p {
    max-width: 340px;
    margin: 0;
    font-size: 14px;
    line-height: 1.55;
    color: var(--text-secondary);
  }
  p :global(strong) {
    color: var(--text-primary);
  }
</style>
