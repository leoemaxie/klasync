<script lang="ts">
  import ButtonSpinner from '$lib/components/shared/ButtonSpinner.svelte';
  import { Radio } from '@lucide/svelte';

  let {
    lecturerName = '',
    lecturerEmail = '',
    apiNotice = '',
    isSaving = false,
    onStartSession,
  }: {
    lecturerName: string;
    lecturerEmail: string;
    apiNotice?: string;
    isSaving?: boolean;
    onStartSession: () => void;
  } = $props();
</script>

<div class="panel start-session-card">
  <div class="start-header">
    <p class="eyebrow">START SESSION</p>
    <h2 class="start-title">Start Live Session</h2>
    <p class="start-desc">
      Create an access code and invite link for students.
    </p>
  </div>

  <div class="lecturer-summary-box">
    <div class="summary-item">
      <span class="sum-label">Lecturer:</span>
      <span class="sum-val">{lecturerName || 'Not set'}</span>
    </div>
    <div class="summary-item">
      <span class="sum-label">Email:</span>
      <span class="sum-val">{lecturerEmail || 'Not set'}</span>
    </div>
  </div>

  {#if apiNotice}
    <p class="error-notice">{apiNotice}</p>
  {/if}

  <button
    type="button"
    class="primary start-btn"
    onclick={onStartSession}
    disabled={!lecturerName.trim() || !lecturerEmail.trim() || isSaving}
  >
    {#if isSaving}
      <ButtonSpinner label="Starting session..." /> Starting...
    {:else}
      <Radio size={16} /> Start Session
    {/if}
  </button>
</div>

<style>
  .start-session-card {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-20);
  }
  .start-header {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-8);
  }
  .start-title {
    font-family: var(--font-display);
    font-size: 24px;
    color: var(--color-warm-cream);
    margin: 0;
  }
  .start-desc {
    font-size: 13px;
    color: var(--color-driftwood);
    margin: 0;
    line-height: 1.4;
  }
  .lecturer-summary-box {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 14px;
    background: rgba(16, 9, 4, 0.5);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
  }
  .summary-item {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
  }
  .sum-label {
    color: var(--color-driftwood);
    letter-spacing: 0.08em;
  }
  .sum-val {
    color: var(--color-warm-cream);
    font-weight: 500;
  }
  .start-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 12px;
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .error-notice {
    font-size: 12px;
    color: var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.1);
    padding: 8px 12px;
    border-radius: 4px;
    border: 1px solid rgba(220, 80, 0, 0.2);
  }
</style>
