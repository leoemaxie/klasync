<script lang="ts">
  import type { Screen } from '$lib/types';
  import { registerLecturer } from '$lib/api/auth';
  import PublicVisualPanel from '$lib/components/shared/PublicVisualPanel.svelte';
  import ButtonSpinner from '$lib/components/shared/ButtonSpinner.svelte';
  import type { SessionState } from '$lib/sessionState.svelte';
  import { persist } from '$lib/rosterUtils';

  let {
    screen = $bindable(),
    appState,
  }: {
    screen: Screen;
    appState?: SessionState;
  } = $props();

  let name = $state('');
  let email = $state('');
  let password = $state('');
  let confirmPassword = $state('');
  let errorMsg = $state('');
  let isSubmitting = $state(false);

  async function handleRegister(e: SubmitEvent) {
    e.preventDefault();
    if (password.length < 8) {
      errorMsg = 'Password must be at least 8 characters long.';
      return;
    }
    if (password !== confirmPassword) {
      errorMsg = 'Passwords do not match.';
      return;
    }
    isSubmitting = true;
    errorMsg = '';
    try {
      const res = await registerLecturer({
        name: name.trim(),
        email: email.trim(),
        password,
      });
      if (appState) {
        appState.currentUser = res.user;
        appState.lecturerName = res.user.name;
        appState.lecturerEmail = res.user.email;
        appState.authNotice = '';
        persist(appState);
      }
      screen = 'lecturer';
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Registration failed.';
    } finally {
      isSubmitting = false;
    }
  }
</script>

<svelte:head>
  <title>Create Lecturer Account — Klasync</title>
</svelte:head>

<section class="join-wrap join-wrap-auth">
  <div class="join-left-content">
    <p class="eyebrow">LECTURER ACCOUNT REGISTRATION</p>
    <h1 class="title-single-line">Create Lecturer Account</h1>

    <form class="join-card panel" onsubmit={handleRegister}>
      <label for="reg-name">
        Full Name &amp; Title
        <input
          id="reg-name"
          bind:value={name}
          placeholder="Dr. Amara Okeke"
          required
          autocomplete="name"
        />
      </label>

      <label for="reg-email">
        Institutional Email
        <input
          id="reg-email"
          type="email"
          bind:value={email}
          placeholder="amara.okeke@university.edu"
          required
          autocomplete="email"
        />
      </label>

      <label for="reg-password">
        Password <span>(minimum 8 characters)</span>
        <input
          id="reg-password"
          type="password"
          bind:value={password}
          placeholder="••••••••••••"
          required
          autocomplete="new-password"
        />
      </label>

      <label for="reg-confirm-password">
        Confirm Password
        <input
          id="reg-confirm-password"
          type="password"
          bind:value={confirmPassword}
          placeholder="••••••••••••"
          required
          autocomplete="new-password"
        />
      </label>

      {#if errorMsg}
        <p class="error" role="alert">{errorMsg}</p>
      {/if}

      <button type="submit" class="primary full" disabled={isSubmitting}>
        {#if isSubmitting}
          <ButtonSpinner label="Creating lecturer account..." /> Creating Account...
        {:else}
          Register & Start Courses
        {/if}
      </button>

      <div class="auth-footer-links">
        <a
          href="#/auth/lecturer/login"
          class="text-link"
          onclick={() => (screen = 'lecturer-login')}
        >
          Already registered? Sign in
        </a>
      </div>
    </form>
  </div>

  <PublicVisualPanel
    title="INCLUSIVE UNIVERSITY LECTURES"
    subtitle="Live Captions · Roster Verification · AI Study Summaries"
  />
</section>
