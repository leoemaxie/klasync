<script lang="ts">
  import type { Screen } from "$lib/types";
  import { onMount } from "svelte";
  import HeroSection from "./HeroSection.svelte";
  import FeatureGridSection from "./FeatureGridSection.svelte";
  import LandingFooter from "./LandingFooter.svelte";

  let { screen = $bindable() }: { screen: Screen } = $props();

  let visible = $state(false);
  onMount(() => {
    requestAnimationFrame(() => {
      visible = true;
    });
  });
</script>

<div class="home-page">
  <HeroSection bind:screen {visible} />
  <FeatureGridSection />
  <LandingFooter bind:screen />
</div>
