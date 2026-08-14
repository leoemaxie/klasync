<script lang="ts">
  import { ArrowRight } from '@lucide/svelte';
  import type { Component } from 'svelte';

  let {
    title,
    description,
    icon: Icon,
    onClick,
  }: {
    title: string;
    description: string;
    icon: Component<any>;
    onClick: () => void;
  } = $props();
</script>

<button type="button" class="spotlight-item" onclick={onClick}>
  <div class="item-icon-wrap">
    <Icon size={16} />
  </div>
  <div class="item-text">
    <span class="item-title">{title}</span>
    <span class="item-desc">{description}</span>
  </div>
  <ArrowRight size={14} class="item-arrow" />
</button>

<style>
  .spotlight-item {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 14px;
    border-radius: 10px;
    background: transparent;
    border: 1px solid transparent;
    color: var(--color-warm-cream);
    text-align: left;
    cursor: pointer;
    transition: background 0.15s ease, border-color 0.15s ease;
    width: 100%;
  }
  .spotlight-item:hover {
    background: rgba(56, 36, 22, 0.6);
    border-color: var(--color-cork-border);
  }
  .item-icon-wrap {
    color: var(--color-ember-accent);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .item-text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .item-title {
    font-size: 14px;
    font-weight: 500;
  }
  .item-desc {
    font-size: 12px;
    color: var(--color-driftwood);
  }
  :global(.item-arrow) {
    color: var(--color-driftwood);
    opacity: 0.6;
  }
</style>
