<script lang="ts">
  import { onMount } from "svelte";
  import Navbar from "$lib/components/shared/Navbar.svelte";
  import NotFoundScreen from "$lib/components/shared/NotFoundScreen.svelte";
  import HomeScreen from "$lib/components/home/HomeScreen.svelte";
  import LecturerFormPanel from "$lib/components/lecturer/LecturerFormPanel.svelte";
  import SessionPanel from "$lib/components/session/SessionPanel.svelte";
  import CaptionControlPanel from "$lib/components/session/CaptionControlPanel.svelte";
  import AttendancePanel from "$lib/components/session/AttendancePanel.svelte";
  import JoinScreen from "$lib/components/session/JoinScreen.svelte";
  import LiveScreen from "$lib/components/session/LiveScreen.svelte";
  import ArchiveScreen from "$lib/components/archive/ArchiveScreen.svelte";
  import LecturerSignIn from "$lib/components/auth/LecturerSignIn.svelte";
  import LecturerRegister from "$lib/components/auth/LecturerRegister.svelte";
  import StudentSignIn from "$lib/components/auth/StudentSignIn.svelte";
  import StudentRegister from "$lib/components/auth/StudentRegister.svelte";
  import PasswordRecovery from "$lib/components/auth/PasswordRecovery.svelte";
  import { createSessionState } from "$lib/sessionState.svelte";
  import {
    copyInvite, endSession, heartbeat, importFile, joinSession, parseRoster,
    publishCaption, refreshAttendance, refreshCaptions, startSession
  } from "$lib/sessionActions";

  const state = createSessionState();

  onMount(() => {
    const saved = localStorage.getItem("klasync-session");
    const savedRoster = localStorage.getItem("klasync-roster");
    const savedLecturer = localStorage.getItem("klasync-lecturer");
    if (saved) state.session = JSON.parse(saved);
    if (savedRoster) state.roster = JSON.parse(savedRoster);
    if (savedLecturer) state.lecturerName = savedLecturer;

    const inviteCode = new URLSearchParams(location.search).get("join");
    if (inviteCode) {
      state.sessionCode = inviteCode.toUpperCase();
      state.screen = "join";
    }

    const captionTimer = window.setInterval(() => {
      if (state.screen === "live" && state.session?.live) void refreshCaptions(state);
    }, 3000);

    return () => window.clearInterval(captionTimer);
  });
</script>

<main>
  <Navbar bind:screen={state.screen} />

  {#if state.screen === "home"}
    <HomeScreen bind:screen={state.screen} />
  {:else if state.screen === "lecturer-login"}
    <LecturerSignIn bind:screen={state.screen} />
  {:else if state.screen === "lecturer-register"}
    <LecturerRegister bind:screen={state.screen} />
  {:else if state.screen === "student-login"}
    <StudentSignIn bind:screen={state.screen} />
  {:else if state.screen === "student-register"}
    <StudentRegister bind:screen={state.screen} />
  {:else if state.screen === "recover-password" || state.screen === "reset-password"}
    <PasswordRecovery bind:screen={state.screen} />
  {:else if state.screen === "lecturer"}
    <section class="page-head">
      <p class="eyebrow">LECTURER WORKSPACE</p>
      <h1>Start a room your students can enter instantly.</h1>
      <p>Only lecturers need an account. Roster enables immediate attendance verification.</p>
    </section>
    <section class="workspace">
      <LecturerFormPanel
        bind:lecturerName={state.lecturerName} bind:lecturerEmail={state.lecturerEmail}
        bind:courseCode={state.courseCode} bind:courseTitle={state.courseTitle}
        bind:rosterText={state.rosterText} rosterNotice={state.rosterNotice}
        onImportFile={(e) => importFile(state, e)} onParseRoster={() => parseRoster(state)}
      />
      <SessionPanel
        session={state.session} apiNotice={state.apiNotice} isSaving={state.isSaving}
        copied={state.copied} lecturerName={state.lecturerName} lecturerEmail={state.lecturerEmail}
        onCopyInvite={() => copyInvite(state)} onEndSession={() => endSession(state)}
        onStartSession={() => startSession(state)}
      />
    </section>
    {#if state.session?.live}
      <CaptionControlPanel
        bind:captionDraft={state.captionDraft} apiNotice={state.apiNotice}
        onPublishCaption={() => publishCaption(state)}
      />
    {/if}
    {#if state.session}
      <AttendancePanel
        sessionCode={state.session.code}
        participants={state.session.participants}
        onRefreshAttendance={() => refreshAttendance(state)}
      />
    {/if}
  {:else if state.screen === "join"}
    <JoinScreen
      bind:sessionCode={state.sessionCode} bind:matric={state.matric}
      bind:displayName={state.displayName} joinError={state.joinError}
      onJoinSession={() => joinSession(state)}
    />
  {:else if state.screen === "live"}
    <LiveScreen
      session={state.session} joinedParticipant={state.joinedParticipant}
      captions={state.captions} captionIndex={state.captionIndex}
      bind:accountCreated={state.accountCreated} bind:screen={state.screen}
      onNextCaption={() => refreshCaptions(state)} onHeartbeat={() => heartbeat(state)}
    />
  {:else if state.screen === "archive"}
    <ArchiveScreen bind:screen={state.screen} />
  {:else}
    <NotFoundScreen bind:screen={state.screen} />
  {/if}
</main>
