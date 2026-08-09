<script lang="ts">
  import type { Screen } from '$lib/types';
  import { requestPasswordReset, completePasswordReset } from '$lib/api/auth';
  import PublicVisualPanel from '$lib/components/shared/PublicVisualPanel.svelte';
  import ButtonSpinner from '$lib/components/shared/ButtonSpinner.svelte';

  let { screen = $bindable() }: { screen: Screen } = $props();

  let mode = $state<'request' | 'complete'>('request');
  let email = $state('');
  let role = $state<'lecturer' | 'student'>('lecturer');
  let resetToken = $state('');
  let newPassword = $state('');
  let statusNotice = $state('');
  let errorMsg = $state('');
  let isSubmitting = $state(false);

  async function handleRequestReset(e: SubmitEvent) {
    e.preventDefault();
    if (mode === 'request') {
      if (!email.trim()) return;
      isSubmitting = true;
      errorMsg = '';
      try {
        await requestPasswordReset(email.trim(), role);
        statusNotice =
          'If an account exists for this email, password recovery instructions have been sent.';
      } catch (err) {
        errorMsg =
          err instanceof Error
            ? err.message
            : 'Unable to process recovery request.';
      } finally {
        isSubmitting = false;
      }
    } else {
      if (!resetToken.trim() || !newPassword.trim()) return;
      isSubmitting = true;
      errorMsg = '';
      try {
        await completePasswordReset(resetToken.trim(), newPassword.trim());
        statusNotice =
          'Password reset successfully. You may now sign in with your new password.';
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
    <p class="eyebrow">ACCOUNT RECOVERY / PASSWORD RESET</p>
    <h1 class="title-single-line">
      {mode === 'request'
        ? 'Recover account access.'
        : 'Complete password reset.'}
    </h1>
    <p class="lede">
      {mode === 'request'
        ? 'Enter your registered email address to receive password reset instructions.'
        : 'Enter your reset token and new password.'}
    </p>

    <form class="join-card panel" onsubmit={handleRequestReset}>
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
            onclick={() => (role = 'lecturer')}
          >
            Lecturer
          </button>
          <button
            type="button"
            class={role === 'student' ? 'primary' : 'outline'}
            aria-pressed={role === 'student'}
            onclick={() => (role = 'student')}
          >
            Student
          </button>
        </div>

        <label for="recover-email">
          Registered Email
          <input
            id="recover-email"
            type="email"
            bind:value={email}
            placeholder="your.name@university.edu"
            required
            autocomplete="email"
          />
        </label>
      {:else}
        <label for="reset-token">
          Reset Token
          <input
            id="reset-token"
            bind:value={resetToken}
            placeholder="Paste reset token..."
            required
          />
        </label>
        <label for="new-password">
          New Password
          <input
            id="new-password"
            type="password"
            bind:value={newPassword}
            placeholder="••••••••"
            required
            autocomplete="new-password"
          />
        </label>
      {/if}

      {#if statusNotice}
        <p class="success" role="status">{statusNotice}</p>
      {/if}
      {#if errorMsg}
        <p class="error" role="alert">{errorMsg}</p>
      {/if}

      <button
        type="submit"
        class="primary full"
        disabled={isSubmitting || !!statusNotice}
      >
        {#if isSubmitting}
          <ButtonSpinner label="Processing password recovery..." /> Processing...
        {:else if mode === 'request'}
          Send Reset Link
        {:else}
          Complete Password Reset
        {/if}
      </button>

      <div class="auth-footer-links">
        <button
          type="button"
          class="text-link"
          onclick={() => (mode = mode === 'request' ? 'complete' : 'request')}
        >
          {mode === 'request'
            ? 'Have a reset token?'
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
    title="SECURE AUTHENTICATION"
    subtitle="Encrypted Identity · Verification Claims · Password Reset"
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

  .title-single-line,
  .lede {
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

    .title-single-line,
    .lede {
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
