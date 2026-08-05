<script lang="ts">
  import type { Screen } from '$lib/types';
  import { registerStudent } from '$lib/api/auth';
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

  let matric = $state('');
  let name = $state('');
  let email = $state('');
  let password = $state('');
  let confirmPassword = $state('');
  let errorMsg = $state('');
  let isSubmitting = $state(false);

  async function handleRegister(e: SubmitEvent) {
    e.preventDefault();
    if (!matric.trim()) {
      errorMsg = 'Please enter your matric or student ID.';
      return;
    }
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
      const res = await registerStudent({
        matric_number: matric.trim(),
        name: name.trim(),
        email: email.trim(),
        password,
      });
      if (appState) {
        appState.currentUser = res.user;
        appState.authNotice = '';
        persist(appState);
      }
      screen = 'archive';
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : 'Registration failed.';
    } finally {
      isSubmitting = false;
    }
  }
</script>

<svelte:head>
  <title>Create Student Account — Klasync</title>
</svelte:head>

<section class="join-wrap join-wrap-auth">
  <div class="join-left-content">
    <p class="eyebrow">STUDENT ACCOUNT REGISTRATION</p>
    <h1 class="title-single-line">Create Student Account</h1>

    <form class="join-card panel" onsubmit={handleRegister}>
      <label for="st-matric">
        Matric / Student ID
        <input id="st-matric" bind:value={matric} placeholder="MAT/2023/001" required />
      </label>

      <label for="st-name">
        Full Name
        <input id="st-name" bind:value={name} placeholder="Ada Okafor" required autocomplete="name" />
      </label>

      <label for="st-email">
        Email Address
        <input
          id="st-email"
          type="email"
          bind:value={email}
          placeholder="ada@student.edu"
          required
          autocomplete="email"
        />
      </label>

      <label for="st-password">
        Password <span>(minimum 8 characters)</span>
        <input
          id="st-password"
          type="password"
          bind:value={password}
          placeholder="••••••••••••"
          required
          autocomplete="new-password"
        />
      </label>

      <label for="st-confirm-password">
        Confirm Password
        <input
          id="st-confirm-password"
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
          <ButtonSpinner label="Creating student account..." /> Creating Account...
        {:else}
          Create Account & Retain Materials
        {/if}
      </button>

      <div class="auth-footer-links">
        <a
          href="#/auth/student/login"
          class="text-link"
          onclick={() => (screen = 'student-login')}
        >
          Already have an account? Sign in
        </a>
      </div>
    </form>
  </div>

  <PublicVisualPanel
    title="STUDENT LEARNING PERSISTENCE"
    subtitle="Claimed Lectures · Audio Replay · AI Notes"
  />
</section>

<style>
  :global(.join-wrap-auth) {
    align-items: flex-start !important;
  }

  .join-left-content {
    gap: 10px;
  }

  .join-card.panel {
    padding: 26px 28px;
  }

  .eyebrow {
    display: none;
  }

  .title-single-line {
    display: none;
  }

  .auth-footer-links {
    display: flex;
    flex-direction: row;
    justify-content: space-between;
    align-items: center;
    gap: var(--spacing-12);
    margin-top: 14px;
    margin-bottom: 0;
    width: 100%;
  }

  @media (max-width: 900px) {
    :global(.join-wrap-auth) {
      align-items: stretch !important;
    }

    .join-left-content {
      gap: 8px;
    }

    .join-card.panel {
      padding: var(--spacing-31);
    }

    .eyebrow {
      display: block;
    }

    .title-single-line {
      display: block;
      white-space: normal;
      margin: 0 0 6px 0;
    }

    .auth-footer-links {
      flex-direction: column;
      align-items: flex-start;
      gap: var(--spacing-12);
      margin-top: var(--spacing-20);
    }
  }
</style>
