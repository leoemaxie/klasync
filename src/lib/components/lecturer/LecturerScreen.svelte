<script lang="ts">
  import LecturerHeader from './LecturerHeader.svelte';
  import LecturerNavTabs, { type TabKey } from './LecturerNavTabs.svelte';
  import CourseSetupTab from './CourseSetupTab.svelte';
  import DeviceSetupTab from './DeviceSetupTab.svelte';
  import LecturerAnalyticsPanel from './LecturerAnalyticsPanel.svelte';
  import SessionPanel from '$lib/components/session/SessionPanel.svelte';
  import CaptionControlPanel from '$lib/components/session/CaptionControlPanel.svelte';
  import AttendancePanel from '$lib/components/session/AttendancePanel.svelte';
  import type { SessionState } from '$lib/sessionState.svelte';
  import {
    copyInvite,
    endSession,
    importFile,
    parseRoster,
    publishCaption,
    refreshAttendance,
    startSession,
  } from '$lib/sessionActions';

  let { appState }: { appState: SessionState } = $props();
  let activeTab = $state<TabKey>('course');
</script>

<div class="lecturer-workspace-wrap">
  <LecturerHeader
    currentUser={appState.currentUser}
    liveCode={appState.session?.live ? appState.session.code : undefined}
  />

  <LecturerNavTabs
    bind:activeTab
    isLive={!!appState.session?.live}
    hasSession={!!appState.session}
    participantCount={appState.session?.participants.length || 0}
  />

  <main class="workspace-body">
    {#if activeTab === 'course'}
      <CourseSetupTab
        bind:lecturerName={appState.lecturerName}
        bind:lecturerEmail={appState.lecturerEmail}
        bind:courseCode={appState.courseCode}
        bind:courseTitle={appState.courseTitle}
        bind:rosterText={appState.rosterText}
        rosterNotice={appState.rosterNotice}
        onImportFile={(e) => importFile(appState, e)}
        onParseRoster={() => parseRoster(appState)}
      />
    {:else if activeTab === 'device'}
      <DeviceSetupTab />
    {:else if activeTab === 'session'}
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
    {:else if activeTab === 'live' && appState.session?.live}
      <CaptionControlPanel
        bind:captionDraft={appState.captionDraft}
        apiNotice={appState.apiNotice}
        onPublishCaption={() => publishCaption(appState)}
      />
    {:else if activeTab === 'attendance' && appState.session}
      <AttendancePanel
        sessionCode={appState.session.code}
        participants={appState.session.participants}
        onRefreshAttendance={() => refreshAttendance(appState)}
      />
    {:else if activeTab === 'analytics'}
      <LecturerAnalyticsPanel
        courseId={appState.courseCode || ''}
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
    padding: calc(var(--nav-height) + 24px) var(--card-padding)
      var(--spacing-68);
    max-width: 1320px;
    margin: 0 auto;
  }
  @media (max-width: 640px) {
    .lecturer-workspace-wrap {
      padding-bottom: 90px;
    }
  }
</style>
