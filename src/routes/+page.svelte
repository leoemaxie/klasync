<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { get } from 'svelte/store';
  import Navbar from '$lib/components/shared/Navbar.svelte';
  import NotFoundScreen from '$lib/components/shared/NotFoundScreen.svelte';
  import OfflineStatusIndicator from '$lib/components/shared/OfflineStatusIndicator.svelte';
  import SpotlightSearchModal from '$lib/components/shared/SpotlightSearchModal.svelte';
  import HomeScreen from '$lib/components/home/HomeScreen.svelte';
  import LecturerScreen from '$lib/components/lecturer/LecturerScreen.svelte';
  import JoinScreen from '$lib/components/session/JoinScreen.svelte';
  import LiveScreen from '$lib/components/session/LiveScreen.svelte';
  import ArchiveScreen from '$lib/components/archive/ArchiveScreen.svelte';
  import LecturerSignIn from '$lib/components/auth/LecturerSignIn.svelte';
  import LecturerRegister from '$lib/components/auth/LecturerRegister.svelte';
  import StudentSignIn from '$lib/components/auth/StudentSignIn.svelte';
  import StudentRegister from '$lib/components/auth/StudentRegister.svelte';
  import PasswordRecovery from '$lib/components/auth/PasswordRecovery.svelte';
  import { createSessionState } from '$lib/sessionState.svelte';
  import { heartbeat, joinSession, refreshCaptions } from '$lib/sessionActions';
  import { location, push, replace } from 'svelte-spa-router';
  import { SCREEN_TO_PATH, screenFromPath } from '$lib/router';
  import { enforceAuthGuard } from '$lib/authGuard';

  const appState = createSessionState();
  let isSpotlightOpen = $state(false);

  onMount(() => {
    const savedSession = localStorage.getItem('klasync-session');
    const savedRoster = localStorage.getItem('klasync-roster');
    const savedRosterText = localStorage.getItem('klasync-rosterText');
    const savedLecturer = localStorage.getItem('klasync-lecturer');
    const savedUser = localStorage.getItem('klasync-user');
    const savedCourseCode = localStorage.getItem('klasync-courseCode');
    const savedCourseTitle = localStorage.getItem('klasync-courseTitle');

    if (savedSession) {
      try {
        appState.session = JSON.parse(savedSession);
      } catch {}
    }
    if (savedRoster) {
      try {
        appState.roster = JSON.parse(savedRoster);
      } catch {}
    }
    if (savedRosterText) {
      appState.rosterText = savedRosterText;
    } else if (appState.roster.length > 0) {
      appState.rosterText = appState.roster
        .map((s) => `${s.matric}, ${s.name}`)
        .join('\n');
    }
    if (savedLecturer) appState.lecturerName = savedLecturer;
    if (savedCourseCode) appState.courseCode = savedCourseCode;
    if (savedCourseTitle) appState.courseTitle = savedCourseTitle;

    if (savedUser) {
      try {
        const user = JSON.parse(savedUser);
        appState.currentUser = user;
        if (user.name) appState.lecturerName = user.name;
        if (user.email) appState.lecturerEmail = user.email;
      } catch {
        appState.currentUser = null;
      }
    }

    const searchJoin = new URLSearchParams(window.location.search).get('join');
    const hashJoin =
      new URLSearchParams(window.location.hash.split('?')[1] || '').get(
        'join'
      ) ||
      new URLSearchParams(window.location.hash.split('?')[1] || '').get('code');
    const inviteCode = searchJoin || hashJoin;
    if (inviteCode) {
      appState.sessionCode = inviteCode.toUpperCase();
      appState.screen = 'join';
      void replace('/join');
    }

    const unsubLoc = location.subscribe(($loc) => {
      const currentLoc = $loc || '/';
      const matchedScreen = enforceAuthGuard(currentLoc, appState);
      if (appState.screen !== matchedScreen) {
        appState.screen = matchedScreen;
      }
    });

    const captionTimer = window.setInterval(() => {
      if (appState.screen === 'live' && appState.session?.live)
        void refreshCaptions(appState);
    }, 3000);

    return () => {
      unsubLoc();
      window.clearInterval(captionTimer);
    };
  });

  // Synchronize screen state changes to router location when updated via component props
  $effect(() => {
    const targetPath = SCREEN_TO_PATH[appState.screen];
    const currentLoc = untrack(() => get(location) || '/');

    if (targetPath && currentLoc !== targetPath) {
      void push(targetPath);
    }
  });
</script>

<main>
  <Navbar
    bind:screen={appState.screen}
    {appState}
    onOpenSpotlight={() => (isSpotlightOpen = true)}
  />
  <OfflineStatusIndicator />

  {#if appState.screen === 'home'}
    <HomeScreen bind:screen={appState.screen} />
  {:else if appState.screen === 'lecturer-login'}
    <LecturerSignIn bind:screen={appState.screen} {appState} />
  {:else if appState.screen === 'lecturer-register'}
    <LecturerRegister bind:screen={appState.screen} {appState} />
  {:else if appState.screen === 'student-login'}
    <StudentSignIn bind:screen={appState.screen} {appState} />
  {:else if appState.screen === 'student-register'}
    <StudentRegister bind:screen={appState.screen} {appState} />
  {:else if appState.screen === 'recover-password' || appState.screen === 'reset-password'}
    <PasswordRecovery bind:screen={appState.screen} />
  {:else if appState.screen === 'lecturer'}
    <LecturerScreen {appState} />
  {:else if appState.screen === 'join'}
    <JoinScreen
      bind:sessionCode={appState.sessionCode}
      bind:matric={appState.matric}
      bind:displayName={appState.displayName}
      joinError={appState.joinError}
      onJoinSession={() => joinSession(appState)}
    />
  {:else if appState.screen === 'live'}
    <LiveScreen
      session={appState.session}
      joinedParticipant={appState.joinedParticipant}
      captions={appState.captions}
      captionIndex={appState.captionIndex}
      bind:accountCreated={appState.accountCreated}
      bind:screen={appState.screen}
      currentUser={appState.currentUser}
      onNextCaption={() => refreshCaptions(appState)}
      onHeartbeat={() => heartbeat(appState)}
    />
  {:else if appState.screen === 'archive'}
    <ArchiveScreen bind:screen={appState.screen} />
  {:else}
    <NotFoundScreen bind:screen={appState.screen} />
  {/if}

  <SpotlightSearchModal
    bind:isOpen={isSpotlightOpen}
    bind:screen={appState.screen}
    bind:sessionCode={appState.sessionCode}
    currentUser={appState.currentUser}
  />
</main>
