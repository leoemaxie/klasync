<script lang="ts">
  import type { Screen } from "$lib/types";
  import { requestPasswordReset } from "$lib/api/auth";
  import PublicVisualPanel from "../PublicVisualPanel.svelte";

  let { screen = $bindable() }: { screen: Screen } = $props();

  let email = $state("");
  let role = $state<"lecturer" | "student">("lecturer");
  let statusNotice = $state("");
  let errorMsg = $state("");
  let isSubmitting = $state(false);

  async function handleRequestReset(e: SubmitEvent) {
    e.preventDefault();
    if (!email.trim()) return;
    isSubmitting = true;
    errorMsg = "";
    try {
      await requestPasswordReset(email.trim(), role);
      statusNotice = "If an account exists for this email, password recovery instructions have been sent.";
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : "Unable to process recovery request.";
    } finally {
      isSubmitting = false;
    }
  }
</script>

<section class="join-wrap">
  <div class="join-left-content">
    <p class="eyebrow">ACCOUNT RECOVERY / PASSWORD RESET</p>
    <h1>Recover account access.</h1>
    <p class="lede">
      Enter your registered email address to receive password reset instructions.
    </p>

    <form class="join-card panel" onsubmit={handleRequestReset}>
      <div class="role-selector">
        <button
          type="button"
          class={role === "lecturer" ? "primary" : "outline"}
          onclick={() => (role = "lecturer")}
        >
          Lecturer
        </button>
        <button
          type="button"
          class={role === "student" ? "primary" : "outline"}
          onclick={() => (role = "student")}
        >
          Student
        </button>
      </div>

      <label>
        Registered Email
        <input type="email" bind:value={email} placeholder="your.name@university.edu" required />
      </label>

      {#if statusNotice}
        <p class="success">{statusNotice}</p>
      {/if}
      {#if errorMsg}
        <p class="error">{errorMsg}</p>
      {/if}

      <button type="submit" class="primary full" disabled={isSubmitting || !!statusNotice}>
        {isSubmitting ? "Sending Request..." : "Send Reset Link"}
      </button>

      <div class="auth-footer-links">
        <button type="button" class="text-link" onclick={() => (screen = role === "lecturer" ? "lecturer-login" : "student-login")}>
          Back to sign in
        </button>
      </div>
    </form>
  </div>

  <PublicVisualPanel title="SECURE AUTHENTICATION" subtitle="Encrypted Identity · Verification Claims · Password Reset" />
</section>
