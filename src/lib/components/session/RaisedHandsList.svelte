<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchRaisedHands, type RaisedHand } from '$lib/api/live';
  import { Hand } from '@lucide/svelte';

  let { sessionCode = '' }: { sessionCode: string } = $props();
  let hands = $state<RaisedHand[]>([]);

  async function load() {
    try {
      hands = await fetchRaisedHands(sessionCode);
    } catch {
      /* transient */
    }
  }

  onMount(() => {
    void load();
    const timer = setInterval(load, 5000);
    return () => clearInterval(timer);
  });
</script>

{#if hands.length}
  <section class="panel hands" aria-label="Raised hands" aria-live="polite">
    <p class="eyebrow">HANDS RAISED ({hands.length})</p>
    <ul>
      {#each hands as hand (hand.participant_id)}
        <li>
          <Hand size={13} /> <strong>{hand.display_name}</strong>
          <span>{hand.matric_number}</span>
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .hands ul {
    list-style: none;
    margin: 6px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
  }
  .hands li {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .hands span {
    color: var(--color-warm-cream-dim);
    font-size: 11px;
  }
</style>
