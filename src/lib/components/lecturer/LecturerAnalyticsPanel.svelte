<script lang="ts">
  import { onMount } from "svelte";
  import {
    fetchCourseAnalytics,
    fetchSessionAnomalies,
    type CourseAnalyticsSummary,
    type AttendanceAnomaly
  } from "$lib/api";
  import SkeletonCard from "$lib/components/shared/SkeletonCard.svelte";
  import ButtonSpinner from "$lib/components/shared/ButtonSpinner.svelte";
  import type { Participant, RosterStudent } from "$lib/types";

  let {
    courseId = "",
    courseCode = "",
    courseTitle = "",
    participants = [],
    roster = []
  }: {
    courseId?: string;
    courseCode?: string;
    courseTitle?: string;
    participants?: Participant[];
    roster?: RosterStudent[];
  } = $props();

  let summary = $state<CourseAnalyticsSummary | null>(null);
  let anomalies = $state<AttendanceAnomaly[]>([]);
  let isLoading = $state(true);
  let isExporting = $state(false);

  onMount(async () => {
    try {
      if (courseId) {
        const [sum, anom] = await Promise.all([
          fetchCourseAnalytics(courseId),
          fetchSessionAnomalies(courseId)
        ]);
        summary = sum;
        anomalies = anom;
      }
    } catch {
      summary = null;
    } finally {
      isLoading = false;
    }
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

      if (studentsToExport.length === 0) {
        rows.push(`${activeCourse},"N/A","No registered students","Pending Session Entry",0`);
      } else {
        rows.push(...studentsToExport);
      }

      const csvContent = rows.join("\n");
      const blob = new Blob([csvContent], { type: "text/csv;charset=utf-8;" });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.setAttribute("download", `official_gradebook_${activeCourse.replaceAll(" ", "_")}.csv`);
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
    } finally {
      isExporting = false;
    }
  }
</script>

<div class="panel analytics-panel">
  <div class="analytics-header">
    <div>
      <p class="eyebrow">LECTURER AUDIT &amp; ATTENDANCE ANALYTICS</p>
      <h2>{courseCode.trim() ? `${courseCode}: ${courseTitle}` : "Course Attendance & Audit Dashboard"}</h2>
    </div>
    <button type="button" class="outline" onclick={exportGradebookReport} disabled={isExporting}>
      {#if isExporting}
        <ButtonSpinner label="Generating official report..." /> Exporting...
      {:else}
        📄 Export Official Gradebook CSV
      {/if}
    </button>
  </div>

  {#if isLoading}
    <SkeletonCard lines={3} label="Fetching attendance analytics..." />
  {:else}
    <div class="metrics-grid">
      <div class="metric-card">
        <span class="m-val">{summary ? `${summary.avg_attendance_percentage}%` : `${participants.length}`}</span>
        <span class="m-lbl">{summary ? "Avg. Room Attendance" : "Active Attendees"}</span>
      </div>

      <div class="metric-card">
        <span class="m-val">
          {summary ? `${summary.roster_verification_match_rate}%` : `${participants.filter(p => p.verified).length}`}
        </span>
        <span class="m-lbl">{summary ? "Roster Match Integrity" : "Verified Roster Matches"}</span>
      </div>

      <div class="metric-card">
        <span class="m-val">{summary ? summary.total_sessions : (courseCode ? "1" : "0")}</span>
        <span class="m-lbl">Sessions Tracked</span>
      </div>

      <div class="metric-card">
        <span class="m-val warning">{summary ? summary.total_anomalies_flagged : anomalies.length}</span>
        <span class="m-lbl">Audit Flags</span>
      </div>
    </div>

    {#if anomalies.length > 0}
      <div class="anomalies-section">
        <p class="eyebrow">ATTENDANCE ANOMALY AUDIT LOG</p>
        <div class="anomalies-list">
          {#each anomalies as a}
            <div class="anomaly-row">
              <span class="anom-badge {a.severity}">⚠️ {a.anomaly_type.toUpperCase()}</span>
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
  .analytics-panel { display: flex; flex-direction: column; gap: var(--spacing-18); margin-top: var(--spacing-24); }
  .analytics-header { display: flex; justify-content: space-between; align-items: center; gap: var(--spacing-14); flex-wrap: wrap; }
  .metrics-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: var(--spacing-12); }
  .metric-card { display: flex; flex-direction: column; padding: var(--spacing-14); background: rgba(16, 9, 4, 0.5); border: 1px solid var(--color-cork-border); border-radius: var(--radius-cards); text-align: center; }
  .m-val { font-family: var(--font-display); font-size: 28px; color: var(--color-warm-cream); }
  .m-val.warning { color: var(--color-ember-accent); }
  .m-lbl { font-size: 10px; letter-spacing: 0.1em; color: var(--color-driftwood); margin-top: 4px; text-transform: uppercase; }
  .anomalies-section { display: flex; flex-direction: column; gap: var(--spacing-12); border-top: 1px dashed var(--color-cork-border); padding-top: var(--spacing-14); }
  .anomalies-list { display: flex; flex-direction: column; gap: 8px; }
  .anomaly-row { display: flex; justify-content: space-between; align-items: center; padding: 8px var(--spacing-12); background: rgba(220, 80, 0, 0.08); border: 1px solid var(--color-cork-border); border-radius: 6px; font-size: 12px; }
  .anom-badge { font-size: 9px; padding: 2px 6px; border-radius: 4px; font-weight: 700; }
  .anom-badge.warning { background: rgba(220, 80, 0, 0.2); color: var(--color-ember-accent); }
  @media (max-width: 768px) {
    .metrics-grid { grid-template-columns: 1fr 1fr; }
  }
</style>

