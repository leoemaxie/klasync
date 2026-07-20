<script lang="ts">
  let {
    lecturerName = $bindable(""),
    lecturerEmail = $bindable(""),
    courseCode = $bindable(""),
    courseTitle = $bindable(""),
    rosterText = $bindable(""),
    rosterNotice = "",
    onImportFile,
    onParseRoster,
  }: {
    lecturerName: string;
    lecturerEmail: string;
    courseCode: string;
    courseTitle: string;
    rosterText: string;
    rosterNotice?: string;
    onImportFile: (event: Event) => void;
    onParseRoster: () => void;
  } = $props();
</script>

<div class="panel">
  <label>
    Your name
    <input bind:value={lecturerName} placeholder="Dr. Amara Okeke" />
  </label>
  <label>
    Lecturer email
    <input
      type="email"
      bind:value={lecturerEmail}
      placeholder="amara@university.edu"
    />
  </label>
  <div class="twocol">
    <label>Course code<input bind:value={courseCode} /></label>
    <label>Course title<input bind:value={courseTitle} /></label>
  </div>
  <label>
    Roster CSV <span>optional</span>
    <input type="file" accept=".csv,text/csv" onchange={onImportFile} />
  </label>
  <p class="hint">
    Or paste CSV rows below: <code>matric number, full name</code>
  </p>
  <textarea bind:value={rosterText} rows="5"></textarea>
  <button class="outline full" onclick={onParseRoster}>
    Prepare roster
  </button>
  {#if rosterNotice}
    <p class="success">{rosterNotice}</p>
  {/if}
</div>
