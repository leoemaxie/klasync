<script lang="ts">
  import { onMount } from 'svelte';
  import type { Screen } from '$lib/types';
  import { logout } from '$lib/api/auth';
  import type { SessionState } from '$lib/sessionState.svelte';
  import { triggerHaptic } from '$lib/native/haptics';
  import { platform } from '$lib/native/platform';
  import {
    minimizeWindow,
    toggleMaximizeWindow,
    closeWindow,
  } from '$lib/native/window';
  import { Menu, X, Search } from '@lucide/svelte';
  import BrandLogo from './navbar/BrandLogo.svelte';

  let {
    screen = $bindable(),
    appState,
    onOpenSpotlight,
  }: {
    screen: Screen;
    appState?: SessionState;
    onOpenSpotlight?: () => void;
  } = $props();
  let mobileMenuOpen = $state(false);
  let isTauri = $state(false);
  let isMac = $state(false);

  onMount(() => {
    isTauri = platform.isTauri;
    isMac = platform.isMacOS;
  });

  function triggerSpotlight() {
    triggerHaptic('light');
    if (onOpenSpotlight) onOpenSpotlight();
    mobileMenuOpen = false;
  }

  function navigate(target: Screen) {
    triggerHaptic('light');
    screen = target;
    mobileMenuOpen = false;
  }

  function handleLecturerAccess() {
    triggerHaptic('medium');
    const role = appState?.currentUser?.role;
    if (role === 'lecturer' || role === 'admin') {
      navigate('lecturer');
    } else {
      if (appState) {
        appState.authNotice =
          role === 'student'
            ? 'Please sign in with a lecturer account.'
            : 'Please sign in to continue.';
      }
      navigate('lecturer-login');
    }
  }

  function handleStudentArchive() {
    triggerHaptic('light');
    const role = appState?.currentUser?.role;
    if (role === 'student') {
      navigate('archive');
    } else {
      if (appState) {
        appState.authNotice =
          role === 'lecturer' || role === 'admin'
            ? 'Please sign in with a student account.'
            : 'Please sign in to continue.';
      }
      navigate('student-login');
    }
  }

  async function handleLogout() {
    triggerHaptic('warning');
    try {
      await logout();
    } catch {}
    if (appState) {
      appState.currentUser = null;
      appState.authNotice = '';
      appState.lecturerName = '';
      appState.lecturerEmail = '';
      localStorage.removeItem('klasync-user');
      localStorage.removeItem('klasync-lecturer');
    }
    navigate('home');
    mobileMenuOpen = false;
  }
</script>

<svelte:window
  onkeydown={(e) =>
    e.key === 'Escape' && mobileMenuOpen && (mobileMenuOpen = false)}
/>

<nav
  class="navbar app-drag-region"
  ondblclick={isTauri ? toggleMaximizeWindow : undefined}
