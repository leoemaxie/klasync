<script lang="ts">
  import type { Screen } from '$lib/types';
  import { logout } from '$lib/api/auth';
  import type { SessionState } from '$lib/sessionState.svelte';
  import { Menu, X } from '@lucide/svelte';

  let { screen = $bindable(), appState }: { screen: Screen; appState?: SessionState } = $props();
  let mobileMenuOpen = $state(false);

  function navigate(target: Screen) {
    screen = target;
    mobileMenuOpen = false;
  }

  function handleLecturerAccess() {
    if (appState?.currentUser && (appState.currentUser.role === 'lecturer' || appState.currentUser.role === 'admin')) {
      navigate('lecturer');
    } else {
      if (appState) appState.authNotice = 'Please sign in to access the Lecturer Workspace.';
      navigate('lecturer-login');
    }
  }

  function handleStudentArchive() {
    if (appState?.currentUser) { navigate('archive'); }
    else { if (appState) appState.authNotice = 'Please sign in to access your Student Archive.'; navigate('student-login'); }
  }

  async function handleLogout() {
    try { await logout(); } catch {}
    if (appState) { appState.currentUser = null; appState.authNotice = ''; localStorage.removeItem('klasync-user'); }
    navigate('home');
    mobileMenuOpen = false;
  }
</script>

<nav class="navbar">
  <button class="brand" onclick={() => navigate('home')} aria-label="Klasync home">
    <svg class="brand-mark" viewBox="0 0 48 48" xmlns="http://www.w3.org/2000/svg">
      <path d="M8 26 A18 18 0 0 1 40 26" fill="none" stroke="#dc5000" stroke-width="2.5" stroke-linecap="round"/>
      <text x="9" y="40" font-family="'Playfair Display', Georgia, serif" font-size="28" font-weight="500" fill="#ffedd7">K</text>
      <g fill="#ffedd7" opacity="0.75">
        <rect x="30" y="28" width="2" height="8" rx="1" /><rect x="33.5" y="24" width="2" height="12" rx="1" /><rect x="37" y="26" width="2" height="10" rx="1" /><rect x="40.5" y="29" width="2" height="6" rx="1" />
      </g>
    </svg>
    <span class="brand-name">Klasync</span>
  </button>

  <button class="mobile-toggle" onclick={() => (mobileMenuOpen = !mobileMenuOpen)} aria-label="Toggle menu">
    {#if mobileMenuOpen}<X size={22} />{:else}<Menu size={22} />{/if}
  </button>

  <div class="nav-actions" class:open={mobileMenuOpen}>
    <button class="nav-btn text" onclick={() => navigate('join')}>Join Session</button>
    <button class="nav-btn text" onclick={handleStudentArchive}>Student Archive</button>

    {#if appState?.currentUser}
      {#if appState.currentUser.role === 'lecturer' || appState.currentUser.role === 'admin'}
        <button class="nav-btn outline" onclick={() => navigate('lecturer')}>Lecturer Workspace</button>
      {/if}
      <div class="user-pill" title={appState.currentUser.email}>
        <span class="user-role-tag">{appState.currentUser.role}</span>
        <span class="user-name">{appState.currentUser.name || appState.currentUser.email}</span>
      </div>
      <button class="nav-btn danger" onclick={handleLogout}>Sign Out</button>
    {:else}
      <button class="nav-btn outline" onclick={handleLecturerAccess}>Lecturer Access</button>
    {/if}
  </div>
</nav>

<style>
  nav.navbar { position: fixed; top: 0; left: 0; right: 0; z-index: 100; height: var(--nav-height); display: flex; align-items: center; justify-content: space-between; padding: 0 var(--card-padding); background: rgba(16, 9, 4, 0.9); backdrop-filter: blur(16px); border-bottom: 1px solid var(--color-cork-border); }
  .brand { display: flex; align-items: center; gap: 10px; background: transparent; border: 0; padding: 0; cursor: pointer; }
  .brand-mark { width: 34px; height: 34px; }
  .brand-name { font-family: var(--font-display); font-size: 17px; font-weight: 500; color: var(--color-warm-cream); }
  .mobile-toggle { display: none; background: transparent; border: 0; color: var(--color-warm-cream); cursor: pointer; padding: 8px; }
  .nav-actions { display: flex; align-items: center; gap: var(--spacing-18); }
  .user-pill { display: flex; align-items: center; gap: 8px; background: rgba(56, 36, 22, 0.4); border: 1px solid var(--color-cork-border); padding: 4px 10px; border-radius: var(--radius-buttons-outlined); font-size: 11px; }
  .user-role-tag { font-size: 9px; text-transform: uppercase; letter-spacing: 0.1em; background: rgba(220, 80, 0, 0.2); color: var(--color-ember-accent); padding: 2px 6px; border-radius: 4px; font-weight: 700; }
  .user-name { color: var(--color-warm-cream); max-width: 130px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  @media (max-width: 768px) {
    .mobile-toggle { display: flex; }
    .nav-actions { position: fixed; top: var(--nav-height); left: 0; right: 0; background: rgba(16, 9, 4, 0.96); backdrop-filter: blur(20px); border-bottom: 1px solid var(--color-cork-border); flex-direction: column; align-items: stretch; padding: var(--spacing-20); gap: var(--spacing-12); transform: translateY(-100%); opacity: 0; pointer-events: none; transition: transform 0.25s ease, opacity 0.25s ease; }
    .nav-actions.open { transform: translateY(0); opacity: 1; pointer-events: auto; }
    .nav-btn { width: 100%; text-align: center; }
  }
</style>
