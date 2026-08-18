<script lang="ts">
  import { onMount } from 'svelte';

  let {
    title = 'ACCESSIBLE LEARNING INFRASTRUCTURE',
    subtitle = 'Live Audio · Real-time Captions · Searchable Archive',
  }: { title?: string; subtitle?: string } = $props();

  const phrases = [
    '...transforming physical lectures into structured, accessible, and searchable study materials in real-time.',
    '...capturing wireless audio, generating live transcripts, and verifying roster attendance automatically.',
    '...ensuring students with mobility or sensory impairments never miss a classroom lecture.',
    '...retaining searchable transcripts, AI revision notes, and flashcard decks forever.',
  ];

  let phraseIndex = $state(0);
  let charIndex = $state(0);
  let currentText = $state('');
  let latency = $state(118);
  let accuracy = $state(99.4);

  onMount(() => {
    const typeTimer = setInterval(() => {
      const target = phrases[phraseIndex];
      if (charIndex < target.length) {
        charIndex++;
        currentText = target.slice(0, charIndex);
      } else {
        setTimeout(() => {
          phraseIndex = (phraseIndex + 1) % phrases.length;
          charIndex = 0;
          currentText = '';
        }, 2200);
      }
    }, 32);

    const metricTimer = setInterval(() => {
      latency = 115 + Math.floor(Math.random() * 10);
      accuracy = +(99.2 + Math.random() * 0.6).toFixed(1);
    }, 1800);

    return () => {
      clearInterval(typeTimer);
      clearInterval(metricTimer);
    };
  });
</script>

<div class="public-visual-panel" aria-hidden="true">
  <div class="visual-arc-bg">
    <svg
      viewBox="0 0 400 400"
      preserveAspectRatio="xMidYMid meet"
      xmlns="http://www.w3.org/2000/svg"
    >
      <circle
        cx="200"
        cy="200"
        r="180"
        fill="none"
        stroke="#dc5000"
        stroke-width="1.2"
        opacity="0.25"
        stroke-dasharray="6 6"
      />
      <circle
        cx="200"
        cy="200"
        r="130"
        fill="none"
        stroke="#ffedd7"
        stroke-width="0.8"
        opacity="0.15"
      />
      <circle
        cx="200"
        cy="200"
        r="80"
        stroke="#dc5000"
        stroke-width="1.5"
        opacity="0.3"
        fill="none"
      />
    </svg>
  </div>

  <div class="visual-k-bg">K</div>

  <div class="waveform-container">
    <div class="waveform-bars">
      {#each [0.35, 0.65, 0.95, 0.75, 0.45, 0.85, 1, 0.6, 0.8, 0.5, 0.9, 0.7, 0.4, 0.8, 0.6, 0.95, 0.55, 0.75] as h, i}
        <div class="bar" style="--h: {h}; --delay: {i * 0.07}s;"></div>
      {/each}
    </div>
    <p class="visual-status-pill">
      <span class="pulse-dot">●</span> LIVE CAPTIONS
    </p>
  </div>

  <div class="caption-preview-card">
    <div class="card-header">
      <div class="header-tag-group">
        <span class="mini-equalizer">
          <span class="m-bar"></span><span class="m-bar"></span><span
            class="m-bar"
          ></span>
        </span>
        <span class="tag">SPEECH TO TEXT</span>
      </div>
      <span class="live-tag"><span class="pulse-dot">●</span> LIVE</span>
    </div>

    <p class="preview-text">
      "{currentText}<span class="typing-cursor">|</span>"
    </p>

    <div class="preview-metrics">
      <span>{latency}ms</span>
      <span>{accuracy}%</span>
      <span>REAL-TIME</span>
    </div>
  </div>

  <div class="visual-footer">
    <p class="eyebrow">{title}</p>
    <p class="visual-subtitle">{subtitle}</p>
  </div>
</div>
