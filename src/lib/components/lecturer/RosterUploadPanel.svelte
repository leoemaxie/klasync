<script lang="ts">
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
  let parsedRows = $derived(
    rosterText
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean)
      .map((line) => {
        const parts = line.split(",");
        const matric = parts[0]?.trim() ?? "";
        const name = parts[1]?.trim() ?? "";
        const isValid = matric.length >= 4 && name.length > 0;
        return { matric, name, isValid };
      })
  );

  const validCount = $derived(parsedRows.filter((r) => r.isValid).length);
  const invalidCount = $derived(parsedRows.length - validCount);

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    isDragging = false;
    if (e.dataTransfer?.files?.length) {
      const file = e.dataTransfer.files[0];
      const fakeEvent = { target: { files: [file] } } as unknown as Event;
      onImportFile(fakeEvent);
    }
  }
</script>

<div class="panel roster-upload-panel">
  <p class="eyebrow">COURSE ROSTER IMPORT &amp; VALIDATION</p>

  <div
    class="dropzone"
    class:dragging={isDragging}
    ondragover={(e) => { e.preventDefault(); isDragging = true; }}
    ondragleave={() => (isDragging = false)}
    ondrop={handleDrop}
    role="region"
    aria-label="File upload dropzone"
  >
    <div class="dropzone-icon">📄</div>
    <p class="dropzone-title">Drag &amp; drop class roster CSV or XLSX file</p>
    <p class="hint">Supports <code>.csv</code> or <code>.xlsx</code> formats with matric number &amp; name</p>
    <label class="outline dropzone-button">
      Browse Files
      <input type="file" accept=".csv,text/csv,.xlsx" onchange={onImportFile} hidden />
    </label>
  </div>

  <div class="paste-section">
    <label>
      Or paste CSV rows directly: <code>matric_number, full_name</code>
      <textarea bind:value={rosterText} rows="4" placeholder="MAT/2023/001, Ada Okafor&#10;MAT/2023/002, Chinedu Obi"></textarea>
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
        <thead>
          <tr>
            <th>Matric No.</th>
            <th>Student Name</th>
            <th>Roster Status</th>
          </tr>
        </thead>
        <tbody>
          {#each parsedRows.slice(0, 5) as row}
            <tr class:row-invalid={!row.isValid}>
              <td><code>{row.matric || "Missing"}</code></td>
              <td>{row.name || "Missing"}</td>
              <td><span class="status-tag">{row.isValid ? "Valid Match" : "Invalid Format"}</span></td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if parsedRows.length > 5}
        <p class="hint">+ {parsedRows.length - 5} more student records mapped</p>
      {/if}
    </div>
  {/if}

  <button type="button" class="outline full" onclick={onParseRoster}>
    Confirm &amp; Prepare Roster ({validCount} Students)
  </button>

  {#if rosterNotice}
    <p class="success">{rosterNotice}</p>
  {/if}
</div>

<style>
  .roster-upload-panel { display: flex; flex-direction: column; gap: var(--spacing-18); }
  .dropzone { border: 2px dashed var(--color-cork-border); border-radius: var(--radius-cards); padding: var(--spacing-24); text-align: center; display: flex; flex-direction: column; align-items: center; gap: var(--spacing-8); transition: border-color 0.2s ease, background 0.2s ease; }
  .dropzone.dragging { border-color: var(--color-ember-accent); background: rgba(220, 80, 0, 0.08); }
  .dropzone-icon { font-size: 24px; }
  .dropzone-title { font-size: 14px; font-weight: 500; color: var(--color-warm-cream); }
  .dropzone-button { margin-top: 8px; cursor: pointer; display: inline-block; }
  .paste-section textarea { width: 100%; margin-top: 6px; }
  .roster-preview-summary { display: flex; gap: var(--spacing-12); }
  .summary-badge { font-size: 10px; letter-spacing: 0.1em; padding: 4px 10px; border-radius: 4px; }
  .summary-badge.valid { background: rgba(74, 183, 114, 0.15); color: #4ab772; border: 1px solid #4ab772; }
  .summary-badge.invalid { background: rgba(220, 80, 0, 0.15); color: var(--color-ember-accent); border: 1px solid var(--color-ember-accent); }
  .mapping-table-wrap { font-size: 12px; }
  .mapping-table { width: 100%; border-collapse: collapse; text-align: left; }
  .mapping-table th, .mapping-table td { padding: 6px 8px; border-bottom: 1px solid var(--color-cork-border); }
  .mapping-table th { font-size: 10px; text-transform: uppercase; color: var(--color-driftwood); }
  .row-invalid { opacity: 0.6; color: var(--color-ember-accent); }
  .status-tag { font-size: 9px; letter-spacing: 0.08em; text-transform: uppercase; }
</style>
