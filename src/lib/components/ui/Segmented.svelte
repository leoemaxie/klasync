<script lang="ts" generics="T extends string">
  import type { Component } from 'svelte';

  type Option = {
    value: T;
    label: string;
    icon?: Component<{ size?: number | string }>;
  };

  let {
    options,
    value = $bindable(),
    label,
  }: { options: Option[]; value: T; label: string } = $props();
</script>

<div class="ui-seg" role="group" aria-label={label}>
  {#each options as option (option.value)}
    {@const Icon = option.icon}
    <button
      type="button"
      class:on={value === option.value}
      aria-pressed={value === option.value}
      onclick={() => (value = option.value)}
    >
      {#if Icon}<Icon size={14} />{/if}
      {option.label}
    </button>
  {/each}
</div>

<style>
  .ui-seg {
    display: inline-flex;
    flex: none;
    padding: 3px;
    gap: 2px;
    background: var(--surface-inset);
    border: 1px solid var(--border-default);
    border-radius: 999px;
  }
  button {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: calc(var(--control-h) - 8px);
    min-height: calc(var(--control-h) - 8px);
    padding: 0 9px;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--font-body);
    font-size: 12.5px;
    font-weight: 500;
    letter-spacing: 0;
    text-transform: none;
    transition:
      background var(--motion-fast) var(--ease-out),
      color var(--motion-fast) var(--ease-out);
  }
  button:hover:not(.on) {
    color: var(--text-primary);
  }
  button.on {
    background: var(--surface-3);
    color: var(--text-primary);
  }
  button:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }
</style>
