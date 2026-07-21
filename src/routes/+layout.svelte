<script lang="ts">
  import "../styles/theme.css";
  import "../styles/layout.css";
  import "../styles/elements.css";
  import "../styles/forms.css";
  import "../styles/components.css";

  let { children } = $props();
</script>

<svelte:head>
  <title>Klasync — Every lecture, within reach</title>
  <meta name="description" content="Live captions, fair attendance, and a lasting learning archive — without making students create an account just to join class." />
</svelte:head>

{@render children()}
