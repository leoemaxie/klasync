<script lang="ts">
  import { UserCheck, Layers } from '@lucide/svelte';
  import type { Course } from '$lib/types';
  import CourseSelectorModal from './CourseSelectorModal.svelte';

  let {
    lecturerName = $bindable(''),
    lecturerEmail = $bindable(''),
    courseCode = $bindable(''),
    courseTitle = $bindable(''),
    academicSession = $bindable('2025/2026'),
    semester = $bindable('Second Semester'),
    courses = $bindable([]),
    activeCourse = $bindable(null),
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
    onCourseSelected?: (course: Course) => void;
  } = $props();

  let isSelectorOpen = $state(false);
</script>

<CourseSelectorModal
  bind:isOpen={isSelectorOpen}
  bind:courses
  bind:activeCourse
  bind:courseCode
  bind:courseTitle
  bind:academicSession
  bind:semester
  {onCourseSelected}
/>

<div class="panel lecturer-form-panel">
  <div class="panel-header">
    <p class="eyebrow">COURSE DETAILS &amp; LECTURER INFO</p>
    <button
      type="button"
      class="outline mini-switcher-btn"
      onclick={() => (isSelectorOpen = true)}
    >
      <Layers size={13} />
      <span>Switch / New Offering</span>
    </button>
  </div>

  <!-- Active Course Indicator Button -->
  <button
    type="button"
    class="offering-summary-pill"
    onclick={() => (isSelectorOpen = true)}
    aria-label="Select or create course offering"
  >
    <div class="pill-left">
      <span class="pill-code">{courseCode || 'NO COURSE SET'}</span>
      <span class="pill-title"
        >{courseTitle || 'Click to select or create a course offering'}</span
      >
    </div>
    <div class="pill-badges">
      <span class="tag session-tag">{academicSession || '2025/2026'}</span>
      <span class="tag semester-tag">{semester || 'Second Semester'}</span>
    </div>
  </button>

  <div class="form-fields">
    <label>
      Lecturer name
      <input bind:value={lecturerName} placeholder="Dr. Amara Okeke" />
    </label>
    <label>
      Institutional email
      <input
        type="email"
        bind:value={lecturerEmail}
        placeholder="amara@university.edu"
      />
    </label>
    <div class="twocol">
      <label
        >Course code<input
          bind:value={courseCode}
          placeholder="e.g. CSC 312"
        /></label
      >
      <label
        >Course title<input
          bind:value={courseTitle}
          placeholder="e.g. Human Computer Interaction"
        /></label
      >
    </div>
    <div class="twocol">
      <label
        >Academic Session<input
          bind:value={academicSession}
          placeholder="e.g. 2025/2026"
        /></label
      >
      <label
        >Semester<input
          bind:value={semester}
          placeholder="e.g. Second Semester"
        /></label
      >
    </div>
  </div>

  <div class="info-footer-card">
    <div class="info-icon">
      <UserCheck size={16} color="var(--color-ember-accent)" />
    </div>
    <div class="info-text">
      <strong>Calendar-Aware Offerings</strong>
      <p class="hint">
        Rosters, sessions, and attendance metrics are securely isolated per
        academic session and semester.
      </p>
    </div>
  </div>
</div>

<style>
  .lecturer-form-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-16);
    height: 100%;
  }
  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    min-height: 24px;
    flex-wrap: wrap;
    gap: 8px;
  }
  .panel-header .eyebrow {
    margin: 0;
  }
  .mini-switcher-btn {
    font-size: 10px;
    padding: 4px 10px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .offering-summary-pill {
    background: rgba(16, 9, 4, 0.55);
    border: 1px solid var(--color-cork-border);
    border-radius: var(--radius-cards);
    padding: 10px 14px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    cursor: pointer;
    transition: all 0.2s ease;
    gap: 8px;
    width: 100%;
    text-align: left;
  }
  .offering-summary-pill:hover {
    border-color: var(--color-ember-accent);
    background: rgba(24, 14, 7, 0.7);
  }
  .pill-left {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .pill-code {
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    font-weight: 700;
    color: var(--color-ember-accent);
    letter-spacing: 0.1em;
  }
  .pill-title {
    font-size: 12px;
    color: var(--color-warm-cream);
  }
  .pill-badges {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }
  .tag {
    font-size: 9px;
    padding: 2px 6px;
    border-radius: 3px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    font-weight: 600;
  }
  .session-tag {
    background: rgba(255, 237, 215, 0.1);
    color: var(--color-driftwood);
  }
  .semester-tag {
    background: rgba(255, 237, 215, 0.15);
    color: var(--color-warm-cream);
  }
  .form-fields {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14);
  }
  .form-fields input {
    padding: 6px 2px;
    font-size: 14px;
    margin-top: 4px;
  }
  .info-footer-card {
    margin-top: auto;
    display: flex;
    align-items: flex-start;
    gap: var(--spacing-12);
    padding: var(--spacing-12) var(--spacing-14);
    background: rgba(16, 9, 4, 0.35);
    border: 1px dashed var(--color-cork-border);
    border-radius: var(--radius-cards);
  }
  .info-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    padding-top: 2px;
  }
  .info-text strong {
    display: block;
    font-size: 12px;
    font-weight: 500;
    color: var(--color-warm-cream);
    margin-bottom: 2px;
  }
  .info-text .hint {
    font-size: 11px;
    line-height: 1.4;
    color: var(--color-driftwood);
  }
</style>
