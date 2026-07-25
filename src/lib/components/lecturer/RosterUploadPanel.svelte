<script lang="ts">
  let {
    rosterText = $bindable(""), rosterNotice = "", onImportFile, onParseRoster,
  }: {
    rosterText: string; rosterNotice?: string;
    onImportFile: (event: Event) => void; onParseRoster: () => void;
  } = $props();

  let isDragging = $state(false);
  let parsedRows = $derived(
    rosterText.split("\n").map((l) => l.trim()).filter(Boolean).map((line) => {
      const parts = line.split(",");
      const matric = parts[0]?.trim() ?? "";
      const name = parts[1]?.trim() ?? "";
      return { matric, name, isValid: matric.length >= 4 && name.length > 0 };
    })
  );
  const validCount = $derived(parsedRows.filter((r) => r.isValid).length);
  const invalidCount = $derived(parsedRows.length - validCount);

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    isDragging = false;
    if (e.dataTransfer?.files?.length) {
      onImportFile({ target: { files: [e.dataTransfer.files[0]] } } as unknown as Event);
    }
  }
</script>

<div class="panel roster-upload-panel">
  <p class="eyebrow">COURSE ROSTER IMPORT &amp; VALIDATION</p>
  <div
    class="dropzone" class:dragging={isDragging}
    ondragover={(e) => { e.preventDefault(); isDragging = true; }}
    ondragleave={() => (isDragging = false)} ondrop={handleDrop}
    role="region" aria-label="File upload dropzone"
  >
    <div class="dropzone-icon">📄</div>
    <p class="dropzone-title">Drag &amp; drop class roster CSV or XLSX file</p>
    <p class="hint">Supports <code>.csv</code> or <code>.xlsx</code> with matric number &amp; name</p>
    <label class="outline dropzone-button">
      Browse Files
      <input type="file" accept=".csv,text/csv,.xlsx" onchange={onImportFile} hidden />
    </label>
  </div>

  <div class="paste-section">
    <label>
      Or paste CSV rows: <code>matric_number, full_name</code>
      <textarea bind:value={rosterText} rows="3" placeholder="MAT/2023/001, Ada Okafor"></textarea>
    </label>
  </div>

  {#if parsedRows.length > 0}
    <div class="roster-preview-summary">
      <div class="summary-badge valid">✓ {validCount} Valid Students</div>
      {#if invalidCount > 0}
        <div class="summary-badge invalid">⚠ {invalidCount} Row Warnings</div>
      {/if}
    </div>
    <div class="mapping-table-wrap">
      <table class="mapping-table">
        <thead><tr><th>Matric No.</th><th>Student Name</th><th>Status</th></tr></thead>
        <tbody>
          {#each parsedRows.slice(0, 4) as row}
            <tr class:row-invalid={!row.isValid}>
              <td><code>{row.matric || "Missing"}</code></td>
              <td>{row.name || "Missing"}</td>
              <td><span class="status-tag">{row.isValid ? "Match" : "Invalid"}</span></td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}

  <button type="button" class="outline full" onclick={onParseRoster}>
    Confirm &amp; Prepare Roster ({validCount} Students)
  </button>
  {#if rosterNotice}<p class="success">{rosterNotice}</p>{/if}
</div>

<style>
  .roster-upload-panel { display: flex; flex-direction: column; gap: var(--spacing-14); }
  .dropzone { border: 2px dashed var(--color-cork-border); border-radius: var(--radius-cards); padding: var(--spacing-18); text-align: center; display: flex; flex-direction: column; align-items: center; gap: 6px; }
  .dropzone.dragging { border-color: var(--color-ember-accent); background: rgba(220, 80, 0, 0.08); }
  .dropzone-icon { font-size: 20px; }
  .dropzone-title { font-size: 13px; font-weight: 500; color: var(--color-warm-cream); }
  .dropzone-button { margin-top: 4px; cursor: pointer; display: inline-block; }
  .paste-section textarea { width: 100%; margin-top: 4px; }
  .roster-preview-summary { display: flex; gap: var(--spacing-8); }
  .summary-badge { font-size: 10px; padding: 3px 8px; border-radius: 4px; }
  .summary-badge.valid { background: rgba(74, 183, 114, 0.15); color: #4ab772; border: 1px solid #4ab772; }
  .summary-badge.invalid { background: rgba(220, 80, 0, 0.15); color: var(--color-ember-accent); border: 1px solid var(--color-ember-accent); }
  .mapping-table { width: 100%; font-size: 11px; border-collapse: collapse; text-align: left; }
  .mapping-table th, .mapping-table td { padding: 4px 6px; border-bottom: 1px solid var(--color-cork-border); }
  .row-invalid { color: var(--color-ember-accent); }
  .status-tag { font-size: 9px; text-transform: uppercase; }
</style>
