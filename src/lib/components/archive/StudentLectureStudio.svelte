<script lang="ts">
  import type { ClaimRecord } from '$lib/api';
  import LectureWorkspace from '$lib/components/studio/LectureWorkspace.svelte';

  let {
    claim,
    onBack,
    focus = $bindable(false),
    listOpen = $bindable(false),
    showListToggle = false,
    startAt = null,
  }: {
    claim: ClaimRecord;
    onBack?: () => void;
    focus?: boolean;
    listOpen?: boolean;
    showListToggle?: boolean;
    startAt?: number | null;
  } = $props();
</script>

<!-- A fresh workspace (and playhead) for every lecture that is opened. -->
{#key `${claim.id || claim.session_code}:${startAt ?? ''}`}
  <LectureWorkspace
    {claim}
    {onBack}
    bind:focus
    bind:listOpen
    {showListToggle}
    {startAt}
  />
{/key}
