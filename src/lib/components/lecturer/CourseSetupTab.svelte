<script lang="ts">
  import LecturerFormPanel from './LecturerFormPanel.svelte';
  import RosterUploadPanel from './RosterUploadPanel.svelte';
  import CourseStudentsRosterPanel from './CourseStudentsRosterPanel.svelte';
  import type { RosterStudent } from '$lib/types';

  let {
    lecturerName = $bindable(''),
    lecturerEmail = $bindable(''),
    courseCode = $bindable(''),
    courseTitle = $bindable(''),
    rosterText = $bindable(''),
    rosterNotice = '',
    roster = [],
    onImportFile,
    onParseRoster,
    onSaveToCloud,
    onRemoveStudent,
    onClearRoster,
  }: {
    lecturerName: string;
    lecturerEmail: string;
    courseCode: string;
    courseTitle: string;
    rosterText: string;
    rosterNotice?: string;
    roster?: RosterStudent[];
    onImportFile: (event: Event) => void;
    onParseRoster: () => Promise<void> | void;
    onSaveToCloud?: () => Promise<void> | void;
    onRemoveStudent?: (matric: string) => void;
    onClearRoster?: () => void;
  } = $props();
</script>

<div class="course-setup-tab">
  <div class="setup-grid">
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
      {onSaveToCloud}
    />
  </div>

  <CourseStudentsRosterPanel
    {roster}
    {courseCode}
    {onRemoveStudent}
    {onClearRoster}
  />
</div>

<style>
  .course-setup-tab {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-24);
  }
  .setup-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--spacing-24);
    align-items: stretch;
  }
  @media (max-width: 900px) {
    .setup-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
