<script lang="ts">
  let { size = 18, label = '' }: { size?: number; label?: string } = $props();
</script>

<span
  class="ui-spinner"
  style:width="{size}px"
  style:height="{size}px"
  role={label ? 'status' : undefined}
  aria-label={label || undefined}
  aria-hidden={label ? undefined : 'true'}
></span>

<style>
  .ui-spinner {
    display: inline-block;
    border-radius: 50%;
    border: 2px solid currentColor;
    border-right-color: transparent;
    opacity: 0.8;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .ui-spinner {
      animation-duration: 1.6s !important;
      animation-iteration-count: infinite !important;
    }
  }
</style>
