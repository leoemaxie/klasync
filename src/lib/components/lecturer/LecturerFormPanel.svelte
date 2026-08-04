<script lang="ts">
  import { UserCheck } from '@lucide/svelte';

  let {
    lecturerName = $bindable(''),
    lecturerEmail = $bindable(''),
    courseCode = $bindable(''),
    courseTitle = $bindable(''),
  }: {
    lecturerName: string;
    lecturerEmail: string;
    courseCode: string;
    courseTitle: string;
  } = $props();
</script>

<div class="panel lecturer-form-panel">
  <div class="panel-header">
    <p class="eyebrow">COURSE DETAILS &amp; LECTURER INFO</p>
  </div>

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
  </div>

  <div class="info-footer-card">
    <div class="info-icon">
      <UserCheck size={16} color="var(--color-ember-accent)" />
    </div>
    <div class="info-text">
      <strong>Lecturer Profiles</strong>
      <p class="hint">
        Your name, email, and course code identify you as the verified session host on student join screens and exported roster logs.
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
    align-items: center;
    min-height: 24px;
  }
  .panel-header .eyebrow {
    margin: 0;
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
