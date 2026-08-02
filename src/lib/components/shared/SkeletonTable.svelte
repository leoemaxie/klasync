<script lang="ts">
  import Skeleton from './Skeleton.svelte';

  let {
    rows = 4,
    cols = 3,
    label = 'Loading table data...',
  }: {
    rows?: number;
    cols?: number;
    label?: string;
  } = $props();
</script>

<div class="skeleton-panel" role="status" aria-busy="true" aria-live="polite">
  <span class="sr-only">{label}</span>

  <table class="skeleton-table">
    <thead>
      <tr>
        {#each Array(cols) as _, colIndex}
          <th style="text-align: left;">
            <Skeleton
              width="70%"
              height="14px"
              label={`Header ${colIndex + 1}`}
            />
          </th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each Array(rows) as _, rowIndex}
        <tr>
          {#each Array(cols) as _, colIndex}
            <td>
              <Skeleton
                width={colIndex === 0
                  ? '85%'
                  : colIndex === cols - 1
                    ? '40%'
                    : '60%'}
                height="18px"
                label={`Row ${rowIndex + 1} item ${colIndex + 1}`}
              />
            </td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
</div>
