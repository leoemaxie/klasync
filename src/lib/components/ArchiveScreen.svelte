<script lang="ts">
  import type { Screen } from "$lib/types";

  let { screen = $bindable() }: { screen: Screen } = $props();
</script>

<section class="join-wrap">
  <p class="eyebrow">STUDENT ARCHIVE</p>
  <h1>Your learning should outlast the room.</h1>
  <div class="join-card">
    <p>
      This prototype records your intent to create an account. The production
      phase will add verified student accounts and secure persistent access to
      lecture resources.
    </p>
    <button class="primary full" onclick={() => (screen = "home")}>
      Return home
    </button>
  </div>
</section>
