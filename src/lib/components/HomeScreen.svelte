<script lang="ts">
  import type { Screen } from "$lib/types";
  import { onMount } from "svelte";

  let { screen = $bindable() }: { screen: Screen } = $props();

  let visible = $state(false);
  onMount(() => {
    requestAnimationFrame(() => { visible = true; });
  });
</script>

<!-- ── Hero ──────────────────────────────────────────────── -->
<section class="hero" class:visible>

  <!-- Decorative arc — echoes the logo -->
  <div class="hero-arc" aria-hidden="true">
    <svg viewBox="0 0 800 400" preserveAspectRatio="xMidYMid meet" xmlns="http://www.w3.org/2000/svg">
      <path d="M 0 380 A 400 400 0 0 1 800 380" fill="none" stroke="#dc5000" stroke-width="1.5" opacity="0.35"/>
      <path d="M 60 380 A 340 340 0 0 1 740 380" fill="none" stroke="#dc5000" stroke-width="0.8" opacity="0.2"/>
    </svg>
  </div>

  <!-- Left: text content -->
  <div class="hero-content">
    <p class="eyebrow">
      <span class="eyebrow-accent">●</span>&nbsp; Accessible learning &nbsp;/&nbsp; Built for the room
    </p>

    <h1>
      Every lecture,<br />
      <em>within reach.</em>
    </h1>

    <p class="lede">
      Live captions, fair attendance, and a lasting learning
      archive — without making students create an account just to join class.
    </p>

    <div class="actions">
      <button class="primary" onclick={() => (screen = "join")}>
        Join a live lecture
      </button>
      <button class="primary-bark" onclick={() => (screen = "lecturer")}>
        Lecturer workspace
      </button>
    </div>
  </div>

  <!-- Right: animated waveform visual -->
  <div class="hero-visual" aria-hidden="true">
    <div class="waveform">
      {#each [0.4, 0.7, 1, 0.85, 0.6, 0.95, 0.5, 0.75, 0.9, 0.55, 0.8, 0.65, 1, 0.45, 0.7] as h, i}
        <div class="bar" style="--h: {h}; --delay: {i * 0.08}s;"></div>
      {/each}
    </div>
    <div class="visual-label">LIVE · CAPTIONED · ARCHIVED</div>
    <!-- Big K letterform in background -->
    <div class="hero-k" aria-hidden="true">K</div>
  </div>

</section>

<!-- ── Feature grid ───────────────────────────────────────── -->
<section class="feature-grid">
  <article>
    <span class="feat-num">01</span>
    <h2>Guest-first entry</h2>
    <p>
      Students enter a short code and matric number. No account, no friction.
      Accounts can wait until they want the archive.
    </p>
  </article>

  <article>
    <span class="feat-num">02</span>
    <h2>Attendance with context</h2>
    <p>
      Roster matches verify participation; everyone else is clearly marked for
      review. Fair, auditable, and transparent.
    </p>
  </article>

  <article>
    <span class="feat-num">03</span>
    <h2>Learning that remains</h2>
    <p>
      Captions and key ideas turn one live session into material worth returning
      to — long after the room empties.
    </p>
  </article>
</section>

<style>
  /* ── Hero layout ─────────────────────────────────────────── */
  .hero {
    position: relative;
    min-height: 100vh;
    display: grid;
    grid-template-columns: 1fr 1fr;
    align-items: center;
    padding: calc(var(--nav-height) + 80px) var(--card-padding) 80px;
    overflow: hidden;
    background: radial-gradient(ellipse 80% 60% at 20% 60%, #1c0e05 0%, var(--color-walnut-shadow) 70%);
  }

  /* Entrance animation */
  .hero-content {
    opacity: 0;
    transform: translateY(28px);
    transition: opacity 0.8s cubic-bezier(0.22, 1, 0.36, 1),
                transform 0.8s cubic-bezier(0.22, 1, 0.36, 1);
    z-index: 2;
    position: relative;
  }

  .hero.visible .hero-content {
    opacity: 1;
    transform: translateY(0);
  }

  /* ── Arc decoration ──────────────────────────────────────── */
  .hero-arc {
    position: absolute;
    bottom: -80px;
    left: 50%;
    transform: translateX(-50%);
    width: 120%;
    max-width: 1000px;
    pointer-events: none;
    z-index: 1;
  }

  .hero-arc svg {
    width: 100%;
    height: auto;
  }

  /* ── Text content ─────────────────────────────────────────── */
  .hero-content {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-31);
    max-width: 600px;
  }

  .eyebrow {
    font-size: var(--text-label);
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--color-driftwood);
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .eyebrow-accent {
    color: var(--color-ember-accent);
  }

  .actions {
    display: flex;
    gap: var(--spacing-14);
    align-items: center;
    flex-wrap: wrap;
  }

  /* ── Right visual panel ──────────────────────────────────── */
  .hero-visual {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    min-height: 300px;
    opacity: 0;
    transform: translateY(20px);
    transition: opacity 1s 0.3s cubic-bezier(0.22, 1, 0.36, 1),
                transform 1s 0.3s cubic-bezier(0.22, 1, 0.36, 1);
  }

  .hero.visible .hero-visual {
    opacity: 1;
    transform: translateY(0);
  }

  /* Big K in background */
  .hero-k {
    position: absolute;
    font-family: var(--font-display);
    font-size: clamp(180px, 22vw, 320px);
    font-weight: 500;
    color: transparent;
    -webkit-text-stroke: 1px rgba(255, 237, 215, 0.06);
    letter-spacing: -0.05em;
    line-height: 1;
    user-select: none;
    pointer-events: none;
    z-index: 0;
  }

  /* ── Waveform bars ───────────────────────────────────────── */
  .waveform {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 100px;
    position: relative;
    z-index: 2;
  }

  .bar {
    width: 6px;
    border-radius: 3px;
    background: linear-gradient(to top, var(--color-ember-accent), rgba(220, 80, 0, 0.3));
    height: calc(var(--h) * 100%);
    animation: pulse 1.8s ease-in-out infinite alternate;
    animation-delay: var(--delay);
  }

  @keyframes pulse {
    0%   { transform: scaleY(0.35); opacity: 0.5; }
    100% { transform: scaleY(1);    opacity: 1;   }
  }

  .visual-label {
    margin-top: 28px;
    font-size: 10px;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: var(--color-driftwood);
    position: relative;
    z-index: 2;
  }

  /* ── Feature grid ─────────────────────────────────────────── */
  .feature-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    border-top: 1px solid var(--color-cork-border);
  }

  .feature-grid article {
    padding: var(--spacing-68) var(--card-padding);
    border-right: 1px solid var(--color-cork-border);
    position: relative;
    transition: background 0.3s ease;
  }

  .feature-grid article:last-child {
    border-right: 0;
  }

  .feature-grid article:hover {
    background: rgba(56, 36, 22, 0.3);
  }

  .feat-num {
    display: block;
    font-family: var(--font-display);
    font-size: 11px;
    color: var(--color-ember-accent);
    letter-spacing: 0.2em;
    margin-bottom: var(--spacing-18);
    opacity: 0.8;
  }

  .feature-grid h2 {
    font-size: clamp(22px, 2.5vw, 32px);
    line-height: 1.1;
    margin-bottom: var(--spacing-18);
  }

  .feature-grid p {
    font-size: 15px;
    line-height: 1.65;
    color: var(--color-warm-cream-dim);
  }

  /* ── Divider line before feature number ──────────────────── */
  .feature-grid article::before {
    content: "";
    display: block;
    width: 24px;
    height: 1.5px;
    background: var(--color-ember-accent);
    opacity: 0.5;
    margin-bottom: var(--spacing-24);
  }

  /* ── Responsive ──────────────────────────────────────────── */
  @media (max-width: 900px) {
    .hero {
      grid-template-columns: 1fr;
    }
    .hero-visual {
      display: none;
    }
    .feature-grid {
      grid-template-columns: 1fr;
    }
    .feature-grid article {
      border-right: 0;
      border-bottom: 1px solid var(--color-cork-border);
    }
    .feature-grid article:last-child {
      border-bottom: 0;
    }
  }
</style>
