<script lang="ts">
  import { onMount } from "svelte";
  import type { Screen } from "$lib/types";
  import PublicVisualPanel from "$lib/components/shared/PublicVisualPanel.svelte";
  import SkeletonCard from "$lib/components/shared/SkeletonCard.svelte";
  import TranscriptViewer from "./TranscriptViewer.svelte";
  import FlashcardDeck from "./FlashcardDeck.svelte";
  import AudioPlayerPanel from "./AudioPlayerPanel.svelte";
  import { getArchiveResources, type ApiResource } from "$lib/api";

  let { screen = $bindable() }: { screen: Screen } = $props();

  let searchQuery = $state("");
  let activeTab = $state<"transcript" | "flashcards" | "audio">("transcript");
  let apiResources = $state<ApiResource[]>([]);
  let isLoading = $state(true);

  let claims = $state([
    { id: "1", course_code: "CSC 312", session_title: "Human Computer Interaction", date: "2026-07-21" }
  ]);

  onMount(async () => {
    try {
      apiResources = await getArchiveResources();
    } catch {
      // Fallback to local default state
    } finally {
      isLoading = false;
    }
  });

  const filteredClaims = $derived(
    claims.filter(
      (c) =>
        c.course_code.toLowerCase().includes(searchQuery.toLowerCase()) ||
        c.session_title.toLowerCase().includes(searchQuery.toLowerCase())
    )
  );
</script>

<section class="join-wrap">
  <div class="join-left-content">
    <p class="eyebrow">STUDENT ARCHIVE / CLAIMED LECTURES</p>
    <h1>Your learning outlasts the room.</h1>
    <p class="lede">
      Access searchable transcripts, interactive flashcards, and audio replays for your verified claims.
    </p>

    <div class="join-card panel">
      <label>
        Search archive
        <input bind:value={searchQuery} placeholder="Search by course code or session title..." />
      </label>

      {#if isLoading}
        <div style="margin: var(--spacing-18) 0;">
          <SkeletonCard lines={3} label="Fetching student lecture archives from API..." />
        </div>
      {:else if filteredClaims.length}
        <div class="archive-list">
          {#each filteredClaims as claim}
            <div class="archive-row">
              <span class="feat-num">{claim.course_code}</span>
              <h3>{claim.session_title}</h3>
              <p class="hint">Claimed on {claim.date} · Verified Student Access</p>
              
              <div class="tab-selector">
                <button type="button" class={activeTab === "transcript" ? "primary" : "outline"} onclick={() => (activeTab = "transcript")}>Transcript</button>
                <button type="button" class={activeTab === "flashcards" ? "primary" : "outline"} onclick={() => (activeTab = "flashcards")}>Flashcards</button>
                <button type="button" class={activeTab === "audio" ? "primary" : "outline"} onclick={() => (activeTab = "audio")}>Audio Stream</button>
              </div>

              {#if activeTab === "transcript"}
                <TranscriptViewer />
              {:else if activeTab === "flashcards"}
                <FlashcardDeck />
              {:else}
                <AudioPlayerPanel />
              {/if}
            </div>
          {/each}
        </div>
      {:else}
        <p class="hint">No claimed lectures match your search query.</p>
      {/if}

      <button type="button" class="primary full" onclick={() => (screen = "home")}>
        Return to Home
      </button>
    </div>
  </div>

  <PublicVisualPanel title="SEARCHABLE LECTURE ARCHIVE" subtitle="Full-text Search · AI Summaries · Audio Stream Replays" />
</section>

