<script lang="ts">
  import type { Participant, Screen, Session } from "$lib/types";

  let {
    session,
    joinedParticipant,
    captions = [],
    captionIndex = 0,
    accountCreated = $bindable(false),
    screen = $bindable(),
    onNextCaption,
    onHeartbeat,
  }: {
    session: Session | null;
    joinedParticipant: Participant | null;
    captions: string[];
    captionIndex: number;
    accountCreated: boolean;
    screen: Screen;
    onNextCaption: () => void;
    onHeartbeat: () => void;
  } = $props();

  function createAccount() {
    accountCreated = true;
    screen = "archive";
  }
</script>

<section class="live-top">
  <div>
    <p class="eyebrow">LIVE / CAPTIONS CONNECTED</p>
    <h1>{session?.title}</h1>
    <p>
      {joinedParticipant?.verified
        ? "Attendance verified against your class roster."
        : "Attendance is provisional; your lecturer can review it."}
    </p>
  </div>
  <button class="outline" onclick={() => (screen = "lecturer")}>
    Lecturer view
  </button>
</section>

<section class="live-grid">
  <article class="captions">
    <p class="eyebrow">LIVE CAPTIONS</p>
    <p class="caption">{captions[captionIndex] ?? "WAITING FOR CAPTIONS"}</p>
    <button class="outline" onclick={onNextCaption}>
      Simulate next caption
    </button>
  </article>
  <aside>
    <p class="eyebrow">KEY IDEA</p>
    <h2>Feedback makes a system responsive.</h2>
    <p>
      Capture the signal, compare it with the desired outcome, and use the
      difference to decide the next action.
    </p>
    <hr />
    <p class="hint">
      Presence check-ins: {joinedParticipant?.heartbeats ?? 0}
    </p>
    <button class="text" onclick={onHeartbeat}>I’m still here</button>
  </aside>
</section>

<section class="archive-cta">
  <div>
    <p class="eyebrow">AFTER CLASS</p>
    <h2>Keep this lecture.</h2>
    <p>
      Create a student account after the session to revisit recordings,
      transcripts, flashcards, and notes.
    </p>
  </div>
  {#if accountCreated}
    <p class="success">
      Account interest saved. In the production backend this will start account
      verification.
    </p>
  {:else}
    <button class="primary" onclick={createAccount}>
      Create account for the archive
    </button>
  {/if}
</section>
