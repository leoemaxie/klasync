<script lang="ts">
  import type { Screen } from '$lib/types';
  import { loginLecturer, logout } from '$lib/api/auth';
  import PublicVisualPanel from '$lib/components/shared/PublicVisualPanel.svelte';
  import ButtonSpinner from '$lib/components/shared/ButtonSpinner.svelte';
  import type { SessionState } from '$lib/sessionState.svelte';
  import { persist } from '$lib/rosterUtils';

  import { Lock } from '@lucide/svelte';

  let {
    screen = $bindable(),
    appState,
  }: {
    screen: Screen;
    appState?: SessionState;
  } = $props();

  let email = $state('');
  let password = $state('');
  let errorMsg = $state('');
  let isSubmitting = $state(false);

  async function handleLogin(e: SubmitEvent) {
    e.preventDefault();
    if (!email.trim() || !password) {
      errorMsg = 'Please enter both institutional email and password.';
      return;
    }
    isSubmitting = true;
    errorMsg = '';
    try {
      const res = await loginLecturer(email.trim(), password);
      if (res.user && res.user.role !== 'lecturer' && res.user.role !== 'admin') {
        await logout().catch(() => {});
        errorMsg = 'This account is registered as a student. Please sign in under Student Sign In.';
        return;
      }
      if (appState) {
        appState.currentUser = res.user;
        appState.lecturerName = res.user.name;
        appState.lecturerEmail = res.user.email;
        appState.authNotice = '';
        persist(appState);
      }
      screen = 'lecturer';
    } catch (err) {
      errorMsg =
        err instanceof Error
          ? err.message
          : 'Invalid authentication credentials.';
    } finally {
      isSubmitting = false;
    }
  }
</script>

<svelte:head>
  <title>Lecturer Sign In — Klasync</title>
</svelte:head>

<section class="join-wrap">
  <div class="join-left-content">
    <p class="eyebrow">LECTURER ACCESS</p>
    <h1>Lecturer Sign In</h1>

    <form class="join-card panel" onsubmit={handleLogin}>
      {#if appState?.authNotice}
        <p
          role="alert"
          class="error"
          style="border: 1px solid var(--color-error, #f0a030); padding: 8px 12px; border-radius: 4px; background: rgba(240, 160, 48, 0.08);"
        >
          <Lock
            size={14}
            aria-hidden="true"
            style="vertical-align: middle; display: inline-block;"
          />
          {appState.authNotice}
        </p>
      {/if}

      <label for="signin-email">
        Institutional Email
        <input
          id="signin-email"
          type="email"
          bind:value={email}
          placeholder="dr.okeke@university.edu"
          required
          autocomplete="email"
        />
      </label>

      <label for="signin-password">
        Password
        <input
          id="signin-password"
          type="password"
          bind:value={password}
          placeholder="••••••••••••"
          required
          autocomplete="current-password"
        />
      </label>

      {#if errorMsg}
        <p class="error" role="alert">{errorMsg}</p>
      {/if}

      <button type="submit" class="primary full" disabled={isSubmitting}>
        {#if isSubmitting}
          <ButtonSpinner label="Authenticating..." /> Authenticating...
        {:else}
          Sign In
        {/if}
      </button>

      <div class="auth-footer-links">
        <a
          href="#/auth/lecturer/register"
          class="text-link"
          onclick={() => (screen = 'lecturer-register')}
        >
          Create account
        </a>
        <a
          href="#/auth/recover-password"
          class="text-link"
          onclick={() => (screen = 'recover-password')}
        >
          Forgot password?
        </a>
      </div>
    </form>
  </div>

  <PublicVisualPanel
    title="LECTURER CONTROL ROOM"
    subtitle="Wireless Mic Streaming · Live Roster Verification · Caption Publishing"
  />
</section>
