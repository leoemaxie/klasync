<script lang="ts">
  import "../styles/theme.css";
  import "../styles/layout.css";
  import "../styles/elements.css";
  import "../styles/forms.css";

  let { children } = $props();
</script>

<svelte:head>
  <title>KLASYNC — accessible learning, in sync</title>
  <meta name="description" content="Guest-first accessible lecture sessions." />
</svelte:head>

{@render children()}
