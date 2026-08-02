<script lang="ts">
  let {
    lecturerName = $bindable(""),
    lecturerEmail = $bindable(""),
    courseCode = $bindable(""),
    courseTitle = $bindable(""),
  }: {
    lecturerName: string;
    lecturerEmail: string;
    courseCode: string;
    courseTitle: string;
  } = $props();
</script>

<div class="panel lecturer-form-panel">
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
    <label>Course code<input bind:value={courseCode} placeholder="e.g. CSC 312" /></label>
    <label>Course title<input bind:value={courseTitle} placeholder="e.g. Human Computer Interaction" /></label>
  </div>
</div>

<style>
  .lecturer-form-panel {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-14);
  }
</style>
