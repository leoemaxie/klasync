<script lang="ts">
  import LmsSyncModal from "./LmsSyncModal.svelte";
  import RosterPreviewTable from "./RosterPreviewTable.svelte";
  import { parseRosterTextToStudents } from "$lib/rosterUtils";
  import { Link, FileText } from "@lucide/svelte";

  let {
    rosterText = $bindable(""),
    rosterNotice = "",
    onImportFile,
    onParseRoster,
  }: {
    rosterText: string;
    rosterNotice?: string;
    onImportFile: (event: Event) => void;
    onParseRoster: () => void;
  } = $props();

  let isDragging = $state(false);
  let isLmsModalOpen = $state(false);
  let fileInputRef: HTMLInputElement | null = $state(null);

  const parsedStudents = $derived(parseRosterTextToStudents(rosterText));
  let rawLinesCount = $derived(rosterText.split(/\r?\n/).map((l) => l.trim()).filter(Boolean).length);

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    isDragging = false;
    if (e.dataTransfer?.files?.length) {
      const file = e.dataTransfer.files[0];
      onImportFile({ currentTarget: { files: [file] }, target: { files: [file] } } as unknown as Event);
    }
  }

  function triggerFileInput() { fileInputRef?.click(); }
  function handleLmsSynced(csvText: string) { rosterText = rosterText ? `${rosterText}\n${csvText}` : csvText; }
</script>

<div class="panel roster-upload-panel">
  <div class="panel-header">
    <p class="eyebrow">COURSE ROSTER IMPORT &amp; VALIDATION</p>
    <button type="button" class="text-link-sync" onclick={() => (isLmsModalOpen = true)}>
      <Link size={14} style="vertical-align: middle; display: inline-block;" /> Canvas / Moodle Sync
    </button>
  </div>

  <div
    class="dropzone" class:dragging={isDragging}
    ondragover={(e) => { e.preventDefault(); isDragging = true; }}
    ondragleave={() => (isDragging = false)}
    ondrop={handleDrop} onclick={triggerFileInput}
    role="button" tabindex="0"
    onkeydown={(e) => (e.key === "Enter" || e.key === " ") && triggerFileInput()}
    aria-label="Upload class roster file"
  >
    <div class="dropzone-icon"><FileText size={24} /></div>
    <p class="dropzone-title">Drag &amp; drop roster CSV or XLSX file</p>
    <p class="hint">Supports <code>.csv</code>, <code>.tsv</code>, or <code>.xlsx</code></p>
    <button type="button" class="outline dropzone-button" onclick={(e) => { e.stopPropagation(); triggerFileInput(); }}>Browse Files</button>
    <input bind:this={fileInputRef} type="file" accept=".csv,text/csv,.tsv,text/tab-separated-values,.xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" onchange={onImportFile} hidden />
  </div>

  <div class="paste-section">
    <label>Or paste CSV rows: <code>matric_number, full_name</code>
      <textarea bind:value={rosterText} rows="3" placeholder="MAT/2023/001, Ada Okafor&#10;MAT/2023/002, Chinedu Obi"></textarea>
    </label>
  </div>

  <RosterPreviewTable {parsedStudents} {rawLinesCount} />

  <button type="button" class="outline full" onclick={onParseRoster} disabled={parsedStudents.length === 0}>
    Confirm &amp; Prepare Roster ({parsedStudents.length} Students)
  </button>

  {#if rosterNotice}
    <p class={rosterNotice.toLowerCase().includes("error") || rosterNotice.toLowerCase().includes("unsupported") ? "error" : "success"}>{rosterNotice}</p>
  {/if}
</div>

<LmsSyncModal bind:isOpen={isLmsModalOpen} onRosterSynced={handleLmsSynced} />

<style>
  .roster-upload-panel { display: flex; flex-direction: column; gap: var(--spacing-14); }
  .panel-header { display: flex; justify-content: space-between; align-items: center; gap: 8px; flex-wrap: wrap; }
  .text-link-sync { background: transparent; border: 0; color: var(--color-warm-cream-dim); font-size: 11px; letter-spacing: 0.08em; text-transform: uppercase; cursor: pointer; text-decoration: underline; }
  .dropzone { border: 2px dashed var(--color-cork-border); border-radius: var(--radius-cards); padding: var(--spacing-20); text-align: center; display: flex; flex-direction: column; align-items: center; gap: 6px; cursor: pointer; background: rgba(16, 9, 4, 0.3); }
  .dropzone:hover, .dropzone.dragging { border-color: var(--color-ember-accent); background: rgba(220, 80, 0, 0.08); }
  .dropzone-title { font-size: 13px; font-weight: 500; color: var(--color-warm-cream); }
  .paste-section textarea { width: 100%; margin-top: 6px; }
</style>
