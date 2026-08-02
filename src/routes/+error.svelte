<script lang="ts">
  import { page } from '$app/stores';
  import Navbar from '$lib/components/shared/Navbar.svelte';
  import type { Screen } from '$lib/types';

  let screen = $state<Screen>('not-found');
</script>

<main>
  <Navbar bind:screen />

  <section class="join-wrap not-found-wrap">
    <div class="not-found-badge">
      <span class="eyebrow-accent">●</span>
      {$page.status ?? 404} / ERROR
    </div>

    <h1 class="code-404-title">{$page.status ?? 404}</h1>

    <h2 class="not-found-heading">
      {$page.error?.message || 'Room not found.'}
    </h2>

    <p class="lede">
      The lecture code, invite link, or page resource you requested is
      unavailable or has expired.
    </p>

    <div class="actions not-found-actions">
      <a href="/" class="primary">Return Home</a>
      <a href="/join" class="outline">Join Session</a>
      <a href="/lecturer/sign-in" class="primary-bark">Lecturer Access</a>
    </div>
  </section>
</main>
