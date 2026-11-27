<script lang="ts">
  import { onMount } from 'svelte';
  import { Download, TriangleAlert } from '@lucide/svelte';
  import {
    downloadResultsCsv,
    getAssessmentReport,
    getIntegrity,
    getNamedResults,
    getRosterStatus,
    type AssessmentReport,
    type IntegrityRow,
    type RosterEntry,
    type StudentScoreRow,
  } from '$lib/api/assessments';
  import Button from '$lib/components/ui/Button.svelte';
  import Spinner from '$lib/components/ui/Spinner.svelte';

  let {
    assessmentId,
    revealed = false,
    refreshKey = 0,
  }: {
    assessmentId: string;
    /** Identities are shown only after the lecturer finalises. */
    revealed?: boolean;
    refreshKey?: number;
  } = $props();

  let report = $state<AssessmentReport | null>(null);
  let roster = $state<RosterEntry[]>([]);
  let integrity = $state<IntegrityRow[]>([]);
  let named = $state<StudentScoreRow[]>([]);
  let loading = $state(true);
  let error = $state('');

  async function load() {
    loading = true;
    error = '';
    try {
      [report, roster, integrity] = await Promise.all([
        getAssessmentReport(assessmentId),
        getRosterStatus(assessmentId),
        getIntegrity(assessmentId),
      ]);
      named = revealed ? await getNamedResults(assessmentId) : [];
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not load the report';
    } finally {
      loading = false;
    }
  }

  onMount(load);
  $effect(() => {
    refreshKey;
    revealed;
    void load();
  });

  const pct = (v: number) => `${Math.round(v * 100)}%`;
  const FLAG_TEXT: Record<string, string> = {
    too_hard: 'Very hard',
    too_easy: 'Very easy',
    non_discriminating: 'Doesn’t separate strong from weak',
    possibly_miskeyed: 'Strong students chose another option: check the key',
  };
  const maxBin = $derived(
    Math.max(1, ...(report?.histogram.map((b) => b.count) ?? [1]))
  );

  async function csv() {
    try {
      const blob = await downloadResultsCsv(assessmentId);
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'assessment-results.csv';
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not export';
    }
  }

  const STATE_LABEL = {
    not_started: 'Not started',
    in_progress: 'In progress',
    submitted: 'Submitted',
  };
</script>

