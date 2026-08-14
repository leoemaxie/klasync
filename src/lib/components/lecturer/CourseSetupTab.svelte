<script lang="ts">
  import LecturerFormPanel from './LecturerFormPanel.svelte';
  import RosterUploadPanel from './RosterUploadPanel.svelte';
  import CourseStudentsRosterPanel from './CourseStudentsRosterPanel.svelte';
  import type { Course, RosterStudent } from '$lib/types';

  let {
    lecturerName = $bindable(''),
    lecturerEmail = $bindable(''),
    courseCode = $bindable(''),
    courseTitle = $bindable(''),
    academicSession = $bindable('2025/2026'),
    semester = $bindable('Second Semester'),
    courses = $bindable([]),
    activeCourse = $bindable(null),
    rosterText = $bindable(''),
    rosterNotice = '',
    roster = [],
    onImportFile,
    onParseRoster,
    onSaveToCloud,
    onRemoveStudent,
    onClearRoster,
    onReloadFromCloud,
    onCourseSelected,
  }: {
    lecturerName: string;
    lecturerEmail: string;
    courseCode: string;
    courseTitle: string;
    academicSession?: string;
    semester?: string;
    courses?: Course[];
    activeCourse?: Course | null;
    rosterText: string;
    rosterNotice?: string;
    roster?: RosterStudent[];
    onImportFile: (event: Event) => void;
    onParseRoster: () => Promise<void> | void;
    onSaveToCloud?: () => Promise<void> | void;
    onRemoveStudent?: (matric: string) => void;
    onClearRoster?: () => void;
    onReloadFromCloud?: () => Promise<void> | void;
    onCourseSelected?: (course: Course) => void;
  } = $props();

  let lastCourseKey = $state('');

  $effect(() => {
    const key = `${courseCode.trim()}::${academicSession.trim()}::${semester.trim()}`;
    if (courseCode.trim() && key !== lastCourseKey && onReloadFromCloud) {
      lastCourseKey = key;
      void onReloadFromCloud();
    }
  });
</script>

<div class="course-setup-tab">
  <div class="setup-grid">
    <LecturerFormPanel
      bind:lecturerName
      bind:lecturerEmail
      bind:courseCode
      bind:courseTitle
      bind:academicSession
      bind:semester
      bind:courses
      bind:activeCourse
      {onCourseSelected}
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
    {onReloadFromCloud}
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
