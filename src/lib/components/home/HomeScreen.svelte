<script lang="ts">
  import type { Screen } from '$lib/types';
  import { onMount } from 'svelte';
  import HeroSection from './HeroSection.svelte';
  import FeatureGridSection from './FeatureGridSection.svelte';
  import LandingFooter from './LandingFooter.svelte';

  let { screen = $bindable() }: { screen: Screen } = $props();

  let visible = $state(false);
  onMount(() => {
    requestAnimationFrame(() => {
      visible = true;
    });
  });
</script>

<svelte:head>
  <title>Klasync — Every lecture, within reach</title>
</svelte:head>

<div class="home-page">
  <HeroSection bind:screen {visible} />
  <FeatureGridSection />
  <LandingFooter bind:screen />
</div>
