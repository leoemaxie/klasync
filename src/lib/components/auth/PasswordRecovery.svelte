<script lang="ts">
  import type { Screen } from '$lib/types';
  import { requestPasswordReset, completePasswordReset } from '$lib/api/auth';
  import PublicVisualPanel from '$lib/components/shared/PublicVisualPanel.svelte';
  import ButtonSpinner from '$lib/components/shared/ButtonSpinner.svelte';

  let { screen = $bindable() }: { screen: Screen } = $props();
  let mode = $state<'request' | 'complete'>(
    screen === 'reset-password' ? 'complete' : 'request'
  );
  let email = $state(''),
    role = $state<'lecturer' | 'student'>('lecturer');
  let resetToken = $state(''),
    newPassword = $state(''),
    confirmPassword = $state('');
  let statusNotice = $state(''),
    errorMsg = $state(''),
    isSubmitting = $state(false);

  $effect(() => {
    if (typeof window !== 'undefined') {
      const q = new URLSearchParams(
        window.location.search || window.location.hash.split('?')[1]
      );
      const tok = q.get('token');
      if (tok) {
        resetToken = tok;
        mode = 'complete';
      }
    }
  });

  function setMode(m: 'request' | 'complete') {
    mode = m;
    errorMsg = '';
    statusNotice = '';
  }

  async function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    errorMsg = '';
    statusNotice = '';
    if (mode === 'request') {
      if (!email.trim()) return (errorMsg = 'Please enter your email address.');
      isSubmitting = true;
      try {
        await requestPasswordReset(email.trim(), role);
        statusNotice = `If an account exists for ${email.trim()}, a reset link has been sent.`;
      } catch (err) {
        errorMsg =
          err instanceof Error ? err.message : 'Unable to process request.';
      } finally {
        isSubmitting = false;
      }
    } else {
      if (!resetToken.trim())
        return (errorMsg = 'Please enter the reset token.');
      if (newPassword.length < 12)
        return (errorMsg = 'Password must be at least 12 characters.');
      if (newPassword !== confirmPassword)
        return (errorMsg = 'Passwords do not match.');
      isSubmitting = true;
      try {
        await completePasswordReset(resetToken.trim(), newPassword.trim());
        statusNotice = 'Password reset successfully. You may now sign in.';
      } catch (err) {
        errorMsg =
          err instanceof Error ? err.message : 'Unable to reset password.';
      } finally {
        isSubmitting = false;
      }
    }
  }
</script>

<svelte:head>
  <title>Password Recovery — Klasync</title>
</svelte:head>

<section class="join-wrap join-wrap-auth">
  <div class="join-left-content">
    <p class="eyebrow">PASSWORD RESET</p>
    <h1 class="title-single-line">
      {mode === 'request' ? 'Reset Password' : 'New Password'}
    </h1>
    <p class="lede">
      {mode === 'request'
        ? 'Enter your email to receive a reset link.'
        : 'Enter your reset token and choose a new password.'}
    </p>

    <form class="join-card panel" onsubmit={handleSubmit}>
      {#if mode === 'request'}
        <div
          class="role-selector"
          role="group"
          aria-label="Account type selection"
        >
          <button
            type="button"
            class={role === 'lecturer' ? 'primary' : 'outline'}
            aria-pressed={role === 'lecturer'}
            onclick={() => (role = 'lecturer')}>Lecturer</button
          >
          <button
            type="button"
            class={role === 'student' ? 'primary' : 'outline'}
            aria-pressed={role === 'student'}
            onclick={() => (role = 'student')}>Student</button
          >
        </div>
        <label for="rec-email"
          >Email
          <input
            id="rec-email"
            type="email"
            bind:value={email}
            placeholder="name@university.edu"
            required
            autocomplete="email"
          />
        </label>
      {:else}
        <label for="rec-tok"
          >Reset token
          <input
            id="rec-tok"
            bind:value={resetToken}
            placeholder="Paste token from email..."
            required
          />
        </label>
        <label for="rec-pw"
          >New password <span>(min. 12 characters)</span>
          <input
            id="rec-pw"
            type="password"
            bind:value={newPassword}
            placeholder="••••••••••••"
            required
            autocomplete="new-password"
          />
        </label>
        <label for="rec-cpw"
          >Confirm password
          <input
            id="rec-cpw"
            type="password"
            bind:value={confirmPassword}
            placeholder="••••••••••••"
            required
            autocomplete="new-password"
          />
        </label>
      {/if}

      {#if statusNotice}
        <div class="feedback-box" role="status" aria-live="polite">
          <span class="feedback-badge"
            >{mode === 'request' ? 'SENT' : 'UPDATED'}</span
          >
          <p class="feedback-msg">{statusNotice}</p>
        </div>
      {/if}
      {#if errorMsg}
        <p class="error" role="alert">{errorMsg}</p>
      {/if}

      <button
        type="submit"
        class="primary full"
        disabled={isSubmitting || (!!statusNotice && mode === 'complete')}
      >
        {#if isSubmitting}<ButtonSpinner label="Processing..." /> Processing...{:else if mode === 'request'}Send
          Reset Link{:else}Reset Password{/if}
      </button>

      <div class="auth-footer-links">
        <button
          type="button"
          class="text-link"
          onclick={() => setMode(mode === 'request' ? 'complete' : 'request')}
        >
          {mode === 'request'
            ? 'Already have a reset token?'
            : 'Need to request a token?'}
        </button>
        <a
          href={role === 'lecturer'
            ? '#/auth/lecturer/login'
            : '#/auth/student/login'}
          class="text-link"
          onclick={() =>
            (screen = role === 'lecturer' ? 'lecturer-login' : 'student-login')}
        >
          Back to sign in
        </a>
      </div>
    </form>
  </div>

  <PublicVisualPanel
    title="PASSWORD RESET"
    subtitle="Secure access · Account recovery · Verification"
  />
</section>

<style>
  .feedback-box {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 0.85rem;
    background: rgba(210, 232, 35, 0.05);
    border: 1px solid rgba(210, 232, 35, 0.3);
    border-radius: 4px;
    margin-bottom: 0.25rem;
  }
  .feedback-badge {
    font-size: var(--text-label, 0.75rem);
    font-family: var(--font-mono, monospace);
    letter-spacing: 0.08em;
    font-weight: 700;
    color: var(--color-warm-cream, #e4e0d4);
  }
  .feedback-msg {
    font-size: 0.88rem;
    line-height: 1.45;
    color: var(--color-warm-cream, #e4e0d4);
    margin: 0;
  }
  .full {
    width: 100%;
  }
</style>
