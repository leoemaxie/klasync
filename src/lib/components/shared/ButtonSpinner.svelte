<script lang="ts">
  let {
    label = "Processing..."
  }: {
    label?: string;
  } = $props();
</script>

<span class="btn-spinner" aria-hidden="true"></span>
<span class="sr-only">{label}</span>
