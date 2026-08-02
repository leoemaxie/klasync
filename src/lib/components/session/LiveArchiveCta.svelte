<script lang="ts">
  import ButtonSpinner from "$lib/components/shared/ButtonSpinner.svelte";

  let {
    accountCreated = false,
    isClaiming = false,
    claimNotice = "",
    onCreateAccount
  }: {
    accountCreated?: boolean;
    isClaiming?: boolean;
    claimNotice?: string;
    onCreateAccount: () => void;
  } = $props();
</script>

<section class="archive-cta panel">
  <div class="cta-info">
    <p class="eyebrow">PERSISTENT LECTURE ACCESS</p>
    <h2>Retain your learning resources.</h2>
    <p class="hint">Create a student account after class to retain transcripts, AI flashcards, and notes.</p>
  </div>
  <div class="cta-action">
    {#if accountCreated}
      <p class="success">Account interest recorded. Access retained for your matric number. {claimNotice}</p>
    {:else}
      <button type="button" class="primary cta-btn" onclick={onCreateAccount} disabled={isClaiming}>
        {#if isClaiming}
          <ButtonSpinner label="Claiming lecture archive..." /> Claiming...
        {:else}
          Create Account to Claim Archive
        {/if}
      </button>
    {/if}
  </div>
</section>

<style>
  .archive-cta { display: flex; justify-content: space-between; align-items: center; gap: var(--spacing-20); flex-wrap: wrap; margin-top: var(--spacing-20); }
  .cta-info h2 { font-size: 18px; color: var(--color-warm-cream); margin: 4px 0; }
  @media (max-width: 640px) {
    .archive-cta { flex-direction: column; align-items: stretch; }
    .cta-btn { width: 100%; text-align: center; }
  }
</style>
