<script lang="ts">
  import type { Screen } from "$lib/types";
  import { registerLecturer } from "$lib/api/auth";
  import PublicVisualPanel from "$lib/components/shared/PublicVisualPanel.svelte";

  let { screen = $bindable() }: { screen: Screen } = $props();

  let name = $state("");
  let email = $state("");
  let password = $state("");
  let confirmPassword = $state("");
  let errorMsg = $state("");
  let isSubmitting = $state(false);

  async function handleRegister(e: SubmitEvent) {
    e.preventDefault();
    if (password.length < 8) {
      errorMsg = "Password must be at least 8 characters long.";
      return;
    }
    if (password !== confirmPassword) {
      errorMsg = "Passwords do not match.";
      return;
    }
    isSubmitting = true;
    errorMsg = "";
    try {
      await registerLecturer({ name: name.trim(), email: email.trim(), password });
      screen = "lecturer";
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : "Registration failed.";
    } finally {
      isSubmitting = false;
    }
  }
</script>

<section class="join-wrap">
  <div class="join-left-content">
    <p class="eyebrow">LECTURER WORKSPACE / ACCOUNT REGISTRATION</p>
    <h1>Create your workspace.</h1>
    <p class="lede">
      Enable physical lecture accessibility for all your university courses in seconds.
    </p>

    <form class="join-card panel" onsubmit={handleRegister}>
      <label>
        Full Name &amp; Title
        <input bind:value={name} placeholder="Dr. Amara Okeke" required />
      </label>

      <label>
        Institutional Email
        <input type="email" bind:value={email} placeholder="amara.okeke@university.edu" required />
      </label>

      <label>
        Password <span>(minimum 8 characters)</span>
        <input type="password" bind:value={password} placeholder="••••••••••••" required />
      </label>

      <label>
        Confirm Password
        <input type="password" bind:value={confirmPassword} placeholder="••••••••••••" required />
      </label>

      {#if errorMsg}
        <p class="error">{errorMsg}</p>
      {/if}

      <button type="submit" class="primary full" disabled={isSubmitting}>
        {isSubmitting ? "Creating Account..." : "Register & Start Courses"}
      </button>

      <div class="auth-footer-links">
        <button type="button" class="text-link" onclick={() => (screen = "lecturer-login")}>
          Already registered? Sign in
        </button>
      </div>
    </form>
  </div>

  <PublicVisualPanel title="INCLUSIVE UNIVERSITY LECTURES" subtitle="1-Click Session Codes · Roster Import · Accessible Higher Ed" />
</section>
