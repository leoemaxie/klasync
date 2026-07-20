<script lang="ts">
  import type { Screen } from "$lib/types";

  let { screen = $bindable() }: { screen: Screen } = $props();
</script>

<nav>
  <!-- Inline SVG logo — the K with arc and waveform -->
  <button class="brand" onclick={() => (screen = "home")} aria-label="Klasync home">
    <svg class="brand-mark" viewBox="0 0 48 48" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
      <!-- Orange arc -->
      <path d="M8 26 A18 18 0 0 1 40 26" fill="none" stroke="#dc5000" stroke-width="2.5" stroke-linecap="round"/>
      <!-- K letterform -->
      <text x="9" y="40" font-family="'Playfair Display', Georgia, serif" font-size="28" font-weight="500" fill="#ffedd7">K</text>
      <!-- Waveform bars -->
      <g fill="#ffedd7" opacity="0.75">
        <rect x="30" y="28" width="2" height="8" rx="1"/>
        <rect x="33.5" y="24" width="2" height="12" rx="1"/>
        <rect x="37" y="26" width="2" height="10" rx="1"/>
        <rect x="40.5" y="29" width="2" height="6" rx="1"/>
      </g>
    </svg>
    <span class="brand-name">Klasync</span>
  </button>

  <div class="nav-actions">
    <button class="text" onclick={() => (screen = "join")}>Join a session</button>
    <button class="outline" onclick={() => (screen = "lecturer")}>Lecturer workspace</button>
  </div>
</nav>

<style>
  nav {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 10;
    height: var(--nav-height);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--card-padding);
    /* frosted dark glass */
    background: rgba(16, 9, 4, 0.82);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
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

  .nav-actions {
    display: flex;
    align-items: center;
    gap: var(--spacing-24);
  }

  @media (max-width: 600px) {
    .brand-name {
      display: none;
    }
    .nav-actions {
      gap: var(--spacing-14);
    }
  }
</style>
