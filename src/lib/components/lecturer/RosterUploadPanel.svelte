<script lang="ts">
  import LmsSyncModal from './LmsSyncModal.svelte';
  import RosterPreviewTable from './RosterPreviewTable.svelte';
  import ButtonSpinner from '$lib/components/shared/ButtonSpinner.svelte';
  import { parseRosterTextToStudents } from '$lib/rosterUtils';
  import { Link, FileText, CheckCircle2, CloudUpload } from '@lucide/svelte';

  let {
    rosterText = $bindable(''),
    rosterNotice = '',
    onImportFile,
    onParseRoster,
    onSaveToCloud,
  }: {
    rosterText: string;
    rosterNotice?: string;
    onImportFile: (event: Event) => void;
    onParseRoster: () => Promise<void> | void;
    onSaveToCloud?: () => Promise<void> | void;
  } = $props();

  let isDragging = $state(false);
  let isLmsModalOpen = $state(false);
  let fileInputRef: HTMLInputElement | null = $state(null);

  let isConfirming = $state(false);
  let isConfirmed = $state(false);
  let isSavingCloud = $state(false);
  let isSavedCloud = $state(false);

  const parsedStudents = $derived(parseRosterTextToStudents(rosterText));
  let rawLinesCount = $derived(
    rosterText
      .split(/\r?\n/)
      .map((l) => l.trim())
      .filter(Boolean).length
  );

  async function handleConfirmRoster() {
    if (parsedStudents.length === 0 || isConfirming || isSavingCloud) return;
    isConfirming = true;
    try {
      await onParseRoster();
      isConfirmed = true;
      setTimeout(() => {
        isConfirmed = false;
      }, 4000);
    } catch {
    } finally {
      isConfirming = false;
    }
  }

  async function handleSaveToCloud() {
    if (parsedStudents.length === 0 || isConfirming || isSavingCloud) return;
    isSavingCloud = true;
    try {
      await onSaveToCloud?.();
      isSavedCloud = true;
      setTimeout(() => {
        isSavedCloud = false;
      }, 4000);
    } catch {
    } finally {
      isSavingCloud = false;
    }
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    isDragging = false;
    if (e.dataTransfer?.files?.length) {
      const file = e.dataTransfer.files[0];
      onImportFile({
        currentTarget: { files: [file] },
        target: { files: [file] },
      } as unknown as Event);
    }
  }

  function triggerFileInput() {
    fileInputRef?.click();
  }
  function handleLmsSynced(csvText: string) {
    rosterText = rosterText ? `${rosterText}\n${csvText}` : csvText;
  }
</script>

