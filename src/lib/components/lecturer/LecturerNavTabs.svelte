<script lang="ts">
  import {
    BookOpen,
    Mic,
    Radio,
    Captions,
    Users,
    BarChart3,
  } from '@lucide/svelte';

  export type TabKey =
    'course' | 'device' | 'session' | 'live' | 'attendance' | 'analytics';

  let {
    activeTab = $bindable('course'),
    isLive = false,
    hasSession = false,
    participantCount = 0,
  }: {
    activeTab: TabKey;
    isLive?: boolean;
    hasSession?: boolean;
    participantCount?: number;
  } = $props();

  const tabs: {
    key: TabKey;
    label: string;
    icon: typeof BookOpen;
    badge?: string;
  }[] = $derived([
    { key: 'course', label: 'Course', icon: BookOpen },
    { key: 'device', label: 'Device', icon: Mic },
    {
      key: 'session',
      label: 'Room',
      icon: Radio,
      badge: isLive ? 'LIVE' : undefined,
    },
    ...(isLive
      ? [{ key: 'live' as TabKey, label: 'Captions', icon: Captions }]
      : []),
    ...(hasSession
      ? [
          {
            key: 'attendance' as TabKey,
            label: 'Attendance',
            icon: Users,
            badge: String(participantCount),
          },
        ]
      : []),
    { key: 'analytics', label: 'Analytics', icon: BarChart3 },
  ]);
  import { triggerHaptic } from '$lib/native/haptics';

  function selectTab(key: TabKey) {
    triggerHaptic('light');
    activeTab = key;
  }
</script>

<nav class="mobile-native-tabs" aria-label="Lecturer navigation">
  <div class="tabs-scroll-track">
    {#each tabs as tab}
      {@const Icon = tab.icon}
      <button
        type="button"
        class="tab-pill"
        class:active={activeTab === tab.key}
        aria-pressed={activeTab === tab.key}
        onclick={() => selectTab(tab.key)}
      >
        <div class="tab-icon-wrapper">
          <Icon size={16} class="tab-icon" aria-hidden="true" />
          {#if tab.badge}
            <span
              class="tab-badge mobile-badge"
              class:live-badge={tab.badge === 'LIVE'}
            >
              {tab.badge}
            </span>
          {/if}
        </div>
        <span class="tab-label">{tab.label}</span>
        {#if tab.badge}
          <span
            class="tab-badge desktop-badge"
            class:live-badge={tab.badge === 'LIVE'}
          >
            {tab.badge}
          </span>
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
    -webkit-backdrop-filter: blur(12px);
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
  .tabs-scroll-track::-webkit-scrollbar {
    display: none;
  }
  .tab-icon-wrapper {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    position: relative;
  }
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
  .tab-pill:hover {
    color: var(--color-warm-cream);
  }
  .tab-pill:active {
    transform: scale(0.96);
  }
  .tab-pill.active {
    background: var(--color-warm-cream);
    border-color: var(--color-warm-cream);
    color: var(--color-walnut-shadow);
    font-weight: 700;
    box-shadow:
      0 2px 12px rgba(255, 237, 215, 0.3),
      inset 0 1px 0 rgba(255, 255, 255, 0.4);
  }
  :global(.tab-pill.active .tab-icon) {
    color: var(--color-walnut-shadow);
  }
  .tab-badge {
    font-size: 9px;
    font-weight: 700;
    padding: 1px 6px;
    border-radius: 9999px;
    background: rgba(255, 255, 255, 0.1);
    color: var(--color-warm-cream);
  }
  .tab-pill.active .desktop-badge {
    background: rgba(16, 9, 4, 0.15);
    color: var(--color-walnut-shadow);
  }
  .tab-badge.live-badge {
    background: var(--color-ember-accent);
    color: #fff;
    animation: blink 1.2s infinite alternate;
  }
  .mobile-badge {
    display: none;
  }
  .desktop-badge {
    display: inline-flex;
  }

  @keyframes blink {
    0% {
      opacity: 0.5;
    }
    100% {
      opacity: 1;
    }
  }

  @media (max-width: 640px) {
    .mobile-native-tabs {
      position: fixed;
      bottom: calc(10px + var(--safe-bottom, env(safe-area-inset-bottom, 0px)));
      top: auto;
      left: 10px;
      right: 10px;
      margin-bottom: 0;
      border-radius: 18px;
      padding: 5px;
      background: rgba(18, 11, 5, 0.92);
      backdrop-filter: blur(16px);
      -webkit-backdrop-filter: blur(16px);
      border: 1px solid var(--color-cork-border);
      box-shadow:
        0 10px 30px rgba(0, 0, 0, 0.75),
        inset 0 1px 0 rgba(255, 237, 215, 0.08);
      z-index: 90;
    }
    .tabs-scroll-track {
      justify-content: space-around;
      gap: 2px;
      padding: 0;
      width: 100%;
    }
    .tab-pill {
      flex: 1 1 0%;
      min-width: 0;
      justify-content: center;
      padding: 7px 2px 5px;
      font-size: 9px;
      flex-direction: column;
      gap: 3px;
      border-radius: 12px;
    }
    .tab-pill.active {
      background: var(--color-bark-brown);
      border-color: var(--color-cork-border);
      color: var(--color-warm-cream);
      box-shadow:
        0 2px 8px rgba(0, 0, 0, 0.5),
        inset 0 1px 0 rgba(255, 237, 215, 0.12);
    }
    :global(.tab-pill.active .tab-icon) {
      color: var(--color-warm-cream);
    }
    .tab-label {
      font-size: 9px;
      letter-spacing: 0.02em;
      max-width: 100%;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .desktop-badge {
      display: none;
    }
    .mobile-badge {
      display: inline-flex;
      position: absolute;
      top: -5px;
      right: -9px;
      font-size: 7.5px;
      font-weight: 700;
      line-height: 1;
      padding: 1px 4px;
      min-width: 13px;
      height: 13px;
      align-items: center;
      justify-content: center;
      border-radius: 9999px;
      background: var(--color-bark-brown);
      color: var(--color-warm-cream);
      border: 1px solid var(--color-cork-border);
      box-shadow: 0 2px 4px rgba(0, 0, 0, 0.5);
      pointer-events: none;
    }
    .mobile-badge.live-badge {
      background: var(--color-ember-accent);
      color: #ffffff;
      border: none;
      box-shadow: 0 0 6px rgba(220, 80, 0, 0.6);
    }
  }
</style>
