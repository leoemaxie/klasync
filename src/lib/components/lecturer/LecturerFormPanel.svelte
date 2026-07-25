<script lang="ts">
  import RosterUploadPanel from "./RosterUploadPanel.svelte";
  import MicStatusPanel from "./MicStatusPanel.svelte";

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

<div class="lecturer-workspace-left">
  <div class="panel">
    <p class="eyebrow">COURSE DETAILS &amp; LECTURER INFO</p>
    <label>
      Lecturer name
      <input bind:value={lecturerName} placeholder="Dr. Amara Okeke" />
    </label>
    <label>
      Institutional email
      <input type="email" bind:value={lecturerEmail} placeholder="amara@university.edu" />
    </label>
    <div class="twocol">
      <label>Course code<input bind:value={courseCode} /></label>
      <label>Course title<input bind:value={courseTitle} /></label>
    </div>
  </div>

  <MicStatusPanel />

  <RosterUploadPanel
    bind:rosterText
    {rosterNotice}
    {onImportFile}
    {onParseRoster}
  />
</div>

<style>
  .lecturer-workspace-left {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-24);
  }
</style>
