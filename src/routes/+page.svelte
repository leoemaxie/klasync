<script lang="ts">
  import { onMount } from "svelte";
  import Navbar from "$lib/components/shared/Navbar.svelte";
  import NotFoundScreen from "$lib/components/shared/NotFoundScreen.svelte";
  import OfflineStatusIndicator from "$lib/components/shared/OfflineStatusIndicator.svelte";
  import HomeScreen from "$lib/components/home/HomeScreen.svelte";
  import LecturerScreen from "$lib/components/lecturer/LecturerScreen.svelte";
  import JoinScreen from "$lib/components/session/JoinScreen.svelte";
  import LiveScreen from "$lib/components/session/LiveScreen.svelte";
  import ArchiveScreen from "$lib/components/archive/ArchiveScreen.svelte";
  import LecturerSignIn from "$lib/components/auth/LecturerSignIn.svelte";
  import LecturerRegister from "$lib/components/auth/LecturerRegister.svelte";
  import StudentSignIn from "$lib/components/auth/StudentSignIn.svelte";
  import StudentRegister from "$lib/components/auth/StudentRegister.svelte";
  import PasswordRecovery from "$lib/components/auth/PasswordRecovery.svelte";
  import { createSessionState } from "$lib/sessionState.svelte";
  import { heartbeat, joinSession, refreshCaptions } from "$lib/sessionActions";
  import { location, push } from "svelte-spa-router";
  import { SCREEN_TO_PATH, screenFromPath } from "$lib/router";
  import { getAccessToken, setAccessToken } from "$lib/api/http";

  const state = createSessionState();

  onMount(() => {
    const savedSession = localStorage.getItem("klasync-session");
    const savedRoster = localStorage.getItem("klasync-roster");
    const savedLecturer = localStorage.getItem("klasync-lecturer");
    const savedUser = localStorage.getItem("klasync-user");

    if (savedSession) state.session = JSON.parse(savedSession);
    if (savedRoster) state.roster = JSON.parse(savedRoster);
    if (savedLecturer) state.lecturerName = savedLecturer;

    if (savedUser) {
      try {
        const user = JSON.parse(savedUser);
        state.currentUser = user;
        if (user.name) state.lecturerName = user.name;
        if (user.email) state.lecturerEmail = user.email;
      } catch {
        state.currentUser = null;
      }
    }

    const searchJoin = new URLSearchParams(window.location.search).get("join");
    const hashJoin =
      new URLSearchParams(window.location.hash.split("?")[1] || "").get("join") ||
      new URLSearchParams(window.location.hash.split("?")[1] || "").get("code");
    const inviteCode = searchJoin || hashJoin;

    if (inviteCode) {
      state.sessionCode = inviteCode.toUpperCase();
      state.screen = "join";
      void push("/join");
    } else {
      const initialPath = $location || "/";
      const initialScreen = screenFromPath(initialPath);
      
      // Guard initial screen
      if (initialScreen === "lecturer" && (!state.currentUser || (state.currentUser.role !== "lecturer" && state.currentUser.role !== "admin"))) {
        state.authNotice = "Please sign in to access the Lecturer Workspace.";
        state.screen = "lecturer-login";
        void push("/lecturer-login");
      } else if (initialScreen === "archive" && !state.currentUser) {
        state.authNotice = "Please sign in to access your Student Archive.";
        state.screen = "student-login";
        void push("/student-login");
      } else {
        state.screen = initialScreen;
        const initialTargetPath = SCREEN_TO_PATH[initialScreen];
        if (initialTargetPath && initialPath !== initialTargetPath) {
          void push(initialTargetPath);
        }
      }
    }

    const captionTimer = window.setInterval(() => {
      if (state.screen === "live" && state.session?.live) void refreshCaptions(state);
    }, 3000);

    return () => window.clearInterval(captionTimer);
  });

  $effect(() => {
    const matchedScreen = screenFromPath($location);

    // Auth guard for lecturer workspace
    if (matchedScreen === "lecturer" && (!state.currentUser || (state.currentUser.role !== "lecturer" && state.currentUser.role !== "admin"))) {
      state.authNotice = "Please sign in to access the Lecturer Workspace.";
      state.screen = "lecturer-login";
      void push("/lecturer-login");
      return;
    }

    // Auth guard for student archive
    if (matchedScreen === "archive" && !state.currentUser) {
      state.authNotice = "Please sign in to access your Student Archive.";
      state.screen = "student-login";
      void push("/student-login");
      return;
    }

    if (state.screen !== matchedScreen) {
      state.screen = matchedScreen;
    }
  });

  $effect(() => {
    // Auth guard on state.screen changes
    if (state.screen === "lecturer" && (!state.currentUser || (state.currentUser.role !== "lecturer" && state.currentUser.role !== "admin"))) {
      state.authNotice = "Please sign in to access the Lecturer Workspace.";
      state.screen = "lecturer-login";
      void push("/lecturer-login");
      return;
    }

    if (state.screen === "archive" && !state.currentUser) {
      state.authNotice = "Please sign in to access your Student Archive.";
      state.screen = "student-login";
      void push("/student-login");
      return;
    }

    const targetPath = SCREEN_TO_PATH[state.screen];
    if (targetPath && $location !== targetPath) {
      void push(targetPath);
    }
  });
</script>

<main>
  <Navbar bind:screen={state.screen} {state} />
  <OfflineStatusIndicator />

  {#if state.screen === "home"}
    <HomeScreen bind:screen={state.screen} />
  {:else if state.screen === "lecturer-login"}
    <LecturerSignIn bind:screen={state.screen} {state} />
  {:else if state.screen === "lecturer-register"}
    <LecturerRegister bind:screen={state.screen} {state} />
  {:else if state.screen === "student-login"}
    <StudentSignIn bind:screen={state.screen} {state} />
  {:else if state.screen === "student-register"}
    <StudentRegister bind:screen={state.screen} {state} />
  {:else if state.screen === "recover-password" || state.screen === "reset-password"}
    <PasswordRecovery bind:screen={state.screen} />
  {:else if state.screen === "lecturer"}
    <LecturerScreen {state} />
  {:else if state.screen === "join"}
    <JoinScreen
      bind:sessionCode={state.sessionCode}
      bind:matric={state.matric}
      bind:displayName={state.displayName}
      joinError={state.joinError}
      onJoinSession={() => joinSession(state)}
    />
  {:else if state.screen === "live"}
    <LiveScreen
      session={state.session}
      joinedParticipant={state.joinedParticipant}
      captions={state.captions}
      captionIndex={state.captionIndex}
      bind:accountCreated={state.accountCreated}
      bind:screen={state.screen}
      onNextCaption={() => refreshCaptions(state)}
      onHeartbeat={() => heartbeat(state)}
    />
  {:else if state.screen === "archive"}
    <ArchiveScreen bind:screen={state.screen} />
  {:else}
    <NotFoundScreen bind:screen={state.screen} />
  {/if}
</main>

