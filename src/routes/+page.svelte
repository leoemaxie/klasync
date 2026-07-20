<script lang="ts">
  import { onMount } from "svelte";
  import Navbar from "$lib/components/Navbar.svelte";
  import HomeScreen from "$lib/components/HomeScreen.svelte";
  import LecturerFormPanel from "$lib/components/LecturerFormPanel.svelte";
  import SessionPanel from "$lib/components/SessionPanel.svelte";
  import CaptionControlPanel from "$lib/components/CaptionControlPanel.svelte";
  import AttendancePanel from "$lib/components/AttendancePanel.svelte";
  import JoinScreen from "$lib/components/JoinScreen.svelte";
  import LiveScreen from "$lib/components/LiveScreen.svelte";
  import ArchiveScreen from "$lib/components/ArchiveScreen.svelte";
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
  {:else}
    <ArchiveScreen bind:screen={state.screen} />
  {/if}
</main>
