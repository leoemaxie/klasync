<script lang="ts">
  import type { Screen } from '$lib/types';
  import { loginStudent } from '$lib/api/auth';
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
      if (appState) {
        appState.currentUser = res.user;
        appState.authNotice = '';
        persist(appState);
      }
      screen = 'archive';
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

<section class="join-wrap">
  <div class="join-left-content">
    <p class="eyebrow">STUDENT PERSISTENT ACCESS / SIGN IN</p>
    <h1>Access your archive.</h1>
    <p class="lede">
      Sign in to access your claimed lecture transcripts, flashcards, AI
      summaries, and revision notes.
    </p>

    <form class="join-card panel" onsubmit={handleLogin}>
      {#if appState?.authNotice}
        <p
          class="error"
          style="border: 1px solid var(--color-ember-accent); padding: 8px 12px; border-radius: 4px; background: rgba(220, 80, 0, 0.1);"
        >
          <Lock
            size={14}
            style="vertical-align: middle; display: inline-block;"
          />
          {appState.authNotice}
        </p>
      {/if}

      <label>
        Student Email
        <input
          type="email"
          bind:value={email}
          placeholder="student@university.edu"
          required
        />
      </label>

      <label>
        Password
        <input
          type="password"
          bind:value={password}
          placeholder="••••••••••••"
          required
        />
      </label>

      {#if errorMsg}
        <p class="error">{errorMsg}</p>
      {/if}

      <button type="submit" class="primary full" disabled={isSubmitting}>
        {#if isSubmitting}
          <ButtonSpinner label="Authenticating student credentials..." /> Authenticating...
        {:else}
          Sign In to Archive
        {/if}
      </button>

      <div class="auth-footer-links">
        <a
          href="#/student-register"
          class="text-link"
          onclick={() => (screen = 'student-register')}
        >
          Create an account for persistent access
        </a>
        <a
          href="#/recover-password"
          class="text-link"
          onclick={() => (screen = 'recover-password')}
        >
          Forgot password?
        </a>
      </div>
    </form>
  </div>

  <PublicVisualPanel
    title="SEARCHABLE LEARNING ARCHIVE"
    subtitle="Transcripts · Summaries · Flashcards · Revision Notes"
  />
</section>
