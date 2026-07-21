<script lang="ts">
  import type { Screen } from "$lib/types";

  let { screen = $bindable() }: { screen: Screen } = $props();
</script>

<footer class="landing-footer">
  <div class="footer-content">
    <p class="footer-privacy">
      <span>KLASYNC Guest Access &amp; Privacy Notice: No account is required to join a live lecture. Guest participation records matric numbers for lecturer roster verification.</span>
    </p>
    <div class="footer-links">
      <button class="text-link" onclick={() => (screen = "join")}>Join Session</button>
      <button class="text-link" onclick={() => (screen = "lecturer")}>Lecturer Access</button>
    </div>
  </div>
</footer>
