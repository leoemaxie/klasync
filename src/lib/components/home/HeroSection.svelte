<script lang="ts">
  import type { Screen } from "$lib/types";
  import WaveformVisual from "./WaveformVisual.svelte";

  let { screen = $bindable(), visible = false }: { screen: Screen; visible?: boolean } = $props();
</script>

<section class="hero" class:visible>
  <div class="hero-arc" aria-hidden="true">
    <svg viewBox="0 0 800 400" preserveAspectRatio="xMidYMid meet" xmlns="http://www.w3.org/2000/svg">
      <path d="M 0 380 A 400 400 0 0 1 800 380" fill="none" stroke="#dc5000" stroke-width="1.5" opacity="0.35"/>
      <path d="M 60 380 A 340 340 0 0 1 740 380" fill="none" stroke="#dc5000" stroke-width="0.8" opacity="0.2"/>
    </svg>
  </div>

  <div class="hero-content">
    <p class="eyebrow">
      <span class="eyebrow-accent">●</span>&nbsp; Assistive Learning Platform &nbsp;/&nbsp; Higher Education
    </p>

    <h1>
      Every lecture,<br />
      <em>accessible to all.</em>
    </h1>

    <p class="lede">
      Transforming physical lectures into live-captioned, searchable, AI-powered learning experiences — with instant guest access for students.
    </p>

    <div class="actions">
      <button class="primary" onclick={() => (screen = "join")}>
        Join a live lecture
      </button>
      <button class="primary-bark" onclick={() => (screen = "lecturer")}>
        Lecturer workspace
      </button>
    </div>
  </div>

  <WaveformVisual {visible} />
</section>
