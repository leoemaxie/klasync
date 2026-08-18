<script lang="ts">
  import type { Screen } from '$lib/types';
  import type { SessionState } from '$lib/sessionState.svelte';
  import { triggerHaptic } from '$lib/native/haptics';
  import { logout } from '$lib/api/auth';
  import { purgeSensitiveAuthStorage } from '$lib/authGuard';
  import {
    Home,
    Radio,
    BookOpen,
    User,
    Search,
    LogOut,
    Layers,
  } from '@lucide/svelte';

  let {
    screen = $bindable(),
    appState,
    onOpenSpotlight,
  }: {
    screen: Screen;
    appState?: SessionState;
    onOpenSpotlight?: () => void;
  } = $props();

  const role = $derived(appState?.currentUser?.role);
  const isLecturer = $derived(role === 'lecturer' || role === 'admin');
  const isStudent = $derived(role === 'student');

  function selectTab(target: Screen) {
    triggerHaptic('light');
    screen = target;
  }

  async function handleLogout() {
    triggerHaptic('warning');
    try {
      await logout();
    } catch {}
    purgeSensitiveAuthStorage();
    if (appState) {
      appState.currentUser = null;
      appState.authNotice = '';
      appState.lecturerName = '';
      appState.lecturerEmail = '';
    }
    screen = 'home';
  }
</script>

<nav class="mobile-bottom-nav" aria-label="Mobile navigation">
  {#if isLecturer}
    <button
      type="button"
      class="tab-item"
      class:active={screen === 'lecturer'}
      onclick={() => selectTab('lecturer')}
    >
      <Layers size={18} /><span>Workspace</span>
    </button>
    <button
      type="button"
      class="tab-item"
      class:active={screen === 'live'}
      onclick={() => selectTab(appState?.session?.live ? 'live' : 'lecturer')}
    >
      <Radio size={18} /><span>Live</span>
    </button>
    <button
      type="button"
      class="tab-item"
      onclick={() => {
        triggerHaptic('light');
        onOpenSpotlight?.();
      }}
    >
      <Search size={18} /><span>Search</span>
    </button>
    <button type="button" class="tab-item" onclick={handleLogout}>
      <LogOut size={18} /><span>Sign Out</span>
    </button>
  {:else if isStudent}
    <button
      type="button"
      class="tab-item"
      class:active={screen === 'archive'}
      onclick={() => selectTab('archive')}
    >
      <BookOpen size={18} /><span>Archive</span>
    </button>
    <button
      type="button"
      class="tab-item"
      class:active={screen === 'join' || screen === 'live'}
      onclick={() => selectTab(appState?.session?.live ? 'live' : 'join')}
    >
      <Radio size={18} /><span>Join</span>
    </button>
    <button
      type="button"
      class="tab-item"
      onclick={() => {
        triggerHaptic('light');
        onOpenSpotlight?.();
      }}
    >
      <Search size={18} /><span>Search</span>
    </button>
    <button type="button" class="tab-item" onclick={handleLogout}>
      <LogOut size={18} /><span>Sign Out</span>
    </button>
  {:else}
    <button
      type="button"
      class="tab-item"
      class:active={screen === 'home'}
      onclick={() => selectTab('home')}
    >
      <Home size={18} /><span>Home</span>
    </button>
    <button
      type="button"
      class="tab-item"
      class:active={screen === 'join' || screen === 'live'}
      onclick={() => selectTab('join')}
    >
      <Radio size={18} /><span>Join</span>
    </button>
    <button
      type="button"
      class="tab-item"
      class:active={screen === 'lecturer-login' ||
        screen === 'lecturer-register'}
      onclick={() => selectTab('lecturer-login')}
    >
      <BookOpen size={18} /><span>Lecturer</span>
    </button>
    <button
      type="button"
      class="tab-item"
      class:active={screen === 'student-login' || screen === 'student-register'}
      onclick={() => selectTab('student-login')}
    >
      <User size={18} /><span>Student</span>
    </button>
  {/if}
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
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .tab-item.active {
    color: var(--color-warm-cream);
  }
  .tab-item.active :global(svg) {
    color: var(--color-ember-accent);
  }
  @media (max-width: 800px) {
    .mobile-bottom-nav {
      display: flex;
    }
  }
</style>
