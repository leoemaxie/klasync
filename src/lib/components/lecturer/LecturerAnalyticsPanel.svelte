<script lang="ts">
  import { onMount } from "svelte";
  import { fetchCourseAnalytics, fetchSessionAnomalies, type CourseAnalyticsSummary, type AttendanceAnomaly } from "$lib/api";
  import SkeletonCard from "$lib/components/shared/SkeletonCard.svelte";
  import ButtonSpinner from "$lib/components/shared/ButtonSpinner.svelte";
  import LecturerAnalyticsMetrics from "./LecturerAnalyticsMetrics.svelte";
  import type { Participant, RosterStudent } from "$lib/types";

  let {
    courseId = "", courseCode = "", courseTitle = "", participants = [], roster = []
  }: {
    courseId?: string; courseCode?: string; courseTitle?: string; participants?: Participant[]; roster?: RosterStudent[];
  } = $props();

  let summary = $state<CourseAnalyticsSummary | null>(null);
  let anomalies = $state<AttendanceAnomaly[]>([]);
  let isLoading = $state(true);
  let isExporting = $state(false);

  onMount(async () => {
    try {
      if (courseId) {
        const [sum, anom] = await Promise.all([fetchCourseAnalytics(courseId), fetchSessionAnomalies(courseId)]);
        summary = sum; anomalies = anom;
      }
    } catch { summary = null; } finally { isLoading = false; }
  });

  async function exportGradebookReport() {
    isExporting = true;
    try {
      await new Promise((resolve) => setTimeout(resolve, 600));
      const activeCourse = courseCode.trim() || summary?.course_code || "COURSE";
      const rows: string[] = ["Course Code,Matric Number,Student Name,Verification Status,Heartbeat Count"];
      const studentsToExport = participants.length > 0
        ? participants.map(p => `${activeCourse},"${p.matric}","${p.name}",${p.verified ? "Verified Match" : "Provisional"},${p.heartbeats}`)
        : roster.map(r => `${activeCourse},"${r.matric}","${r.name}",Registered Roster,0`);
      rows.push(...(studentsToExport.length === 0 ? [`${activeCourse},"N/A","No registered students","Pending Session Entry",0`] : studentsToExport));
      const blob = new Blob([rows.join("\n")], { type: "text/csv;charset=utf-8;" });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.setAttribute("download", `official_gradebook_${activeCourse.replaceAll(" ", "_")}.csv`);
      document.body.appendChild(link); link.click(); document.body.removeChild(link);
    } finally { isExporting = false; }
  }
</script>

<div class="panel analytics-panel">
  <div class="analytics-header">
    <div>
      <p class="eyebrow">LECTURER AUDIT &amp; ATTENDANCE ANALYTICS</p>
      <h2>{courseCode.trim() ? `${courseCode}: ${courseTitle}` : "Course Attendance & Audit Dashboard"}</h2>
    </div>
    <button type="button" class="outline" onclick={exportGradebookReport} disabled={isExporting}>
      {#if isExporting}<ButtonSpinner label="Generating official report..." /> Exporting...{:else}📄 Export Official Gradebook CSV{/if}
    </button>
  </div>

  {#if isLoading}
    <SkeletonCard lines={3} label="Fetching attendance analytics..." />
  {:else}
    <LecturerAnalyticsMetrics {summary} {anomalies} {participants} {courseCode} />
    {#if anomalies.length > 0}
      <div class="anomalies-section">
        <p class="eyebrow">ATTENDANCE ANOMALY AUDIT LOG</p>
        <div class="anomalies-list">
          {#each anomalies as a}
            <div class="anomaly-row">
              <span class="anom-badge warning">⚠️ {a.anomaly_type.toUpperCase()}</span>
              <p><strong>{a.matric_number}</strong>: {a.description}</p>
              <small>{new Date(a.logged_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</small>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .analytics-panel { display: flex; flex-direction: column; gap: var(--spacing-18); }
  .analytics-header { display: flex; justify-content: space-between; align-items: center; gap: var(--spacing-14); flex-wrap: wrap; }
  .anomalies-section { display: flex; flex-direction: column; gap: var(--spacing-12); border-top: 1px dashed var(--color-cork-border); padding-top: var(--spacing-14); }
  .anomalies-list { display: flex; flex-direction: column; gap: 8px; }
  .anomaly-row { display: flex; justify-content: space-between; align-items: center; padding: 8px var(--spacing-12); background: rgba(220, 80, 0, 0.08); border: 1px solid var(--color-cork-border); border-radius: 6px; font-size: 12px; }
  .anom-badge { font-size: 9px; padding: 2px 6px; border-radius: 4px; font-weight: 700; background: rgba(220, 80, 0, 0.2); color: var(--color-ember-accent); }
</style>
