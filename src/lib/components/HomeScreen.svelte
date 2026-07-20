<script lang="ts">
  import type { Screen } from "$lib/types";

  let { screen = $bindable() }: { screen: Screen } = $props();
</script>

<section class="hero">
  <p class="eyebrow">ACCESSIBLE LEARNING / BUILT FOR THE ROOM</p>
  <h1>Every lecture,<br /><em>within reach.</em></h1>
  <p class="lede">
    Live captions, fair attendance, and a lasting learning archive—without
    making students create an account just to join class.
  </p>
  <div class="actions">
    <button class="primary" onclick={() => (screen = "join")}>
      Join a live lecture
    </button>
    <button class="outline" onclick={() => (screen = "lecturer")}>
      Create a lecturer workspace
    </button>
  </div>
</section>

<section class="feature-grid">
  <article>
    <b>01</b>
    <h2>Guest-first entry</h2>
    <p>
      Students enter a short code and matric number. Accounts can wait until
      they want the archive.
    </p>
  </article>
  <article>
    <b>02</b>
    <h2>Attendance with context</h2>
    <p>
      Roster matches verify participation; everyone else is clearly marked for
      review.
    </p>
  </article>
  <article>
    <b>03</b>
    <h2>Learning that remains</h2>
    <p>
      Captions and key ideas turn one live session into material worth returning
      to.
    </p>
  </article>
</section>
