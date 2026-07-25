<script lang="ts">
  import type { Screen } from "$lib/types";
  import { loginLecturer } from "$lib/api/auth";
  import PublicVisualPanel from "$lib/components/shared/PublicVisualPanel.svelte";

  let { screen = $bindable() }: { screen: Screen } = $props();

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
      await loginLecturer(email.trim(), password);
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
        {isSubmitting ? "Authenticating..." : "Sign In to Workspace"}
      </button>

      <div class="auth-footer-links">
        <button type="button" class="text-link" onclick={() => (screen = "lecturer-register")}>
          Need an account? Register
        </button>
        <button type="button" class="text-link" onclick={() => (screen = "recover-password")}>
          Forgot password?
        </button>
      </div>
    </form>
  </div>

  <PublicVisualPanel title="LECTURER CONTROL ROOM" subtitle="Wireless Mic Streaming · Live Roster Verification · Caption Publishing" />
</section>
