<script lang="ts">
  import { onMount } from "svelte";
  import type { Participant, Screen, Session } from "$lib/types";
  import { connectCaptionWebSocket } from "$lib/api/captions";

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

  let wsConnected = $state(false);

  onMount(() => {
    if (!session?.code) return;
    const cleanup = connectCaptionWebSocket(session.code, (cap) => {
      wsConnected = true;
      if (!captions.includes(cap.text)) {
        captions = [...captions, cap.text];
        captionIndex = captions.length - 1;
      }
    });
    return cleanup;
  });

  function createAccount() {
    accountCreated = true;
    screen = "archive";
  }
</script>

<section class="live-top">
  <div>
    <p class="eyebrow">
      <span class="eyebrow-accent">●</span> LIVE / {wsConnected ? "WEBSOCKET STREAM" : "CAPTIONS CONNECTED"}
    </p>
    <h1>{session?.title ?? "Live Lecture"}</h1>
    <p class="lede">
      {joinedParticipant?.verified
        ? "Verified student presence on active roster."
        : "Provisional attendance recorded. Lecturer review pending."}
    </p>
  </div>
  <button class="outline" onclick={() => (screen = "lecturer")}>
    Lecturer View
  </button>
</section>

<section class="live-grid">
  <article class="captions panel" aria-live="polite">
    <p class="eyebrow">REAL-TIME CAPTION STREAM</p>
    <p class="caption">{captions[captionIndex] ?? "WAITING FOR LECTURER SPEECH..."}</p>
    <button class="outline" onclick={onNextCaption}>
      Next Caption
    </button>
  </article>

  <aside class="panel">
    <p class="eyebrow">KEY IDEA / SUMMARY</p>
    <h2>Feedback makes a system responsive.</h2>
    <p class="hint">
      Capture signal, compare with target outcome, and adjust action accordingly.
    </p>
    <hr />
    <p class="eyebrow">
      Check-ins: {joinedParticipant?.heartbeats ?? 0}
    </p>
    <button class="primary full" onclick={onHeartbeat}>I'm still here</button>
  </aside>
</section>

<section class="archive-cta panel">
  <div>
    <p class="eyebrow">PERSISTENT ACCESS</p>
    <h2>Retain your learning resources.</h2>
    <p>Create a student account after class to retain transcripts, flashcards, and notes.</p>
  </div>
  {#if accountCreated}
    <p class="success">Account interest recorded. Access retained for your matric number.</p>
  {:else}
    <button class="primary" onclick={createAccount}>Create Account to Claim Archive</button>
  {/if}
</section>
