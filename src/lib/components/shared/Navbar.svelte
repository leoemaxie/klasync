<script lang="ts">
  import type { Screen } from "$lib/types";
  import { logout } from "$lib/api/auth";

  let { screen = $bindable() }: { screen: Screen } = $props();
  let mobileMenuOpen = $state(false);

  function navigate(target: Screen) {
    screen = target;
    mobileMenuOpen = false;
  }

  async function handleLogout() {
    try {
      await logout();
    } catch {
      // Clear token locally regardless
    }
    navigate("home");
  }
</script>

<nav>
  <button class="brand" onclick={() => navigate("home")} aria-label="Klasync home">
    <svg class="brand-mark" viewBox="0 0 48 48" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
      <path d="M8 26 A18 18 0 0 1 40 26" fill="none" stroke="#dc5000" stroke-width="2.5" stroke-linecap="round"/>
      <text x="9" y="40" font-family="'Playfair Display', Georgia, serif" font-size="28" font-weight="500" fill="#ffedd7">K</text>
      <g fill="#ffedd7" opacity="0.75">
        <rect x="30" y="28" width="2" height="8" rx="1"/>
        <rect x="33.5" y="24" width="2" height="12" rx="1"/>
        <rect x="37" y="26" width="2" height="10" rx="1"/>
        <rect x="40.5" y="29" width="2" height="6" rx="1"/>
      </g>
    </svg>
    <span class="brand-name">Klasync</span>
  </button>

  <button
    class="mobile-toggle"
    onclick={() => (mobileMenuOpen = !mobileMenuOpen)}
    aria-label={mobileMenuOpen ? "Close menu" : "Open menu"}
    aria-expanded={mobileMenuOpen}
  >
    {#if mobileMenuOpen}
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
        <line x1="18" y1="6" x2="6" y2="18"></line>
        <line x1="6" y1="6" x2="18" y2="18"></line>
      </svg>
    {:else}
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
        <line x1="3" y1="6" x2="21" y2="6"></line>
        <line x1="3" y1="12" x2="21" y2="12"></line>
        <line x1="3" y1="18" x2="21" y2="18"></line>
      </svg>
    {/if}
  </button>

  <div class="nav-actions" class:open={mobileMenuOpen}>
    <button class="nav-btn text" onclick={() => navigate("join")}>Join Session</button>
    <button class="nav-btn text" onclick={() => navigate("student-login")}>Student Archive</button>
    <button class="nav-btn outline" onclick={() => navigate("lecturer-login")}>Lecturer Access</button>
    {#if screen === "lecturer" || screen === "archive" || screen === "live"}
      <button class="nav-btn text" onclick={handleLogout}>Sign Out</button>
    {/if}
  </div>
</nav>

<style>
  nav {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 100;
    height: var(--nav-height);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--card-padding);
    background: rgba(16, 9, 4, 0.88);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border-bottom: 1px solid var(--color-cork-border);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    background: transparent;
    border: 0;
    padding: 0;
    cursor: pointer;
    text-decoration: none;
    z-index: 101;
  }

  .brand-mark {
    width: 36px;
    height: 36px;
    flex-shrink: 0;
  }

  .brand-name {
    font-family: var(--font-display);
    font-size: 17px;
    font-weight: 500;
    color: var(--color-warm-cream);
    letter-spacing: -0.01em;
  }

  .mobile-toggle {
    display: none;
    background: transparent;
    border: 0;
    color: var(--color-warm-cream);
    cursor: pointer;
    padding: 8px;
    border-radius: var(--radius-buttons-outlined);
    z-index: 101;
    align-items: center;
    justify-content: center;
  }

  .mobile-toggle:hover {
    color: var(--color-ember-accent);
  }

  .nav-actions {
    display: flex;
    align-items: center;
    gap: var(--spacing-24);
  }

  @media (max-width: 768px) {
    .mobile-toggle {
      display: flex;
    }

    .nav-actions {
      position: fixed;
      top: var(--nav-height);
      left: 0;
      right: 0;
      background: rgba(16, 9, 4, 0.96);
      backdrop-filter: blur(20px);
      -webkit-backdrop-filter: blur(20px);
      border-bottom: 1px solid var(--color-cork-border);
      flex-direction: column;
      align-items: stretch;
      padding: var(--spacing-24) var(--card-padding);
      gap: var(--spacing-14);
      transform: translateY(-100%);
      opacity: 0;
      pointer-events: none;
      transition: transform 0.25s cubic-bezier(0.22, 1, 0.36, 1), opacity 0.25s ease;
      box-shadow: 0 16px 32px rgba(0, 0, 0, 0.5);
    }

    .nav-actions.open {
      transform: translateY(0);
      opacity: 1;
      pointer-events: auto;
    }

    .nav-btn {
      width: 100%;
      text-align: center;
      padding: 12px var(--spacing-18);
    }

    .nav-btn.text {
      text-decoration: none;
      border: 1px solid rgba(255, 237, 215, 0.1);
      border-radius: var(--radius-buttons-outlined);
    }
  }
</style>
