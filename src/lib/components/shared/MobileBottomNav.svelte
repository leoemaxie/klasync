<script lang="ts">
  import type { Screen } from '$lib/types';
  import type { SessionState } from '$lib/sessionState.svelte';
  import { triggerHaptic } from '$lib/native/haptics';
  import { Home, Radio, BookOpen, User } from '@lucide/svelte';

  let { screen = $bindable(), appState }: { screen: Screen; appState?: SessionState } = $props();

  function selectTab(target: Screen) {
    triggerHaptic('light');
    screen = target;
  }

  function handleAccountTap() {
    triggerHaptic('medium');
    const role = appState?.currentUser?.role;
    screen = role === 'lecturer' || role === 'admin' ? 'lecturer' : role === 'student' ? 'archive' : 'lecturer-login';
  }

  const isArchive = $derived(screen === 'archive');
  const isHome = $derived(screen === 'home');
  const isJoin = $derived(screen === 'join' || screen === 'live');
  const isAccount = $derived(screen === 'lecturer' || screen === 'lecturer-login' || screen === 'student-login');
</script>

<nav class="mobile-bottom-nav" aria-label="Mobile navigation">
  <button type="button" class="tab-item" class:active={isHome} onclick={() => selectTab('home')}>
    <Home size={18} /><span>Home</span>
  </button>
  <button type="button" class="tab-item" class:active={isJoin} onclick={() => selectTab('join')}>
    <Radio size={18} /><span>Join Live</span>
  </button>
  <button type="button" class="tab-item" class:active={isArchive} onclick={() => selectTab('archive')}>
    <BookOpen size={18} /><span>Studio</span>
  </button>
  <button type="button" class="tab-item" class:active={isAccount} onclick={handleAccountTap}>
    <User size={18} /><span>Account</span>
  </button>
</nav>

<style>
  .mobile-bottom-nav {
    display: none;
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    background: rgba(16, 9, 4, 0.92);
    backdrop-filter: blur(20px);
    border-top: 1px solid var(--color-cork-border);
    padding: 6px 12px calc(6px + env(safe-area-inset-bottom, 0px));
    z-index: 90;
    justify-content: space-around;
    align-items: center;
  }
  .tab-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    background: transparent;
    border: none;
    color: var(--color-driftwood);
    font-size: 10px;
    font-weight: 500;
    text-transform: uppercase;
    padding: 6px 12px;
    min-height: 44px;
    min-width: 54px;
    border-radius: 6px;
    transition: all 0.15s ease;
  }
  .tab-item.active { color: var(--color-warm-cream); }
  .tab-item.active :global(svg) { color: var(--color-ember-accent); }
  @media (max-width: 800px) {
    .mobile-bottom-nav { display: flex; }
  }
</style>
