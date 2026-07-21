<script lang="ts">
  import type { Screen } from "$lib/types";

  let { screen = $bindable() }: { screen: Screen } = $props();
</script>

<section class="join-wrap not-found-wrap">
  <div class="not-found-badge" aria-hidden="true">
    <span class="eyebrow-accent">●</span> 404 / ROOM UNRESOLVED
  </div>

  <h1 class="code-404-title">404</h1>

  <h2 class="not-found-heading">Room not found.</h2>

  <p class="lede">
    The lecture code, invite link, or resource you are looking for is unavailable, expired, or does not exist.
  </p>

  <div class="actions not-found-actions">
    <button class="primary" onclick={() => (screen = "home")}>
      Return Home
    </button>
    <button class="outline" onclick={() => (screen = "join")}>
      Join Live Session
    </button>
    <button class="primary-bark" onclick={() => (screen = "lecturer-login")}>
      Lecturer Access
    </button>
  </div>
</section>
