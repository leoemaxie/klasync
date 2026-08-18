<script lang="ts">
  import ButtonSpinner from '$lib/components/shared/ButtonSpinner.svelte';
  import { X } from '@lucide/svelte';

  let {
    isOpen = $bindable(false),
    onRosterSynced,
  }: {
    isOpen: boolean;
    onRosterSynced: (csvText: string) => void;
  } = $props();

  let lmsProvider = $state<'canvas' | 'moodle'>('canvas');
  let externalCourseId = $state('');
  let apiToken = $state('');
  let isSyncing = $state(false);
  let syncNotice = $state('');

  let modalEl = $state<HTMLDivElement | null>(null);

  $effect(() => {
    if (isOpen && modalEl) {
      const firstInput = modalEl.querySelector<HTMLElement>('input, button');
      firstInput?.focus();
    }
  });

  function handleModalKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      isOpen = false;
      return;
    }
    if (e.key === 'Tab' && modalEl) {
      const focusables = modalEl.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      );
      if (!focusables.length) return;
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  }

  async function handleSync(e: SubmitEvent) {
    e.preventDefault();
    if (!externalCourseId.trim()) return;
    isSyncing = true;
    syncNotice = '';
    try {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      const simulatedRosterCsv = `MAT/2023/101, Chidi Nnamdi\nMAT/2023/102, Fatima Bello\nMAT/2023/103, Tunde Bakare`;
      onRosterSynced(simulatedRosterCsv);
      syncNotice = `Successfully synced 3 student roster records from ${lmsProvider.toUpperCase()}!`;
      setTimeout(() => {
        isOpen = false;
        syncNotice = '';
      }, 1200);
    } finally {
      isSyncing = false;
    }
  }
</script>

{#if isOpen}
  <div
    bind:this={modalEl}
    class="modal-backdrop"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    aria-label="LMS Roster Sync Modal"
    onkeydown={handleModalKeydown}
  >
    <div class="panel modal-card">
      <div class="modal-header">
        <p class="eyebrow">LMS IMPORT</p>
        <button type="button" class="text" onclick={() => (isOpen = false)}>
          <X
            size={14}
            aria-hidden="true"
            style="vertical-align: middle; display: inline-block;"
          /> Close
        </button>
      </div>

      <h2>Sync with LMS</h2>
      <p class="lede">Import student list from Canvas or Moodle.</p>

      <form onsubmit={handleSync} class="lms-form">
        <div
          class="lms-provider-selector"
          role="group"
          aria-label="LMS Provider Selection"
        >
          <button
            type="button"
            class={lmsProvider === 'canvas' ? 'primary' : 'outline'}
            aria-pressed={lmsProvider === 'canvas'}
            onclick={() => (lmsProvider = 'canvas')}
          >
            Canvas
          </button>
          <button
            type="button"
            class={lmsProvider === 'moodle' ? 'primary' : 'outline'}
            aria-pressed={lmsProvider === 'moodle'}
            onclick={() => (lmsProvider = 'moodle')}
          >
            Moodle
          </button>
        </div>

        <label for="lms-course-id">
          Course ID
          <input
            id="lms-course-id"
            bind:value={externalCourseId}
            placeholder="e.g. 10492"
            required
          />
        </label>

        <label for="lms-api-token">
          API token (optional)
          <input
            id="lms-api-token"
            type="password"
            bind:value={apiToken}
            placeholder="canvas_token_••••••••"
          />
        </label>

        {#if syncNotice}
          <p class="success" role="status">{syncNotice}</p>
        {/if}

        <button
          type="submit"
          class="primary full"
          disabled={isSyncing || !externalCourseId.trim()}
        >
          {#if isSyncing}
            <ButtonSpinner label="Syncing..." /> Syncing...
          {:else}
            Sync from {lmsProvider === 'canvas' ? 'Canvas' : 'Moodle'}
          {/if}
        </button>
      </form>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 200;
    background: rgba(16, 9, 4, 0.85);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--card-padding);
  }
  .modal-card {
    max-width: 520px;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14);
  }
  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .lms-form {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14);
  }
  .lms-provider-selector {
    display: flex;
    gap: var(--spacing-12);
  }
  .lms-provider-selector button {
    flex: 1;
  }
</style>
