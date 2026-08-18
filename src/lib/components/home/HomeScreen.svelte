<script lang="ts">
  import type { Screen } from '$lib/types';
  import type { SessionState } from '$lib/sessionState.svelte';
  import { onMount } from 'svelte';
  import HeroSection from './HeroSection.svelte';
  import FeatureGridSection from './FeatureGridSection.svelte';
  import LandingFooter from './LandingFooter.svelte';

  let {
    screen = $bindable(),
    appState,
  }: {
    screen: Screen;
    appState?: SessionState;
  } = $props();

  let visible = $state(false);

  onMount(() => {
    if (appState?.currentUser?.role === 'student') {
      screen = 'archive';
      return;
    }
    if (
      appState?.currentUser?.role === 'lecturer' ||
      appState?.currentUser?.role === 'admin'
    ) {
      screen = 'lecturer';
      return;
    }
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
