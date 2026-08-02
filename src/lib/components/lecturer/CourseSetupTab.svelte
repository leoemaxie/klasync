<script lang="ts">
  import LecturerFormPanel from "./LecturerFormPanel.svelte";
  import RosterUploadPanel from "./RosterUploadPanel.svelte";

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

<div class="course-setup-tab">
  <LecturerFormPanel
    bind:lecturerName
    bind:lecturerEmail
    bind:courseCode
    bind:courseTitle
  />
  <RosterUploadPanel
    bind:rosterText
    {rosterNotice}
    {onImportFile}
    {onParseRoster}
  />
</div>

<style>
  .course-setup-tab {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-24);
    align-items: start;
  }
  @media (max-width: 900px) {
    .course-setup-tab {
      grid-template-columns: 1fr;
    }
  }
</style>
