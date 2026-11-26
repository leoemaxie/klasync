<script lang="ts">
  import type { Screen } from '$lib/types';
  import { Archive, Radio } from '@lucide/svelte';
  import Button from '$lib/components/ui/Button.svelte';

  let {
    claimsCount = 0,
    onNavigate,
  }: {
    claimsCount: number;
    onNavigate: (screen: Screen) => void;
  } = $props();
</script>

<header class="topbar">
  <div class="titles">
    <h1 class="title">Lecture archive</h1>
    <p class="sub">
      <Archive size={14} />
      {claimsCount}
      {claimsCount === 1 ? 'lecture' : 'lectures'}, searchable and ready to
      study
    </p>
  </div>
  <Button
    variant="outline"
    onclick={() => onNavigate('join')}
    title="Join a live session"
  >
    <Radio size={14} /> Join a session
  </Button>
</header>

<style>
  .topbar {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 12px;
    padding-bottom: 14px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .title {
    margin: 0;
    font-size: clamp(26px, 3vw, 34px);
    font-weight: 500;
    line-height: 1.1;
  }
  .sub {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 6px 0 0;
    font-size: 13.5px;
    color: var(--text-secondary);
  }
  .sub :global(svg) {
    color: var(--text-accent);
  }
</style>
