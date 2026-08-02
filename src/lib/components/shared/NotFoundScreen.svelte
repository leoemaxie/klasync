<script lang="ts">
  import type { Screen } from '$lib/types';

  let { screen = $bindable() }: { screen: Screen } = $props();
</script>

<section class="join-wrap not-found-wrap">
  <div class="not-found-badge" aria-hidden="true">
    <span class="eyebrow-accent">●</span> 404 / ROOM UNRESOLVED
  </div>

  <h1 class="code-404-title">404</h1>

  <h2 class="not-found-heading">Room not found.</h2>

  <p class="lede">
    The session code or link is invalid or expired.
  </p>

  <div class="actions not-found-actions">
    <a href="#/" class="primary" onclick={() => (screen = 'home')}>
      Return Home
    </a>
    <a href="#/join" class="outline" onclick={() => (screen = 'join')}>
      Join Live Session
    </a>
    <a
      href="#/lecturer-login"
      class="primary-bark"
      onclick={() => (screen = 'lecturer-login')}
    >
      Lecturer Access
    </a>
  </div>
</section>
