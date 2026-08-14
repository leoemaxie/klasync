<script lang="ts">
  import type { Screen } from '$lib/types';
  import { triggerHaptic } from '$lib/native/haptics';
  import { Search, X, ArrowRight } from '@lucide/svelte';
  import type { AuthUser } from '$lib/api/auth';
  import {
    getShortcutActions,
    type ShortcutAction,
  } from './spotlight/spotlightActions';
  import SpotlightItem from './spotlight/SpotlightItem.svelte';

  let {
    isOpen = $bindable(false),
    screen = $bindable(),
    sessionCode = $bindable(''),
    currentUser = null,
  }: {
    isOpen: boolean;
    screen: Screen;
    sessionCode?: string;
    currentUser?: AuthUser | null;
  } = $props();

  let searchQuery = $state('');
  let searchInputRef: HTMLInputElement | null = $state(null);

  const actions = $derived(getShortcutActions(currentUser));
  const filteredActions = $derived(
    actions.filter(
      (a) =>
        a.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        a.description.toLowerCase().includes(searchQuery.toLowerCase())
    )
  );

  function handleGlobalKeyDown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      triggerHaptic('light');
      isOpen = !isOpen;
    }
  }

  $effect(() => {
    if (isOpen && searchInputRef) {
      setTimeout(() => searchInputRef?.focus(), 50);
    }
  });

  function executeAction(act: ShortcutAction) {
    triggerHaptic('medium');
    const res = act.action({ screen, isOpen });
    screen = res.screen;
    isOpen = res.isOpen;
  }

  function handleQuickJoinSubmit() {
    if (searchQuery.trim().length >= 4 && searchQuery.trim().length <= 8) {
      triggerHaptic('success');
      sessionCode = searchQuery.trim().toUpperCase();
      screen = 'join';
      isOpen = false;
      searchQuery = '';
    }
  }
</script>

<svelte:window onkeydown={handleGlobalKeyDown} />

{#if isOpen}
  <div
    class="spotlight-backdrop"
    role="presentation"
    onclick={() => (isOpen = false)}
    onkeydown={(e) => e.key === 'Escape' && (isOpen = false)}
  >
    <div
      class="spotlight-dialog panel"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      aria-label="Spotlight Quick Search"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="spotlight-header">
        <Search size={18} class="search-icon" />
        <input
          bind:this={searchInputRef}
          type="text"
          bind:value={searchQuery}
          placeholder="Search commands or enter 6-digit session code..."
          class="spotlight-input"
          onkeydown={(e) => e.key === 'Enter' && handleQuickJoinSubmit()}
        />
        {#if searchQuery}
          <button
            type="button"
            class="clear-btn text"
            onclick={() => (searchQuery = '')}
            aria-label="Clear query"
          >
            <X size={14} />
          </button>
        {/if}
        <span class="cmd-badge">ESC to close</span>
      </div>

      {#if searchQuery.trim().length >= 4 && searchQuery.trim().length <= 8}
        <div class="quick-code-option">
          <button
            type="button"
            class="quick-code-btn"
            onclick={handleQuickJoinSubmit}
          >
            <span>Join room <strong>{searchQuery.toUpperCase()}</strong></span>
            <ArrowRight size={14} />
          </button>
        </div>
      {/if}

      <div class="spotlight-results">
        <p class="section-label">QUICK NAVIGATION &amp; ACTIONS</p>
        {#each filteredActions as item (item.id)}
          <SpotlightItem
            title={item.title}
            description={item.description}
            icon={item.icon}
            onClick={() => executeAction(item)}
          />
        {/each}

        {#if filteredActions.length === 0 && !searchQuery}
          <p class="empty-hint">No matching actions found</p>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .spotlight-backdrop {
    position: fixed;
    inset: 0;
    z-index: 999;
    background: rgba(10, 5, 2, 0.85);
    backdrop-filter: blur(12px);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: clamp(40px, 12vh, 120px);
    padding-left: 16px;
    padding-right: 16px;
  }
  .spotlight-dialog {
    width: 100%;
    max-width: 640px;
    background: #180e07;
    border: 1px solid var(--color-cork-border);
    border-radius: 16px;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.8);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .spotlight-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--color-cork-border);
  }
  :global(.search-icon) {
    color: var(--color-driftwood);
  }
  .spotlight-input {
    flex: 1;
    background: transparent;
    border: 0;
    outline: none;
    color: var(--color-warm-cream);
    font-size: 16px;
    font-weight: 400;
  }
  .clear-btn {
    padding: 4px;
    color: var(--color-driftwood);
  }
  .cmd-badge {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--color-driftwood);
    border: 1px solid var(--color-cork-border);
    padding: 3px 8px;
    border-radius: 6px;
  }
  .quick-code-option {
    padding: 8px 16px;
    background: rgba(220, 80, 0, 0.1);
    border-bottom: 1px solid var(--color-cork-border);
  }
  .quick-code-btn {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: transparent;
    border: 0;
    color: var(--color-warm-cream);
    font-size: 14px;
    cursor: pointer;
    padding: 8px 12px;
  }
  .spotlight-results {
    padding: 12px 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 380px;
    overflow-y: auto;
  }
  .section-label {
    font-size: 10px;
    letter-spacing: 0.12em;
    color: var(--color-driftwood);
    margin-bottom: 6px;
    padding-left: 8px;
  }
  .empty-hint {
    padding: 16px;
    font-size: 13px;
    color: var(--color-driftwood);
    text-align: center;
  }
</style>
