<script lang="ts">
  import { Hand, CheckCircle } from "@lucide/svelte";
  import ButtonSpinner from "$lib/components/shared/ButtonSpinner.svelte";

  let {
    heartbeats = 0,
    isHandRaised = false,
    isCheckingIn = false,
    onCheckIn,
    onToggleHandRaise
  }: {
    heartbeats?: number;
    isHandRaised?: boolean;
    isCheckingIn?: boolean;
    onCheckIn: () => void;
    onToggleHandRaise: () => void;
  } = $props();
</script>

<aside class="panel presence-card">
  <div class="key-idea-block">
    <p class="eyebrow">KEY LECTURE CONCEPT</p>
    <h3>Feedback makes a system responsive.</h3>
    <p class="hint">Capture signal, compare with target outcome, and adjust action accordingly.</p>
  </div>

  <hr class="divider" />

  <div class="presence-block">
    <div class="presence-header">
      <p class="eyebrow">ATTENDANCE CHECK-INS</p>
      <span class="heartbeat-count">{heartbeats} Counted</span>
    </div>

    <div class="student-action-row">
      <button type="button" class="primary full checkin-btn" onclick={onCheckIn} disabled={isCheckingIn}>
        {#if isCheckingIn}
          <ButtonSpinner label="Recording check-in..." /> Checking in...
        {:else}
          <CheckCircle size={16} style="vertical-align: middle; display: inline-block;" /> I'm still here
        {/if}
      </button>

      <button
        type="button"
        class={isHandRaised ? "danger full" : "outline full"}
        onclick={onToggleHandRaise}
      >
        <Hand size={16} style="vertical-align: middle; display: inline-block;" />
        {isHandRaised ? "Hand Raised (Click to Lower)" : "Raise Hand"}
      </button>
    </div>
  </div>
</aside>

<style>
  .presence-card { display: flex; flex-direction: column; gap: var(--spacing-14); }
  .key-idea-block h3 { font-size: 16px; margin: 4px 0; color: var(--color-warm-cream); }
  .divider { border: 0; border-top: 1px dashed var(--color-cork-border); margin: 4px 0; }
  .presence-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
  .heartbeat-count { font-size: 11px; font-weight: 700; color: #4ab772; background: rgba(74, 183, 114, 0.1); border: 1px solid #4ab772; padding: 2px 8px; border-radius: 9999px; }
  .student-action-row { display: flex; flex-direction: column; gap: 10px; }
  @media (max-width: 640px) { .student-action-row { gap: 12px; } }
</style>