{#if loading && !report}
  <Spinner />
{:else if error && !report}
  <p class="oops" role="alert">{error}</p>
{:else if report}
  <div class="rep">
    {#if report.provisional}
      <p class="prov">
        <TriangleAlert size={14} /> Provisional: {report.pending_grades} answer{report.pending_grades ===
        1
          ? ''
          : 's'} still need a confirmed grade, and they count as zero until then.
      </p>
    {/if}

    <section class="tiles" aria-label="Progress">
      <div><strong>{report.funnel.assigned}</strong><span>Enrolled</span></div>
      <div><strong>{report.funnel.started}</strong><span>Started</span></div>
      <div>
        <strong>{report.funnel.submitted}</strong><span>Submitted</span>
      </div>
      <div>
        <strong>{report.funnel.graded}</strong><span>Fully marked</span>
      </div>
    </section>

    {#if report.summary}
      <section class="surface">
        <h4>Scores</h4>
        <dl class="stats">
          <div>
            <dt>Mean</dt>
            <dd>{pct(report.summary.mean)}</dd>
          </div>
          <div>
            <dt>Median</dt>
            <dd>{pct(report.summary.median)}</dd>
          </div>
          <div>
            <dt>Spread (sd)</dt>
            <dd>{pct(report.summary.std_dev)}</dd>
          </div>
          <div>
            <dt>Lowest–highest</dt>
            <dd>{pct(report.summary.min)}–{pct(report.summary.max)}</dd>
          </div>
          <div>
            <dt>Pass rate (50%)</dt>
            <dd>{pct(report.summary.pass_rate)}</dd>
          </div>
        </dl>
        <div class="hist" role="img" aria-label="Score distribution">
          {#each report.histogram as bin (bin.from)}
            <div
              class="col"
              title={`${pct(bin.from)}–${pct(bin.to)}: ${bin.count}`}
            >
              <span class="count">{bin.count || ''}</span>
              <span class="hbar" style={`height:${(bin.count / maxBin) * 100}%`}
              ></span>
              <span class="lab">{Math.round(bin.from * 100)}</span>
            </div>
          {/each}
        </div>
      </section>
    {:else}
      <p class="empty">No fully marked attempts yet.</p>
    {/if}

    <section class="surface">
      <h4>Questions</h4>
      <div class="tablewrap">
        <table>
          <thead>
            <tr>
              <th>#</th><th>Question</th><th>Correct</th><th>Separates</th><th>
                Skipped
              </th><th>Time</th>
            </tr>
          </thead>
          <tbody>
            {#each report.items as item (item.question_id)}
              <tr>
                <td>{item.position + 1}</td>
                <td class="qtext">
                  {item.prompt}
                  {#if item.topic_tag}<span class="topic">
                      {item.topic_tag}
                    </span>{/if}
                  {#each item.flags as f (f)}<span class="flag">
                      {FLAG_TEXT[f]}
                    </span>{/each}
                  {#if item.options.length}
                    <div class="opts">
                      {#each item.options as o (o.option)}
                        <span
                          class="opt"
                          class:key={o.correct}
                          title={`${o.picked} chose ${o.option}`}
                        >
                          {o.option}: {pct(o.share)}
                        </span>
                      {/each}
                    </div>
                  {/if}
                </td>
                <td>{pct(item.difficulty)}</td>
                <td>
                  {item.discrimination === null
                    ? '–'
                    : item.discrimination.toFixed(2)}
                </td>
                <td>{pct(item.skip_rate)}</td>
                <td>{item.avg_time_secs}s</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      <p class="help">
        “Separates” compares the top and bottom 27% of the class; it needs at
        least 10 attempts.
      </p>
    </section>

    {#if report.topics.length}
      <section class="surface">
        <h4>Topics, weakest first</h4>
        <ul class="topics">
          {#each report.topics as t (t.topic)}
            <li>
              <div class="trow">
                <span>{t.topic}</span><span>{pct(t.mastery)}</span>
              </div>
              <div class="tbar" class:weak={t.mastery < 0.5}>
                <span style={`width:${t.mastery * 100}%`}></span>
              </div>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <section class="surface">
      <h4>Who has taken it</h4>
      <p class="help">Status only. Scores stay blind until you finalise.</p>
      <div class="tablewrap">
        <table>
          <thead>
            <tr>
              <th>Student</th><th>Matric</th><th>Status</th><th>Attempts</th>
            </tr>
          </thead>
          <tbody>
            {#each roster as r (r.student_account_id)}
              <tr>
                <td>{r.display_name}</td>
                <td>{r.matric_number}</td>
                <td>
                  <span class="st" data-s={r.state}>
                    {STATE_LABEL[r.state]}
                  </span>
                </td>
                <td>{r.attempts}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </section>

    <section class="surface">
      <h4>Integrity signals</h4>
      <p class="help">
        {report.integrity.attempts_with_flags} with flags · {report.integrity
          .offline_submits} submitted offline ·
        {report.integrity.late_submits} late. Flags are for you to review, never penalties.
      </p>
      {#if integrity.some((i) => i.violations > 0 || i.submitted_offline || i.late)}
        <div class="tablewrap">
          <table>
            <thead>
              <tr><th>Candidate</th><th>Flags</th><th>Signals</th></tr>
            </thead>
            <tbody>
              {#each integrity.filter((i) => i.violations > 0 || i.submitted_offline || i.late) as i (i.alias)}
                <tr>
                  <td>{i.alias}</td>
                  <td>
                    {#if i.late}<span class="flag">Late</span>{/if}
                    {#if i.submitted_offline}<span class="flag">
                        Submitted offline
                      </span>{/if}
                  </td>
                  <td>
                    {Object.entries(i.events)
                      .map(([k, n]) => `${k.replaceAll('_', ' ')} ×${n}`)
                      .join(', ') || '–'}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </section>

    {#if revealed}
      <section class="surface">
        <div class="named-head">
          <h4>Results by student</h4>
          <Button variant="outline" size="sm" onclick={csv}>
            <Download size={14} /> Export CSV
          </Button>
        </div>
        <div class="tablewrap">
          <table>
            <thead>
              <tr>
                <th>Student</th><th>Matric</th><th>Attempts</th><th>
                  Score
                </th><th></th>
              </tr>
            </thead>
            <tbody>
              {#each named as r (r.student_account_id)}
                <tr>
                  <td>{r.display_name}</td>
                  <td>{r.matric_number}</td>
                  <td>{r.attempts}</td>
                  <td>{r.percent === null ? '–' : `${r.percent}%`}</td>
                  <td>
                    {#if r.late}<span class="flag">Late</span>{/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </section>
    {/if}
    {#if error}<p class="oops" role="alert">{error}</p>{/if}
  </div>
{/if}

<style>
  .rep {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .surface {
    padding: 16px;
    background: var(--surface-2);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
  }
  h4 {
    margin: 0 0 10px;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: var(--text-accent);
  }
  .prov {
    margin: 0;
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 9px 14px;
    border-radius: var(--radius-md);
    background: var(--surface-accent);
    color: #e3a23c;
    font-size: 13px;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 10px;
  }
  .tiles div {
    padding: 14px;
    border-radius: var(--radius-lg);
    background: var(--surface-2);
    border: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
  }
  .tiles strong {
    font-size: 26px;
    font-weight: 500;
  }
  .tiles span,
  dt {
    font-size: 12px;
    color: var(--text-tertiary);
  }
  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: 20px;
    margin: 0 0 14px;
  }
  dd {
    margin: 0;
    font-size: 18px;
  }
  .hist {
    display: flex;
    gap: 6px;
    height: 130px;
    align-items: flex-end;
  }
  .col {
    flex: 1;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    align-items: center;
    gap: 3px;
    min-width: 0;
  }
  .hbar {
    width: 100%;
    background: var(--fill-accent);
    border-radius: 4px 4px 0 0;
    min-height: 2px;
  }
  .count,
  .lab {
    font-size: 11px;
    color: var(--text-tertiary);
  }
  .tablewrap {
    overflow-x: auto;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13.5px;
  }
  th {
    text-align: left;
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-tertiary);
    padding: 6px 10px;
    border-bottom: 1px solid var(--border-subtle);
  }
  td {
    padding: 9px 10px;
    border-bottom: 1px solid var(--border-subtle);
    vertical-align: top;
  }
  .qtext {
    max-width: 480px;
  }
  .topic,
  .flag,
  .opt {
    display: inline-block;
    margin: 4px 6px 0 0;
    padding: 1px 8px;
    border-radius: 99px;
    background: var(--surface-inset);
    font-size: 11.5px;
    color: var(--text-secondary);
  }
  .flag {
    color: #e3a23c;
  }
  .opt.key {
    color: #5fae7a;
  }
  .st[data-s='submitted'] {
    color: #5fae7a;
  }
  .st[data-s='in_progress'] {
    color: #e3a23c;
  }
  .st[data-s='not_started'] {
    color: var(--text-tertiary);
  }
  .help {
    margin: 8px 0 0;
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .topics {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .trow {
    display: flex;
    justify-content: space-between;
    font-size: 13.5px;
    margin-bottom: 4px;
  }
  .tbar {
    height: 8px;
    border-radius: 99px;
    background: var(--surface-inset);
    overflow: hidden;
  }
  .tbar span {
    display: block;
    height: 100%;
    background: #5fae7a;
  }
  .tbar.weak span {
    background: #e3a23c;
  }
  .named-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .named-head h4 {
    margin: 0;
  }
  .empty {
    color: var(--text-tertiary);
  }
  .oops {
    color: #d9715c;
  }
</style>
