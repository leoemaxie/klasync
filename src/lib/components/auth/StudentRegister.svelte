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

<section class="join-wrap">
  <div class="join-left-content">
    <p class="eyebrow">STUDENT ACCOUNT / PERSISTENT ARCHIVE</p>
    <h1>Retain your learning.</h1>
    <p class="lede">
      An account unlocks your claimed lecture materials. Joining live lectures
      remains account-free.
    </p>

    <form class="join-card panel" onsubmit={handleRegister}>
      <label>
        Matric / Student ID
        <input bind:value={matric} placeholder="MAT/2023/001" required />
      </label>

      <label>
        Full Name
        <input bind:value={name} placeholder="Ada Okafor" required />
      </label>

      <label>
        Email Address
        <input
          type="email"
          bind:value={email}
          placeholder="ada@student.edu"
          required
        />
      </label>

      <label>
        Password <span>(minimum 8 characters)</span>
        <input
          type="password"
          bind:value={password}
          placeholder="••••••••••••"
          required
        />
      </label>

      <label>
        Confirm Password
        <input
          type="password"
          bind:value={confirmPassword}
          placeholder="••••••••••••"
          required
        />
      </label>

      {#if errorMsg}
        <p class="error">{errorMsg}</p>
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
          href="#/student-login"
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
