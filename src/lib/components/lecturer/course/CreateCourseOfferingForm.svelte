<script lang="ts">
  import { createCourse } from '$lib/api/courses';
  import { triggerHaptic } from '$lib/native/haptics';
  import type { Course } from '$lib/types';

  let {
    onCreated,
    onCancel,
  }: {
    onCreated: (c: Course) => void;
    onCancel: () => void;
  } = $props();

  let newCode = $state('');
  let newTitle = $state('');
  let newSession = $state('2025/2026');
  let newSemester = $state('Second Semester');
  let isSubmitting = $state(false);
  let formError = $state('');

  async function handleCreateOffering(e: SubmitEvent) {
    e.preventDefault();
    if (
      !newCode.trim() ||
      !newTitle.trim() ||
      !newSession.trim() ||
      !newSemester.trim()
    ) {
      formError =
        'All fields (Code, Title, Academic Session, Semester) are required.';
      triggerHaptic('error');
      return;
    }
    isSubmitting = true;
    formError = '';
    try {
      const created = await createCourse({
        code: newCode.trim().toUpperCase(),
        title: newTitle.trim(),
        academic_session: newSession.trim(),
        semester: newSemester.trim(),
      });
      triggerHaptic('success');
      onCreated(created);
    } catch (err: any) {
      formError = err?.message || 'Could not create course offering.';
      triggerHaptic('error');
    } finally {
      isSubmitting = false;
    }
  }
</script>

<form class="create-offering-form" onsubmit={handleCreateOffering}>
  <p class="section-lead">Add a new course offering</p>

  {#if formError}
    <div class="error-banner" role="alert">{formError}</div>
  {/if}

  <div class="twocol">
    <label for="offering-code">
      Course code
      <input
        id="offering-code"
        type="text"
        bind:value={newCode}
        placeholder="e.g. MEE 541"
        required
      />
    </label>
    <label for="offering-title">
      Course title
      <input
        id="offering-title"
        type="text"
        bind:value={newTitle}
        placeholder="e.g. Advanced Fluid Dynamics"
        required
      />
    </label>
  </div>

  <div class="twocol">
    <label for="offering-session">
      Academic session
      <select
        id="offering-session"
        bind:value={newSession}
        class="select-input"
        required
      >
        <option value="2025/2026">2025/2026</option>
        <option value="2026/2027">2026/2027</option>
        <option value="2027/2028">2027/2028</option>
      </select>
    </label>
    <label for="offering-semester">
      Semester
      <select
        id="offering-semester"
        bind:value={newSemester}
        class="select-input"
        required
      >
        <option value="Second Semester">Second Semester</option>
        <option value="First Semester">First Semester</option>
        <option value="Harmattan">Harmattan</option>
        <option value="Rain">Rain</option>
        <option value="Summer">Summer</option>
      </select>
    </label>
  </div>

  <div class="form-actions-row">
    <button
      type="button"
      class="outline"
      onclick={onCancel}
      disabled={isSubmitting}>Cancel</button
    >
    <button type="submit" class="primary" disabled={isSubmitting}>
      {isSubmitting ? 'Creating...' : 'Create Course'}
    </button>
  </div>
</form>

<style>
  .create-offering-form {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-16);
  }
  .section-lead {
    font-size: 13px;
    color: var(--color-driftwood);
    margin: 0;
  }
  .error-banner {
    background: rgba(220, 53, 69, 0.15);
    border: 1px solid rgba(220, 53, 69, 0.4);
    color: #ff8585;
    padding: 8px 12px;
    border-radius: 4px;
    font-size: 12px;
  }
  .select-input {
    width: 100%;
    margin-top: var(--spacing-8);
    background: rgba(16, 9, 4, 0.6);
    border: 1px solid var(--color-cork-border);
    color: var(--color-warm-cream);
    padding: 8px 10px;
    font-size: 13px;
    border-radius: 4px;
    outline: none;
  }
  .form-actions-row {
    display: flex;
    justify-content: flex-end;
    gap: var(--spacing-12);
    margin-top: var(--spacing-8);
  }
</style>
