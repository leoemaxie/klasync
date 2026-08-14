<script lang="ts">
  import { onMount } from 'svelte';
  import LecturerHeader from './LecturerHeader.svelte';
  import LecturerNavTabs, { type TabKey } from './LecturerNavTabs.svelte';
  import CourseSetupTab from './CourseSetupTab.svelte';
  import DeviceSetupTab from './DeviceSetupTab.svelte';
  import LecturerAnalyticsPanel from './LecturerAnalyticsPanel.svelte';
  import SessionPanel from '$lib/components/session/SessionPanel.svelte';
  import CaptionControlPanel from '$lib/components/session/CaptionControlPanel.svelte';
  import AttendancePanel from '$lib/components/session/AttendancePanel.svelte';
  import type { SessionState } from '$lib/sessionState.svelte';
  import type { Course } from '$lib/types';
  import { getCourses } from '$lib/api/courses';
  import { connectCaptionWebSocket } from '$lib/api/captions';
  import {
    copyInvite,
    endSession,
    importFile,
    ingestCaption,
    loadCourseRosterFromApi,
    parseRoster,
    publishCaption,
    refreshAttendance,
    saveToCloudRoster,
    startSession,
  } from '$lib/sessionActions';
  import { removeStudentFromRoster, clearRoster } from '$lib/rosterUtils';

  let { appState }: { appState: SessionState } = $props();
  let activeTab = $state<TabKey>('course');

  onMount(async () => {
    try {
      const courseList = await getCourses();
      appState.courses = courseList;
      if (courseList.length > 0) {
        const found =
          courseList.find(
            (c) =>
              c.code.toLowerCase() === appState.courseCode.toLowerCase() &&
              c.academic_session === appState.academicSession
          ) || courseList[0];

        appState.activeCourse = found;
        if (!appState.courseCode) appState.courseCode = found.code;
        if (!appState.courseTitle) appState.courseTitle = found.title;
        if (found.academic_session) appState.academicSession = found.academic_session;
        if (found.semester) appState.semester = found.semester;
      }
    } catch {}

    if (appState.courseCode) {
      void loadCourseRosterFromApi(appState);
    }
  });

  function handleCourseSelected(course: Course) {
    appState.activeCourse = course;
    appState.courseCode = course.code;
    appState.courseTitle = course.title;
    appState.academicSession = course.academic_session;
    appState.semester = course.semester;
    void loadCourseRosterFromApi(appState);
  }

  let wsCleanup: (() => void) | undefined;

  $effect(() => {
    const code = appState.session?.live ? appState.session.code : null;
    if (code) {
      wsCleanup?.();
      wsCleanup = connectCaptionWebSocket(code, (cap) => {
        if (cap.text) {
          ingestCaption(appState, {
            text: cap.text,
            timestamp: cap.created_at,
          });
        }
      });
    } else {
      wsCleanup?.();
      wsCleanup = undefined;
    }

    return () => {
      wsCleanup?.();
    };
  });
</script>

<svelte:head>
  <title>Lecturer Workspace — Klasync</title>
</svelte:head>

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

  <div class="workspace-body">
    {#if activeTab === 'course'}
      <CourseSetupTab
        bind:lecturerName={appState.lecturerName}
        bind:lecturerEmail={appState.lecturerEmail}
        bind:courseCode={appState.courseCode}
        bind:courseTitle={appState.courseTitle}
        bind:academicSession={appState.academicSession}
        bind:semester={appState.semester}
        bind:courses={appState.courses}
        bind:activeCourse={appState.activeCourse}
        bind:rosterText={appState.rosterText}
        roster={appState.roster}
        rosterNotice={appState.rosterNotice}
        onImportFile={(e) => importFile(appState, e)}
        onParseRoster={() => parseRoster(appState)}
        onSaveToCloud={() => saveToCloudRoster(appState)}
        onRemoveStudent={(matric) => removeStudentFromRoster(appState, matric)}
        onClearRoster={() => clearRoster(appState)}
        onReloadFromCloud={() => loadCourseRosterFromApi(appState)}
        onCourseSelected={handleCourseSelected}
      />
    {:else if activeTab === 'device'}
      <DeviceSetupTab
        sessionCode={appState.session?.code}
        onCaptionIngested={(cap) => ingestCaption(appState, cap)}
      />
    {:else if activeTab === 'session'}
      <SessionPanel
        session={appState.session}
        apiNotice={appState.apiNotice}
        isSaving={appState.isSaving}
        isEndingSession={appState.isSaving}
        copied={appState.copied}
        lecturerName={appState.lecturerName}
        lecturerEmail={appState.lecturerEmail}
        onCopyInvite={() => copyInvite(appState)}
        onEndSession={() => endSession(appState)}
        onStartSession={() => startSession(appState)}
        onCaptionIngested={(cap) => ingestCaption(appState, cap)}
      />
    {:else if activeTab === 'live' && appState.session?.live}
      <CaptionControlPanel
        bind:captionDraft={appState.captionDraft}
        captions={appState.captions}
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
        courseId={appState.courseId || appState.activeCourse?.id || ''}
        courseCode={appState.courseCode}
        courseTitle={appState.courseTitle}
        sessionId={appState.session?.id || ''}
        sessionCode={appState.session?.code || ''}
        participants={appState.session?.participants || []}
        roster={appState.roster}
      />
    {/if}
  </div>
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
