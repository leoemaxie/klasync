<script lang="ts">
  import { BookOpen, Mic, Radio, Captions, Users, BarChart3 } from "@lucide/svelte";

  export type TabKey = "course" | "device" | "session" | "live" | "attendance" | "analytics";

  let {
    activeTab = $bindable("course"),
    isLive = false,
    hasSession = false,
    participantCount = 0
  }: {
    activeTab: TabKey;
    isLive?: boolean;
    hasSession?: boolean;
    participantCount?: number;
  } = $props();

  const tabs: { key: TabKey; label: string; icon: typeof BookOpen; badge?: string }[] = $derived([
    { key: "course", label: "Course", icon: BookOpen },
    { key: "device", label: "Device", icon: Mic },
    { key: "session", label: "Room", icon: Radio, badge: isLive ? "LIVE" : undefined },
    ...(isLive ? [{ key: "live" as TabKey, label: "Captions", icon: Captions }] : []),
    ...(hasSession ? [{ key: "attendance" as TabKey, label: "Attendance", icon: Users, badge: String(participantCount) }] : []),
    { key: "analytics", label: "Analytics", icon: BarChart3 }
  ]);
</script>

<nav class="mobile-native-tabs" aria-label="Lecturer control room navigation">
  <div class="tabs-scroll-track">
    {#each tabs as tab}
      <button
        type="button"
        class="tab-pill"
        class:active={activeTab === tab.key}
        onclick={() => (activeTab = tab.key)}
      >
        <svelte:component this={tab.icon} size={15} class="tab-icon" />
        <span class="tab-label">{tab.label}</span>
        {#if tab.badge}
          <span class="tab-badge" class:live-badge={tab.badge === "LIVE"}>{tab.badge}</span>
        {/if}
      </button>
    {/each}
  </div>
</nav>

<style>
  .mobile-native-tabs {
    position: sticky;
    top: calc(var(--nav-height) + 8px);
    z-index: 40;
    margin-bottom: var(--spacing-24);
    background: rgba(16, 9, 4, 0.85);
    backdrop-filter: blur(12px);
    border: 1px solid var(--color-cork-border);
    border-radius: 9999px;
    padding: 4px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.4);
  }
  .tabs-scroll-track {
    display: flex;
    align-items: center;
    gap: 4px;
    overflow-x: auto;
    scrollbar-width: none;
    -ms-overflow-style: none;
    -webkit-overflow-scrolling: touch;
    padding: 2px;
  }
  .tabs-scroll-track::-webkit-scrollbar { display: none; }
  .tab-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    border-radius: 9999px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--color-driftwood);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    white-space: nowrap;
    cursor: pointer;
    flex-shrink: 0;
    touch-action: manipulation;
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
    -webkit-tap-highlight-color: transparent;
  }
  .tab-pill:hover { color: var(--color-warm-cream); }
  .tab-pill:active { transform: scale(0.96); }
  .tab-pill.active {
    background: var(--color-bark-brown);
    border-color: var(--color-warm-cream-dim);
    color: var(--color-warm-cream);
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.5), inset 0 1px 0 rgba(255, 255, 255, 0.1);
  }
  .tab-badge {
    font-size: 9px;
    font-weight: 700;
    padding: 1px 6px;
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.1);
    color: var(--color-warm-cream);
  }
  .tab-badge.live-badge {
    background: var(--color-ember-accent);
    color: #fff;
    animation: blink 1.2s infinite alternate;
  }
  @keyframes blink { 0% { opacity: 0.5; } 100% { opacity: 1; } }

  @media (max-width: 640px) {
    .mobile-native-tabs {
      position: fixed;
      bottom: 12px;
      top: auto;
      left: 12px;
      right: 12px;
      margin-bottom: 0;
      border-radius: 20px;
      padding: 6px;
      box-shadow: 0 10px 30px rgba(0, 0, 0, 0.7);
    }
    .tab-pill {
      flex: 1;
      justify-content: center;
      padding: 10px 10px;
      font-size: 10px;
      flex-direction: column;
      gap: 3px;
    }
    .tab-label { font-size: 9px; }
  }
</style>
