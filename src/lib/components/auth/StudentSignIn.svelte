<script lang="ts">
  import type { Screen } from "$lib/types";
  import { loginStudent } from "$lib/api/auth";
  import PublicVisualPanel from "../PublicVisualPanel.svelte";

  let { screen = $bindable() }: { screen: Screen } = $props();

  let email = $state("");
  let password = $state("");
  let errorMsg = $state("");
  let isSubmitting = $state(false);

  async function handleLogin(e: SubmitEvent) {
    e.preventDefault();
    if (!email.trim() || !password) {
      errorMsg = "Please enter your email and password.";
      return;
    }
    isSubmitting = true;
    errorMsg = "";
    try {
      await loginStudent(email.trim(), password);
      screen = "archive";
    } catch (err) {
      errorMsg = err instanceof Error ? err.message : "Invalid authentication credentials.";
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
      Sign in to access your claimed lecture transcripts, flashcards, AI summaries, and revision notes.
    </p>

    <form class="join-card panel" onsubmit={handleLogin}>
      <label>
        Student Email
        <input type="email" bind:value={email} placeholder="student@university.edu" required />
      </label>

      <label>
        Password
        <input type="password" bind:value={password} placeholder="••••••••••••" required />
      </label>

      {#if errorMsg}
        <p class="error">{errorMsg}</p>
      {/if}

      <button type="submit" class="primary full" disabled={isSubmitting}>
        {isSubmitting ? "Authenticating..." : "Sign In to Archive"}
      </button>

      <div class="auth-footer-links">
        <button type="button" class="text-link" onclick={() => (screen = "student-register")}>
          Create an account for persistent access
        </button>
        <button type="button" class="text-link" onclick={() => (screen = "recover-password")}>
          Forgot password?
        </button>
      </div>
    </form>
  </div>

  <PublicVisualPanel title="SEARCHABLE LEARNING ARCHIVE" subtitle="Transcripts · Summaries · Flashcards · Revision Notes" />
</section>
