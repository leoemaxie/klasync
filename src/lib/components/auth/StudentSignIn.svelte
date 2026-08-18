<script lang="ts">
  import type { Screen } from '$lib/types';
  import { loginStudent, logout } from '$lib/api/auth';
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
      errorMsg = 'Please enter your email and password.';
      return;
    }
    isSubmitting = true;
    errorMsg = '';
    try {
      const res = await loginStudent(email.trim(), password);
      if (res.user && res.user.role !== 'student') {
        await logout().catch(() => {});
        errorMsg =
          'This account is registered as a lecturer. Please sign in as a lecturer.';
        return;
      }
      if (appState) {
        appState.currentUser = res.user;
        appState.authNotice = '';
        persist(appState);
      }
      screen = 'archive';
    } catch (err) {
      errorMsg =
        err instanceof Error ? err.message : 'Invalid email or password.';
    } finally {
      isSubmitting = false;
    }
  }
</script>

<svelte:head>
  <title>Student Sign In — Klasync</title>
</svelte:head>

<section class="join-wrap join-wrap-auth">
  <div class="join-left-content">
    <p class="eyebrow">STUDENT ACCESS</p>
    <h1 class="title-single-line">Student Sign In</h1>

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

      <label for="student-signin-email">
        Email
        <input
          id="student-signin-email"
          type="email"
          bind:value={email}
          placeholder="student@university.edu"
          required
          autocomplete="email"
        />
      </label>

      <label for="student-signin-password">
        Password
        <input
          id="student-signin-password"
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
          <ButtonSpinner label="Signing in..." /> Signing in...
        {:else}
          Sign In
        {/if}
      </button>

      <div class="auth-footer-links">
        <a
          href="#/auth/student/register"
          class="text-link"
          onclick={() => (screen = 'student-register')}
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
    title="STUDY ARCHIVE"
    subtitle="Live transcripts · Flashcards · Study notes"
  />
</section>
