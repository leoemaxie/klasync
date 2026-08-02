<script lang="ts">
  import Skeleton from './Skeleton.svelte';

  let {
    lines = 3,
    showHeading = true,
    label = 'Loading card details...',
  }: {
    lines?: number;
    showHeading?: boolean;
    label?: string;
  } = $props();
</script>

<div class="skeleton-panel" role="status" aria-busy="true" aria-live="polite">
  <span class="sr-only">{label}</span>

  {#if showHeading}
    <div
      style="display: flex; justify-content: space-between; align-items: center;"
    >
      <Skeleton width="45%" height="24px" label="Loading card heading" />
      <Skeleton
        width="18%"
        height="16px"
        borderRadius="12px"
        label="Loading status tag"
      />
    </div>
  {/if}

  {#each Array(lines) as _, i}
    <Skeleton
      width={i === lines - 1 ? '65%' : '100%'}
      height="16px"
      label={`Loading line ${i + 1}`}
    />
  {/each}
</div>
