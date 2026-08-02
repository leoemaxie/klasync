<script lang="ts">
  import LecturerFormPanel from "./LecturerFormPanel.svelte";
  import LecturerAnalyticsPanel from "./LecturerAnalyticsPanel.svelte";
  import SessionPanel from "$lib/components/session/SessionPanel.svelte";
  import CaptionControlPanel from "$lib/components/session/CaptionControlPanel.svelte";
  import AttendancePanel from "$lib/components/session/AttendancePanel.svelte";
  import type { SessionState } from "$lib/sessionState.svelte";
  import {
    copyInvite,
    endSession,
    importFile,
    parseRoster,
    publishCaption,
    refreshAttendance,
    startSession
  } from "$lib/sessionActions";
  import { ClipboardList, Mic, Users, BarChart3 } from "@lucide/svelte";

  let { appState }: { appState: SessionState } = $props();

  let activeTab = $state<"setup" | "live" | "attendance" | "analytics">("setup");
</script>

<div class="lecturer-workspace-wrap">
  <header class="workspace-header">
    <div class="header-badge-row">
      <span class="eyebrow">LECTURER CONTROL ROOM</span>
      {#if appState.currentUser}
        <span class="auth-status-badge">AUTHENTICATED LECTURER</span>
      {/if}
      {#if appState.session?.live}
        <span class="live-status-pill"><span class="pulse-dot">●</span> SESSION LIVE ({appState.session.code})</span>
      {/if}
    </div>
    <h1>Start a room your students can enter instantly.</h1>
    <p class="lede">
      Configure your lecture roster, stream captions in real time, and monitor student presence verification.
    </p>

    <!-- Quick Navigation Bar for Mobile & Desktop -->
    <nav class="workspace-nav-tabs" aria-label="Lecturer workspace views">
      <button
        type="button"
        class={activeTab === "setup" ? "tab-btn active" : "tab-btn"}
        onclick={() => (activeTab = "setup")}
      >
        <ClipboardList size={14} style="vertical-align: middle; display: inline-block;" /> Course &amp; Roster Setup
      </button>

      {#if appState.session?.live}
        <button
          type="button"
          class={activeTab === "live" ? "tab-btn active" : "tab-btn"}
          onclick={() => (activeTab = "live")}
        >
          <Mic size={14} style="vertical-align: middle; display: inline-block;" /> Live Captions Control
        </button>
      {/if}

      {#if appState.session}
        <button
          type="button"
          class={activeTab === "attendance" ? "tab-btn active" : "tab-btn"}
          onclick={() => (activeTab = "attendance")}
        >
          <Users size={14} style="vertical-align: middle; display: inline-block;" /> Attendance ({appState.session.participants.length})
        </button>
      {/if}

      <button
        type="button"
        class={activeTab === "analytics" ? "tab-btn active" : "tab-btn"}
        onclick={() => (activeTab = "analytics")}
      >
        <BarChart3 size={14} style="vertical-align: middle; display: inline-block;" /> Analytics &amp; Audit
      </button>
    </nav>
  </header>

  <main class="workspace-body">
    {#if activeTab === "setup"}
      <section class="workspace-grid">
        <LecturerFormPanel
          bind:lecturerName={appState.lecturerName}
          bind:lecturerEmail={appState.lecturerEmail}
          bind:courseCode={appState.courseCode}
          bind:courseTitle={appState.courseTitle}
          bind:rosterText={appState.rosterText}
          rosterNotice={appState.rosterNotice}
          onImportFile={(e) => importFile(appState, e)}
          onParseRoster={() => parseRoster(appState)}
        />
        <SessionPanel
          session={appState.session}
          apiNotice={appState.apiNotice}
          isSaving={appState.isSaving}
          copied={appState.copied}
          lecturerName={appState.lecturerName}
          lecturerEmail={appState.lecturerEmail}
          onCopyInvite={() => copyInvite(appState)}
          onEndSession={() => endSession(appState)}
          onStartSession={() => startSession(appState)}
        />
      </section>
    {:else if activeTab === "live"}
      {#if appState.session?.live}
        <CaptionControlPanel
          bind:captionDraft={appState.captionDraft}
          apiNotice={appState.apiNotice}
          onPublishCaption={() => publishCaption(appState)}
        />
      {:else}
        <p class="hint">No live session active. Start a session from the Course Setup tab.</p>
      {/if}
    {:else if activeTab === "attendance"}
      {#if appState.session}
        <AttendancePanel
          sessionCode={appState.session.code}
          participants={appState.session.participants}
          onRefreshAttendance={() => refreshAttendance(appState)}
        />
      {:else}
        <p class="hint">No session created yet. Start a session to view room attendance.</p>
      {/if}
    {:else if activeTab === "analytics"}
      <LecturerAnalyticsPanel
        courseId={appState.courseCode || ""}
        courseCode={appState.courseCode}
        courseTitle={appState.courseTitle}
        participants={appState.session?.participants || []}
        roster={appState.roster}
      />
    {/if}
  </main>
</div>

<style>
  .lecturer-workspace-wrap {
    padding: calc(var(--nav-height) + 32px) var(--card-padding) var(--spacing-68);
    max-width: 1320px;
    margin: 0 auto;
  }

  .workspace-header {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14);
    margin-bottom: var(--spacing-31);
    border-bottom: 1px solid var(--color-cork-border);
    padding-bottom: var(--spacing-24);
  }

  .header-badge-row {
    display: flex;
    align-items: center;
    gap: var(--spacing-12);
    flex-wrap: wrap;
  }

  .auth-status-badge {
    font-size: 9px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #4ab772;
    background: rgba(74, 183, 114, 0.12);
    border: 1px solid #4ab772;
    padding: 3px 8px;
    border-radius: 4px;
    font-weight: 700;
  }

  .live-status-pill {
    font-size: 9px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.12);
    border: 1px solid var(--color-ember-accent);
    padding: 3px 8px;
    border-radius: 4px;
    font-weight: 700;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .pulse-dot {
    animation: blink 1s infinite alternate;
  }

  @keyframes blink {
    0% { opacity: 0.3; }
    100% { opacity: 1; }
  }

  .workspace-nav-tabs {
    display: flex;
    gap: var(--spacing-8);
    margin-top: var(--spacing-14);
    overflow-x: auto;
    padding-bottom: 4px;
  }

  .tab-btn {
    background: rgba(16, 9, 4, 0.5);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-buttons-pill);
    color: var(--color-warm-cream-dim);
    padding: 8px 18px;
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    white-space: nowrap;
    transition: all 0.2s ease;
  }

  .tab-btn:hover {
    border-color: var(--color-warm-cream);
    color: var(--color-warm-cream);
  }

  .tab-btn.active {
    background: var(--color-bark-brown);
    border-color: var(--color-warm-cream);
    color: var(--color-warm-cream);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }

  .workspace-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-24);
    align-items: start;
  }

  @media (max-width: 900px) {
    .workspace-grid {
      grid-template-columns: 1fr;
    }
    .lecturer-workspace-wrap {
      padding: calc(var(--nav-height) + 20px) var(--card-padding) var(--spacing-45);
    }
  }
</style>

