<script lang="ts">
  import type { Screen } from "$lib/types";

  let { screen = $bindable() }: { screen: Screen } = $props();
</script>

<nav>
  <button class="brand" onclick={() => (screen = "home")}>
    KLA<span>SYNC</span>
  </button>
  <div class="nav-actions">
    <button class="text" onclick={() => (screen = "join")}>
      Join a session
    </button>
    <button class="outline" onclick={() => (screen = "lecturer")}>
      Lecturer workspace
    </button>
  </div>
</nav>