>
  <BrandLogo
    onClick={() => {
      const role = appState?.currentUser?.role;
      if (role === 'student') navigate('archive');
      else if (role === 'lecturer' || role === 'admin') navigate('lecturer');
      else navigate('home');
    }}
  />

  <button
    class="mobile-toggle app-no-drag"
    onclick={() => {
      triggerHaptic('light');
      mobileMenuOpen = !mobileMenuOpen;
    }}
    aria-label="Toggle navigation menu"
    aria-expanded={mobileMenuOpen}
    aria-controls="nav-actions-menu"
  >
    {#if mobileMenuOpen}<X size={22} aria-hidden="true" />{:else}<Menu
        size={22}
        aria-hidden="true"
      />{/if}
  </button>

  <div
    id="nav-actions-menu"
    class="nav-actions app-no-drag"
    class:open={mobileMenuOpen}
  >
    <button
      class="nav-btn text search-trigger"
      onclick={triggerSpotlight}
      aria-label="Spotlight search"
    >
      <Search size={14} aria-hidden="true" style="vertical-align: middle;" />
      <span class="spotlight-kbd">⌘K</span>
    </button>

    {#if !appState?.currentUser || appState.currentUser.role === 'student'}
      <button class="nav-btn text" onclick={() => navigate('join')}
        >Join Session</button
      >
      <button class="nav-btn text" onclick={handleStudentArchive}
        >Archive</button
      >
    {/if}

    {#if appState?.currentUser}
      <div class="user-pill" title={appState.currentUser.email}>
        <span class="user-name"
          >{appState.currentUser.name || appState.currentUser.email}</span
        >
      </div>
      <button class="nav-btn danger" onclick={handleLogout}>Sign Out</button>
    {:else}
      <button class="nav-btn outline" onclick={handleLecturerAccess}
        >Lecturer Sign In</button
      >
    {/if}

    {#if isTauri && !isMac}
      <div class="window-controls app-no-drag" role="group" aria-label="Window controls">
        <button
          type="button"
          class="win-btn"
          onclick={minimizeWindow}
          aria-label="Minimize"
          title="Minimize"
        >
          <svg width="10" height="1" viewBox="0 0 10 1"
            ><rect width="10" height="1" fill="currentColor" /></svg
          >
        </button>
        <button
          type="button"
          class="win-btn"
          onclick={toggleMaximizeWindow}
          aria-label="Maximize"
          title="Maximize"
        >
          <svg width="10" height="10" viewBox="0 0 10 10"
            ><rect
              width="9"
              height="9"
              x="0.5"
              y="0.5"
              fill="none"
              stroke="currentColor"
            /></svg
          >
        </button>
        <button
          type="button"
          class="win-btn close"
          onclick={closeWindow}
          aria-label="Close"
          title="Close"
        >
          <svg width="10" height="10" viewBox="0 0 10 10"
            ><path
              d="M1,1 L9,9 M9,1 L1,9"
              stroke="currentColor"
              stroke-width="1.2"
            /></svg
          >
        </button>
      </div>
    {/if}
  </div>
</nav>

<style>
  .navbar {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    height: var(--nav-height);
    background: rgba(16, 9, 4, 0.85);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border-bottom: 1px solid var(--color-cork-border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--card-padding);
    z-index: 100;
    user-select: none;
  }
  .mobile-toggle {
    display: none;
    background: transparent;
    border: none;
    color: var(--color-warm-cream);
    cursor: pointer;
    padding: 6px;
  }
  .nav-actions {
    display: flex;
    align-items: center;
    gap: var(--spacing-16);
  }
  .nav-btn {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    padding: 6px 14px;
    border-radius: 4px;
    min-height: 32px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .search-trigger {
    display: flex;
    align-items: center;
    gap: var(--spacing-8);
  }
  .spotlight-kbd {
    font-size: 10px;
    color: var(--color-driftwood);
    border: 1px solid var(--color-cork-border);
    padding: 1px 5px;
    border-radius: 4px;
  }
  .user-pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0 14px;
    height: 32px;
    border: 1px solid var(--color-cork-border);
    border-radius: 999px;
    background: rgba(255, 237, 215, 0.06);
    box-sizing: border-box;
  }
  .user-name {
    font-size: 11px;
    font-weight: 500;
    color: var(--color-driftwood);
    line-height: 1;
    display: inline-flex;
    align-items: center;
    letter-spacing: 0.02em;
    transform: translateY(-0.5px);
  }
  .window-controls {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: var(--spacing-8);
    border-left: 1px solid var(--color-cork-border);
    padding-left: var(--spacing-8);
  }
  .win-btn {
    width: 36px;
    height: 32px;
    background: transparent;
    border: 0;
    color: var(--color-driftwood);
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    padding: 0;
    transition: background 0.15s ease, color 0.15s ease;
  }
  .win-btn:hover {
    background: rgba(255, 237, 215, 0.1);
    color: var(--color-warm-cream);
  }
  .win-btn.close:hover {
    background: #dc3545;
    color: #ffffff;
  }
  @media (max-width: 800px) {
    .mobile-toggle {
      display: block;
    }
    .nav-actions {
      display: none;
      position: absolute;
      top: var(--nav-height);
      left: 0;
      right: 0;
      flex-direction: column;
      background: #100904;
      border-bottom: 1px solid var(--color-cork-border);
      padding: var(--spacing-18) var(--card-padding);
      gap: var(--spacing-12);
      align-items: stretch;
    }
    .nav-actions.open {
      display: flex;
    }
    .nav-btn {
      width: 100%;
      text-align: center;
    }
    .window-controls {
      display: none;
    }
  }
</style>
