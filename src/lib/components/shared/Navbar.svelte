<script lang="ts">
  import type { Screen } from '$lib/types';
  import { logout } from '$lib/api/auth';
  import type { SessionState } from '$lib/sessionState.svelte';
  import { triggerHaptic } from '$lib/native/haptics';
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
        appState.authNotice = role === 'student'
          ? 'Access restricted: Lecturer Workspace is only accessible to lecturer accounts.'
          : 'Please sign in to access the Lecturer Workspace.';
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
        appState.authNotice = role === 'lecturer' || role === 'admin'
          ? 'Access restricted: Student Archive is only accessible to student accounts.'
          : 'Please sign in to access your Student Archive.';
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

<svelte:window onkeydown={(e) => e.key === 'Escape' && mobileMenuOpen && (mobileMenuOpen = false)} />

<nav class="navbar app-drag-region">
  <BrandLogo onClick={() => navigate('home')} />

  <button
    class="mobile-toggle app-no-drag"
    onclick={() => { triggerHaptic('light'); mobileMenuOpen = !mobileMenuOpen; }}
    aria-label="Toggle navigation menu"
    aria-expanded={mobileMenuOpen}
    aria-controls="nav-actions-menu"
  >
    {#if mobileMenuOpen}<X size={22} aria-hidden="true" />{:else}<Menu size={22} aria-hidden="true" />{/if}
  </button>

  <div id="nav-actions-menu" class="nav-actions app-no-drag" class:open={mobileMenuOpen}>
    <button class="nav-btn text search-trigger" onclick={triggerSpotlight} aria-label="Spotlight search">
      <Search size={14} aria-hidden="true" style="vertical-align: middle;" />
      <span class="spotlight-kbd">⌘K</span>
    </button>

    {#if !appState?.currentUser || appState.currentUser.role === 'student'}
      <button class="nav-btn text" onclick={() => navigate('join')}>Join Session</button>
      <button class="nav-btn text" onclick={handleStudentArchive}>Courses &amp; Archive</button>
    {/if}

    {#if appState?.currentUser}
      <div class="user-pill" title={appState.currentUser.email}>
        <span class="user-name">{appState.currentUser.name || appState.currentUser.email}</span>
      </div>
      <button class="nav-btn danger" onclick={handleLogout}>Sign Out</button>
    {:else}
      <button class="nav-btn outline" onclick={handleLecturerAccess}>Lecturer Access</button>
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
    background: rgba(16, 9, 4, 0.72);
    backdrop-filter: blur(16px);
    border-bottom: 1px solid var(--color-cork-border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--card-padding);
    z-index: 100;
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
    padding: 6px 12px;
    border: 1px solid var(--color-cork-border);
    border-radius: 999px;
    background: rgba(255, 237, 215, 0.06);
  }
  .user-name {
    font-size: 11px;
    font-weight: 500;
    color: var(--color-driftwood);
  }
  @media (max-width: 800px) {
    .mobile-toggle { display: block; }
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
    .nav-actions.open { display: flex; }
    .nav-btn { width: 100%; text-align: center; }
  }
</style>