<div class="panel roster-upload-panel">
  <div class="panel-header">
    <p class="eyebrow">COURSE ROSTER IMPORT &amp; VALIDATION</p>
    <button
      type="button"
      class="text-link-sync"
      onclick={() => (isLmsModalOpen = true)}
    >
      <Link
        size={14}
        aria-hidden="true"
        style="vertical-align: middle; display: inline-block;"
      /> Canvas / Moodle Sync
    </button>
  </div>

  <div
    class="dropzone"
    class:dragging={isDragging}
    ondragover={(e) => {
      e.preventDefault();
      isDragging = true;
    }}
    ondragleave={() => (isDragging = false)}
    ondrop={handleDrop}
    onclick={triggerFileInput}
    role="button"
    tabindex="0"
    onkeydown={(e) =>
      (e.key === 'Enter' || e.key === ' ') && triggerFileInput()}
    aria-label="Upload class roster file"
  >
    <div class="dropzone-icon"><FileText size={24} aria-hidden="true" /></div>
    <p class="dropzone-title">Drag &amp; drop CSV or XLSX file</p>
    <p class="hint">
      Supports <code>.csv</code>, <code>.tsv</code>, or <code>.xlsx</code>
    </p>
    <button
      type="button"
      class="outline dropzone-button"
      onclick={(e) => {
        e.stopPropagation();
        triggerFileInput();
      }}>Browse Files</button
    >
    <input
      bind:this={fileInputRef}
      type="file"
      accept=".csv,text/csv,.tsv,text/tab-separated-values,.xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
      onchange={onImportFile}
      hidden
    />
  </div>

  <div class="paste-section">
    <label for="roster-paste-input"
      >Or paste CSV rows: <code>matric_number, full_name</code>
      <textarea
        id="roster-paste-input"
        bind:value={rosterText}
        rows="3"
        placeholder={'MAT/2023/001, Ada Okafor\nMAT/2023/002, Chinedu Obi'}
      ></textarea>
    </label>
  </div>

  <RosterPreviewTable {parsedStudents} {rawLinesCount} />

  <div class="roster-actions-grid">
    <button
      type="button"
      class={isConfirming
        ? 'saving'
        : isConfirmed
          ? 'success'
          : parsedStudents.length > 0
            ? 'primary'
            : 'outline'}
      onclick={handleConfirmRoster}
      disabled={parsedStudents.length === 0 || isConfirming || isSavingCloud}
    >
      {#if isConfirming}
        <ButtonSpinner label="Saving..." /> Saving...
      {:else if isConfirmed}
        <CheckCircle2
          size={16}
          aria-hidden="true"
          style="vertical-align: middle; display: inline-block; margin-right: 4px;"
        /> Confirmed ({parsedStudents.length})
      {:else}
        Confirm Roster ({parsedStudents.length})
      {/if}
    </button>

    <button
      type="button"
      class={isSavingCloud ? 'saving' : isSavedCloud ? 'success' : 'outline'}
      onclick={handleSaveToCloud}
      disabled={parsedStudents.length === 0 || isConfirming || isSavingCloud}
    >
      {#if isSavingCloud}
        <ButtonSpinner label="Saving to cloud..." /> Save to Cloud
      {:else if isSavedCloud}
        <CheckCircle2
          size={16}
          aria-hidden="true"
          style="vertical-align: middle; display: inline-block; margin-right: 4px;"
        /> Synced to Cloud
      {:else}
        <CloudUpload
          size={16}
          aria-hidden="true"
          style="vertical-align: middle; display: inline-block; margin-right: 4px;"
        /> Save to Cloud
      {/if}
    </button>
  </div>

  {#if rosterNotice}
    <p
      role={rosterNotice.toLowerCase().includes('error') ||
      rosterNotice.toLowerCase().includes('unsupported')
        ? 'alert'
        : 'status'}
      class={rosterNotice.toLowerCase().includes('error') ||
      rosterNotice.toLowerCase().includes('unsupported')
        ? 'error'
        : 'success'}
    >
      {rosterNotice}
    </p>
  {/if}
</div>

<LmsSyncModal bind:isOpen={isLmsModalOpen} onRosterSynced={handleLmsSynced} />

<style>
  .roster-upload-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-8);
    height: 100%;
  }
  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    min-height: 24px;
  }
  .panel-header .eyebrow {
    margin: 0;
  }
  .text-link-sync {
    background: transparent;
    border: 0;
    color: var(--color-warm-cream-dim);
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    cursor: pointer;
    text-decoration: underline;
  }
  .dropzone {
    border: 2px dashed var(--color-cork-border);
    border-radius: var(--radius-cards);
    padding: var(--spacing-20);
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    background: rgba(16, 9, 4, 0.3);
  }
  .dropzone:hover,
  .dropzone.dragging {
    border-color: var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.08);
  }
  .dropzone-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--color-warm-cream);
  }
  .paste-section textarea {
    width: 100%;
    margin-top: 6px;
  }
  .roster-actions-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  @media (max-width: 600px) {
    .roster-actions-grid {
      grid-template-columns: 1fr;
    }
  }
  button.success {
    background: rgba(74, 183, 114, 0.2) !important;
    border: 1px solid #4ab772 !important;
    color: #4ab772 !important;
  }
  button.saving {
    background: rgba(220, 80, 0, 0.15) !important;
    border: 1px solid var(--color-ember-accent) !important;
    color: var(--color-warm-cream) !important;
  }
  :global(.spin-icon) {
    animation: btn-rotate 0.8s linear infinite;
  }
  @keyframes btn-rotate {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
</style>
