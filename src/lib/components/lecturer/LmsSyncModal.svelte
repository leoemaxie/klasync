<script lang="ts">
  import ButtonSpinner from "$lib/components/shared/ButtonSpinner.svelte";
  import { X } from "@lucide/svelte";

  let {
    isOpen = $bindable(false),
    onRosterSynced,
  }: {
    isOpen: boolean;
    onRosterSynced: (csvText: string) => void;
  } = $props();

  let lmsProvider = $state<"canvas" | "moodle">("canvas");
  let externalCourseId = $state("");
  let apiToken = $state("");
  let isSyncing = $state(false);
  let syncNotice = $state("");

  async function handleSync(e: SubmitEvent) {
    e.preventDefault();
    if (!externalCourseId.trim()) return;
    isSyncing = true;
    syncNotice = "";
    try {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      const simulatedRosterCsv = `MAT/2023/101, Chidi Nnamdi\nMAT/2023/102, Fatima Bello\nMAT/2023/103, Tunde Bakare`;
      onRosterSynced(simulatedRosterCsv);
      syncNotice = `Successfully synced 3 student roster records from ${lmsProvider.toUpperCase()}!`;
      setTimeout(() => {
        isOpen = false;
        syncNotice = "";
      }, 1200);
    } finally {
      isSyncing = false;
    }
  }
</script>

{#if isOpen}
  <div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="LMS Roster Sync Modal">
    <div class="panel modal-card">
      <div class="modal-header">
        <p class="eyebrow">LMS ROSTER AUTOMATION</p>
        <button type="button" class="text" onclick={() => (isOpen = false)}>
          <X size={14} style="vertical-align: middle; display: inline-block;" /> Close
        </button>
      </div>

      <h2>Sync Roster with University LMS</h2>
      <p class="lede">Connect directly to Canvas or Moodle to pull authoritative student enrollment lists.</p>

      <form onsubmit={handleSync} class="lms-form">
        <div class="lms-provider-selector">
          <button
            type="button"
            class={lmsProvider === "canvas" ? "primary" : "outline"}
            onclick={() => (lmsProvider = "canvas")}
          >
            Canvas LMS
          </button>
          <button
            type="button"
            class={lmsProvider === "moodle" ? "primary" : "outline"}
            onclick={() => (lmsProvider = "moodle")}
          >
            Moodle LMS
          </button>
        </div>

        <label>
          Course Code / LMS External ID
          <input bind:value={externalCourseId} placeholder="e.g. 10492" required />
        </label>

        <label>
          LMS Integration Token <span>(Optional)</span>
          <input type="password" bind:value={apiToken} placeholder="canvas_token_••••••••" />
        </label>

        {#if syncNotice}
          <p class="success">{syncNotice}</p>
        {/if}

        <button type="submit" class="primary full" disabled={isSyncing || !externalCourseId.trim()}>
          {#if isSyncing}
            <ButtonSpinner label="Fetching enrollment list from LMS..." /> Syncing Roster...
          {:else}
            Import &amp; Verify Roster from {lmsProvider.toUpperCase()}
          {/if}
        </button>
      </form>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop { position: fixed; inset: 0; z-index: 200; background: rgba(16, 9, 4, 0.85); backdrop-filter: blur(8px); display: flex; align-items: center; justify-content: center; padding: var(--card-padding); }
  .modal-card { max-width: 520px; width: 100%; display: flex; flex-direction: column; gap: var(--spacing-14); }
  .modal-header { display: flex; justify-content: space-between; align-items: center; }
  .lms-form { display: flex; flex-direction: column; gap: var(--spacing-14); }
  .lms-provider-selector { display: flex; gap: var(--spacing-12); }
  .lms-provider-selector button { flex: 1; }
</style>
