<script lang="ts">
  import LmsSyncModal from "./LmsSyncModal.svelte";
  import { parseRosterTextToStudents } from "$lib/rosterUtils";
  import { Link, FileText, Check, TriangleAlert } from "@lucide/svelte";

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
  
  let rawLinesCount = $derived(
    rosterText.split(/\r?\n/).map((l) => l.trim()).filter(Boolean).length
  );
  let validCount = $derived(parsedStudents.length);
  let invalidCount = $derived(Math.max(0, rawLinesCount - validCount));

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    isDragging = false;
    if (e.dataTransfer?.files?.length) {
      const file = e.dataTransfer.files[0];
      const customEvent = {
        currentTarget: { files: [file] },
        target: { files: [file] },
      } as unknown as Event;
      onImportFile(customEvent);
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
    <button type="button" class="text-link-sync" onclick={() => (isLmsModalOpen = true)}>
      <Link size={14} style="vertical-align: middle; display: inline-block;" /> Canvas / Moodle Sync
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
    onkeydown={(e) => (e.key === "Enter" || e.key === " ") && triggerFileInput()}
    aria-label="Upload class roster file"
  >
    <div class="dropzone-icon"><FileText size={24} /></div>
    <p class="dropzone-title">Drag &amp; drop class roster CSV or XLSX file</p>
    <p class="hint">Supports <code>.csv</code>, <code>.tsv</code>, or <code>.xlsx</code> with matric number &amp; student name</p>
    <button type="button" class="outline dropzone-button" onclick={(e) => { e.stopPropagation(); triggerFileInput(); }}>
      Browse Files
    </button>
    <input
      bind:this={fileInputRef}
      type="file"
      accept=".csv,text/csv,.tsv,text/tab-separated-values,.xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
      onchange={onImportFile}
      hidden
    />
  </div>

  <div class="paste-section">
    <label>
      Or paste CSV rows: <code>matric_number, full_name</code>
      <textarea
        bind:value={rosterText}
        rows="3"
        placeholder="MAT/2023/001, Ada Okafor&#10;MAT/2023/002, Chinedu Obi"
      ></textarea>
    </label>
  </div>

  {#if parsedStudents.length > 0}
    <div class="roster-preview-summary">
      <div class="summary-badge valid">
        <Check size={12} style="vertical-align: middle; display: inline-block;" /> {validCount} Valid Student{validCount === 1 ? "" : "s"}
      </div>
      {#if invalidCount > 0}
        <div class="summary-badge invalid">
          <TriangleAlert size={12} style="vertical-align: middle; display: inline-block;" /> {invalidCount} Header/Ignored Row{invalidCount === 1 ? "" : "s"}
        </div>
      {/if}
    </div>
    <div class="mapping-table-wrap">
      <table class="mapping-table">
        <thead>
          <tr>
            <th>Matric No.</th>
            <th>Student Name</th>
            <th>Status</th>
          </tr>
        </thead>
        <tbody>
          {#each parsedStudents.slice(0, 5) as student}
            <tr>
              <td><code>{student.matric}</code></td>
              <td>{student.name}</td>
              <td><span class="status-tag match">Verified Match</span></td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if parsedStudents.length > 5}
        <p class="table-more-hint">+ {parsedStudents.length - 5} more student records parsed</p>
      {/if}
    </div>
  {/if}

  <button type="button" class="outline full" onclick={onParseRoster} disabled={parsedStudents.length === 0}>
    Confirm &amp; Prepare Roster ({validCount} Student{validCount === 1 ? "" : "s"})
  </button>

  {#if rosterNotice}
    <p class={rosterNotice.toLowerCase().includes("error") || rosterNotice.toLowerCase().includes("unsupported") ? "error" : "success"}>
      {rosterNotice}
    </p>
  {/if}
</div>

<LmsSyncModal bind:isOpen={isLmsModalOpen} onRosterSynced={handleLmsSynced} />

<style>
  .roster-upload-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14);
  }
  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
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
  .text-link-sync:hover {
    color: var(--color-warm-cream);
  }
  .dropzone {
    border: 2px dashed var(--color-cork-border);
    border-radius: var(--radius-cards);
    padding: var(--spacing-24) var(--spacing-18);
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    transition: border-color 0.2s ease, background 0.2s ease;
    background: rgba(16, 9, 4, 0.3);
  }
  .dropzone:hover,
  .dropzone.dragging {
    border-color: var(--color-ember-accent);
    background: rgba(220, 80, 0, 0.08);
  }
  .dropzone-icon {
    font-size: 24px;
  }
  .dropzone-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--color-warm-cream);
  }
  .dropzone-button {
    margin-top: 4px;
    cursor: pointer;
  }
  .paste-section textarea {
    width: 100%;
    margin-top: 6px;
  }
  .roster-preview-summary {
    display: flex;
    gap: var(--spacing-8);
  }
  .summary-badge {
    font-size: 10px;
    padding: 3px 8px;
    border-radius: 4px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .summary-badge.valid {
    background: rgba(74, 183, 114, 0.15);
    color: #4ab772;
    border: 1px solid #4ab772;
  }
  .summary-badge.invalid {
    background: rgba(220, 80, 0, 0.15);
    color: var(--color-ember-accent);
    border: 1px solid var(--color-ember-accent);
  }
  .mapping-table-wrap {
    background: rgba(16, 9, 4, 0.4);
    border: 1px solid var(--color-cork-border);
    border-radius: 6px;
    padding: 8px;
    overflow-x: auto;
  }
  .mapping-table {
    width: 100%;
    font-size: 11px;
    border-collapse: collapse;
    text-align: left;
  }
  .mapping-table th {
    color: var(--color-driftwood);
    font-size: 9px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    padding: 6px 8px;
    border-bottom: 1px solid var(--color-cork-border);
  }
  .mapping-table td {
    padding: 6px 8px;
    border-bottom: 1px solid rgba(64, 55, 46, 0.4);
    color: var(--color-warm-cream);
  }
  .status-tag.match {
    font-size: 9px;
    text-transform: uppercase;
    color: #4ab772;
  }
  .table-more-hint {
    font-size: 10px;
    color: var(--color-driftwood);
    margin-top: 6px;
    text-align: center;
  }
</style>

