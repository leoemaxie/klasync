<script lang="ts">
  import { onMount } from 'svelte';
  import type { Participant, Screen, Session } from '$lib/types';
  import type { AuthUser } from '$lib/api/auth';
  import { connectCaptionWebSocket } from '$lib/api/captions';
  import {
    sendHandRaise,
    clearHandRaise,
    sendPresenceHeartbeat,
    claimAttendance,
  } from '$lib/api';
  import { requestWakeLock, releaseWakeLock } from '$lib/native/wakelock';
  import { triggerHaptic } from '$lib/native/haptics';
  import AccessibilityDrawer from './AccessibilityDrawer.svelte';
  import LiveQaPanel from './LiveQaPanel.svelte';
  import LiveHeader from './LiveHeader.svelte';
  import LiveCaptionCard from './LiveCaptionCard.svelte';
  import LivePresenceCard from './LivePresenceCard.svelte';
  import LiveArchiveCta from './LiveArchiveCta.svelte';

  let {
    session,
    joinedParticipant,
    courseCode = '',
    captions = $bindable([]),
    captionIndex = $bindable(0),
    accountCreated = $bindable(false),
    screen = $bindable(),
    currentUser = null,
    onNextCaption,
    onHeartbeat,
  }: {
    session: Session | null;
    joinedParticipant: Participant | null;
    courseCode?: string;
    captions?: string[];
    captionIndex?: number;
    accountCreated: boolean;
    screen: Screen;
    currentUser?: AuthUser | null;
    onNextCaption: () => void;
    onHeartbeat: () => void;
  } = $props();

  let wsConnected = $state(false);
  let isHandRaised = $state(false);
  let isCheckingIn = $state(false);
  let isClaiming = $state(false);
  let claimNotice = $state('');
  let fontSize = $state('18px');
  let dyslexicFont = $state(false);
  let lineHeight = $state(1.6);

  onMount(() => {
    void requestWakeLock();
    let wsCleanup: (() => void) | undefined;
    if (session?.code) {
      wsCleanup = connectCaptionWebSocket(session.code, (cap) => {
        wsConnected = true;
        if (cap.text?.trim()) {
          const text = cap.text.trim();
          const filtered = captions.filter(
            (c) => c !== 'WAITING FOR LIVE CAPTIONS.'
          );
          if (!filtered.includes(text)) {
            captions = [...filtered, text];
            captionIndex = captions.length - 1;
            triggerHaptic('light');
          }
        }
      });
    }
    return () => {
      wsCleanup?.();
      void releaseWakeLock();
    };
  });

  async function handleToggleHandRaise() {
    triggerHaptic(isHandRaised ? 'light' : 'medium');
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
      isHandRaised = !isHandRaised;
    }
  }

  async function handleCheckIn() {
    triggerHaptic('success');
    isCheckingIn = true;
    try {
      onHeartbeat();
      if (session?.code && joinedParticipant?.matric)
        await sendPresenceHeartbeat(
          session.code,
          joinedParticipant.matric
        ).catch(() => {});
    } finally {
      isCheckingIn = false;
    }
  }

  async function createAccount() {
    triggerHaptic('success');
    isClaiming = true;
    try {
      if (session?.code && joinedParticipant?.matric) {
        await claimAttendance(
          session.code,
          joinedParticipant.matric,
          joinedParticipant.id,
          session.title,
          courseCode || 'COURSE'
        ).catch(() => {});
      }
      accountCreated = true;
      screen = 'archive';
    } finally {
      isClaiming = false;
    }
  }
</script>

<svelte:head
  ><title
    >{session?.title
      ? `${session.title} — Live · Klasync`
      : 'Live Session — Klasync'}</title
  ></svelte:head
>

<div class="live-workspace-wrap">
  <LiveHeader
    {session}
    {joinedParticipant}
    {wsConnected}
    {currentUser}
    onLecturerView={() => (screen = 'lecturer')}
  />
  <AccessibilityDrawer bind:fontSize bind:dyslexicFont bind:lineHeight />
  <section class="live-content-grid">
    <LiveCaptionCard
      {captions}
      {captionIndex}
      {dyslexicFont}
      {fontSize}
      {lineHeight}
      {onNextCaption}
    />
    <LivePresenceCard
      heartbeats={joinedParticipant?.heartbeats ?? 0}
      {isHandRaised}
      {isCheckingIn}
      onCheckIn={handleCheckIn}
      onToggleHandRaise={handleToggleHandRaise}
    />
  </section>
  <LiveQaPanel
    sessionCode={session?.code ?? 'A4K9QZ'}
    participantId={joinedParticipant?.id}
  />
  <LiveArchiveCta
    {accountCreated}
    {isClaiming}
    {claimNotice}
    onCreateAccount={createAccount}
  />
</div>

<style>
  .live-workspace-wrap {
    padding: calc(var(--nav-height) + 16px) var(--card-padding)
      calc(var(--spacing-68) + env(safe-area-inset-bottom, 0px));
    max-width: 1320px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-16);
  }
  .live-content-grid {
    display: grid;
    grid-template-columns: 2fr 1fr;
    gap: var(--spacing-16);
    align-items: start;
  }
  @media (max-width: 900px) {
    .live-content-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
