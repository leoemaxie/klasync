<script lang="ts">
  import type { Screen } from "$lib/types";
  import { loginLecturer } from "$lib/api/auth";
  import PublicVisualPanel from "$lib/components/shared/PublicVisualPanel.svelte";
  import ButtonSpinner from "$lib/components/shared/ButtonSpinner.svelte";
  import type { SessionState } from "$lib/sessionState.svelte";
  import { persist } from "$lib/rosterUtils";

  import { Lock } from "@lucide/svelte";

  let {
    screen = $bindable(),
    appState
  }: {
    screen: Screen;
    appState?: SessionState;
  } = $props();

  let email = $state("");
  let password = $state("");
  let errorMsg = $state("");
  let isSubmitting = $state(false);

  async function handleLogin(e: SubmitEvent) {
    e.preventDefault();
    if (!email.trim() || !password) {
      errorMsg = "Please enter both institutional email and password.";
      return;
    }
    isSubmitting = true;
    errorMsg = "";
    try {
      const res = await loginLecturer(email.trim(), password);
      if (appState) {
        appState.currentUser = res.user;
        appState.lecturerName = res.user.name;
        appState.lecturerEmail = res.user.email;
        appState.authNotice = "";
        persist(appState);
      }
      screen = "lecturer";
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : "Invalid authentication credentials.";
    } finally {
      isSubmitting = false;
    }
  }
</script>


<section class="join-wrap">
  <div class="join-left-content">
    <p class="eyebrow">LECTURER ACCESS / SIGN IN</p>
    <h1>Welcome back.</h1>
    <p class="lede">
      Sign in to launch live lecture rooms, stream captions, and manage attendance rosters.
    </p>

    <form class="join-card panel" onsubmit={handleLogin}>
      {#if appState?.authNotice}
        <p class="error" style="border: 1px solid var(--color-ember-accent); padding: 8px 12px; border-radius: 4px; background: rgba(220, 80, 0, 0.1);">
          <Lock size={14} style="vertical-align: middle; display: inline-block;" /> {appState.authNotice}
        </p>
      {/if}

      <label>
        Institutional Email
        <input type="email" bind:value={email} placeholder="dr.okeke@university.edu" required />
      </label>

      <label>
        Password
        <input type="password" bind:value={password} placeholder="••••••••••••" required />
      </label>

      {#if errorMsg}
        <p class="error">{errorMsg}</p>
      {/if}

      <button type="submit" class="primary full" disabled={isSubmitting}>
        {#if isSubmitting}
          <ButtonSpinner label="Authenticating credentials..." /> Authenticating...
        {:else}
          Sign In to Workspace
        {/if}
      </button>

      <div class="auth-footer-links">
        <a href="#/lecturer-register" class="text-link" onclick={() => (screen = "lecturer-register")}>
          Need an account? Register
        </a>
        <a href="#/recover-password" class="text-link" onclick={() => (screen = "recover-password")}>
          Forgot password?
        </a>
      </div>
    </form>
  </div>

  <PublicVisualPanel title="LECTURER CONTROL ROOM" subtitle="Wireless Mic Streaming · Live Roster Verification · Caption Publishing" />
</section>
