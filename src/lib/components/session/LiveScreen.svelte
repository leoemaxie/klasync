<script lang="ts">
  import { onMount } from "svelte";
  import type { Participant, Screen, Session } from "$lib/types";
  import { connectCaptionWebSocket } from "$lib/api/captions";
  import { sendHandRaise, clearHandRaise, sendPresenceHeartbeat, claimAttendance } from "$lib/api";
  import Skeleton from "$lib/components/shared/Skeleton.svelte";
  import ButtonSpinner from "$lib/components/shared/ButtonSpinner.svelte";
  import AccessibilityDrawer from "./AccessibilityDrawer.svelte";
  import LiveQaPanel from "./LiveQaPanel.svelte";

  import { Hand } from "@lucide/svelte";

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
  let isHandRaised = $state(false);
  let isCheckingIn = $state(false);
  let isClaiming = $state(false);
  let claimNotice = $state("");

  // Accessibility reading settings state
  let fontSize = $state("18px");
  let dyslexicFont = $state(false);
  let lineHeight = $state(1.6);

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

  async function handleToggleHandRaise() {
    if (!session?.code || !joinedParticipant?.id) return;
    try {
      if (isHandRaised) {
        await clearHandRaise(session.code, joinedParticipant.id);
        isHandRaised = false;
      } else {
        await sendHandRaise(session.code, joinedParticipant.id);
        isHandRaised = true;
      }
    } catch {
      // Toggle locally on fallback
      isHandRaised = !isHandRaised;
    }
  }

  async function handleCheckIn() {
    isCheckingIn = true;
    try {
      onHeartbeat();
      if (session?.code && joinedParticipant?.matric) {
        await sendPresenceHeartbeat(session.code, joinedParticipant.matric).catch(() => {});
      }
    } finally {
      isCheckingIn = false;
    }
  }

  async function createAccount() {
    isClaiming = true;
    try {
      if (session?.code && joinedParticipant?.matric) {
        try {
          await claimAttendance(session.code, joinedParticipant.matric);
        } catch (err) {
          claimNotice = err instanceof Error ? err.message : "Claim recorded locally";
        }
      }
      accountCreated = true;
      screen = "archive";
    } finally {
      isClaiming = false;
    }
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

<AccessibilityDrawer bind:fontSize bind:dyslexicFont bind:lineHeight />

<section class="live-grid">
  <article
    class="captions panel"
    class:dyslexic-mode={dyslexicFont}
    style="font-size: {fontSize}; line-height: {lineHeight};"
    aria-live="polite"
  >
    <p class="eyebrow">REAL-TIME CAPTION STREAM</p>
    {#if captions.length === 0}
      <div style="margin: var(--spacing-14) 0;">
        <Skeleton height="56px" label="Waiting for speech-to-text audio stream..." />
      </div>
    {:else}
      <p class="caption">{captions[captionIndex] ?? "WAITING FOR LECTURER SPEECH..."}</p>
    {/if}
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
    <div class="student-action-row">
      <button class="primary full" onclick={handleCheckIn} disabled={isCheckingIn}>
        {#if isCheckingIn}
          <ButtonSpinner label="Recording check-in..." /> Checking in...
        {:else}
          I'm still here
        {/if}
      </button>
      <button class={isHandRaised ? "danger" : "outline"} onclick={handleToggleHandRaise}>
        <Hand size={16} style="vertical-align: middle; display: inline-block;" /> {isHandRaised ? "Hand Raised" : "Raise Hand"}
      </button>
    </div>
  </aside>
</section>

<LiveQaPanel sessionCode={session?.code ?? "A4K9QZ"} participantId={joinedParticipant?.id} />

<section class="archive-cta panel">
  <div>
    <p class="eyebrow">PERSISTENT ACCESS</p>
    <h2>Retain your learning resources.</h2>
    <p>Create a student account after class to retain transcripts, flashcards, and notes.</p>
  </div>
  {#if accountCreated}
    <p class="success">Account interest recorded. Access retained for your matric number. {claimNotice}</p>
  {:else}
    <button class="primary" onclick={createAccount} disabled={isClaiming}>
      {#if isClaiming}
        <ButtonSpinner label="Claiming lecture archive..." /> Claiming...
      {:else}
        Create Account to Claim Archive
      {/if}
    </button>
  {/if}
</section>

<style>
  .student-action-row {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-12);
  }
</style>
