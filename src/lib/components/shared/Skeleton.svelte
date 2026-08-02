<script lang="ts">
  let {
    width = '100%',
    height = '20px',
    borderRadius = '6px',
    label = 'Loading content...',
    className = '',
  }: {
    width?: string;
    height?: string;
    borderRadius?: string;
    label?: string;
    className?: string;
  } = $props();
</script>

<div
  class="skeleton-box {className}"
  style="width: {width}; height: {height}; border-radius: {borderRadius};"
  role="status"
  aria-busy="true"
  aria-live="polite"
>
  <span class="sr-only">{label}</span>
</div>
