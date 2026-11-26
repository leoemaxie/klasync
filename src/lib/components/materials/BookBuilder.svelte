<script lang="ts">
  import { onMount } from 'svelte';
  import {
    ArrowDown,
    ArrowUp,
    BookOpen,
    Download,
    Plus,
    Trash2,
  } from '@lucide/svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import EmptyState from '$lib/components/ui/EmptyState.svelte';
  import Spinner from '$lib/components/ui/Spinner.svelte';
  import {
    downloadBook,
    fetchBook,
    fetchCourseMaterials,
    publishBook,
    rebuildBook,
    saveBook,
    type BookUnit,
    type CourseBook,
    type ExportFormat,
  } from '$lib/api/materials';
  import { exportFilename, saveBlob } from '$lib/materials/download';

  type Lecture = Awaited<
    ReturnType<typeof fetchCourseMaterials>
  >['materials'][number];
  type Version = Awaited<ReturnType<typeof fetchBook>>['versions'][number];

  let { courseId }: { courseId: string } = $props();

  let book = $state<CourseBook | null>(null);
  let versions = $state<Version[]>([]);
  let lectures = $state<Lecture[]>([]);
  let loading = $state(true);
  let busy = $state('');
  let error = $state('');
  let notice = $state('');
  let dirty = $state(false);
  let sendEmail = $state(false);

  const titleOf = (id: string) =>
    lectures.find((l) => l.id === id)?.title ?? 'Unknown lecture';
  const placed = $derived(
    new Set((book?.structure ?? []).flatMap((u) => u.material_ids))
  );
  const unplaced = $derived(
    lectures.filter(
      (l) => l.id && l.status === 'published' && !placed.has(l.id)
    )
  );

  async function load() {
    loading = true;
    try {
      const [b, l] = await Promise.all([
        fetchBook(courseId),
        fetchCourseMaterials(courseId),
      ]);
      book = b.book;
      versions = b.versions;
      lectures = l.materials;
      dirty = false;
      error = '';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Could not load the course book';
    } finally {
      loading = false;
    }
  }
  onMount(() => void load());

  function edit(fn: (b: CourseBook) => void) {
    if (!book) return;
    const next = structuredClone($state.snapshot(book)) as CourseBook;
    fn(next);
    book = next;
    dirty = true;
  }

  const addUnit = () =>
    edit((b) =>
      b.structure.push({
        title: `Part ${b.structure.length + 1}`,
        material_ids: [],
      })
    );
  const removeUnit = (i: number) => edit((b) => b.structure.splice(i, 1));
  const moveUnit = (i: number, d: -1 | 1) =>
    edit((b) => {
      const j = i + d;
      if (j < 0 || j >= b.structure.length) return;
      [b.structure[i], b.structure[j]] = [b.structure[j], b.structure[i]];
    });
  const moveLecture = (u: number, i: number, d: -1 | 1) =>
    edit((b) => {
      const ids = b.structure[u].material_ids;
      const j = i + d;
      if (j < 0 || j >= ids.length) return;
      [ids[i], ids[j]] = [ids[j], ids[i]];
    });
  const removeLecture = (u: number, i: number) =>
    edit((b) => b.structure[u].material_ids.splice(i, 1));
  const addLecture = (id: string) =>
    edit((b) => {
      if (b.structure.length === 0)
        b.structure.push({ title: 'Part 1', material_ids: [] });
      b.structure[b.structure.length - 1].material_ids.push(id);
    });

  async function run(label: string, fn: () => Promise<string | void>) {
    busy = label;
    error = '';
    notice = '';
    try {
      notice = (await fn()) ?? '';
    } catch (e) {
      error = e instanceof Error ? e.message : 'Something went wrong';
    } finally {
      busy = '';
    }
  }

  const save = () =>
    run('save', async () => {
      if (!book) return;
      const res = await saveBook(courseId, {
        title: book.title,
        front_matter: book.front_matter,
        structure: book.structure as BookUnit[],
      });
      book = res.book;
      dirty = false;
      return 'Saved.';
    });

  const rebuild = () =>
    run('rebuild', async () => {
      if (dirty) await save();
      const r = await rebuildBook(courseId);
      await load();
      return `Built version ${r.version_no}: ${r.chapters_rebuilt} chapters rebuilt, ${r.chapters_reused} reused.`;
    });

  const publish = (v: Version) =>
    run('publish', async () => {
      await publishBook(courseId, v.id, {
        deliver: sendEmail ? ['email'] : [],
      });
      await load();
      return `Version ${v.version_no} published to students.`;
    });

  const exportAs = (format: ExportFormat, v?: Version) =>
    run(`export-${format}`, async () => {
      const blob = await downloadBook(courseId, format, v?.id);
      saveBlob(blob, exportFilename(book?.title ?? 'course-book', format));
    });
</script>

{#if loading}
  <div class="center"><Spinner /></div>
{:else if book}
  <div class="wrap">
    {#if book.stale}
      <p class="banner" role="status">
        Materials changed since the last build. Rebuild to include them.
      </p>
    {/if}

    <section class="card">
      <label class="field">
        <span>Book title</span>
        <input
          type="text"
          value={book.title}
          oninput={(e) => edit((b) => (b.title = e.currentTarget.value))}
        />
      </label>
      <label class="field">
        <span>Subtitle</span>
        <input
          type="text"
          value={book.front_matter.subtitle ?? ''}
          oninput={(e) =>
            edit(
              (b) =>
                (b.front_matter.subtitle = e.currentTarget.value || undefined)
            )}
        />
      </label>
      <label class="field">
        <span>Preface</span>
        <textarea
          rows="3"
          value={book.front_matter.preface ?? ''}
          oninput={(e) =>
            edit(
              (b) =>
                (b.front_matter.preface = e.currentTarget.value || undefined)
            )}></textarea>
      </label>
      <label class="field">
        <span>Syllabus</span>
        <textarea
          rows="3"
          value={book.front_matter.syllabus ?? ''}
          oninput={(e) =>
            edit(
              (b) =>
                (b.front_matter.syllabus = e.currentTarget.value || undefined)
            )}></textarea>
      </label>
    </section>

    <section class="card">
      <div class="head">
        <h3>Structure</h3>
        <Button size="sm" variant="outline" onclick={addUnit}
          ><Plus size={14} /> Add part</Button
        >
      </div>
      {#if book.structure.length === 0}
        <p class="muted">
          No parts yet. With no structure, all published lectures are included
          in order.
        </p>
      {/if}
      {#each book.structure as unit, u}
        <div class="unit">
          <div class="unit-head">
            <input
              type="text"
              aria-label="Part title"
              value={unit.title ?? ''}
              oninput={(e) =>
                edit((b) => (b.structure[u].title = e.currentTarget.value))}
            />
            <Button
              icon
              size="sm"
              aria-label="Move part up"
              disabled={u === 0}
              onclick={() => moveUnit(u, -1)}><ArrowUp size={14} /></Button
            >
            <Button
              icon
              size="sm"
              aria-label="Move part down"
              disabled={u === book.structure.length - 1}
              onclick={() => moveUnit(u, 1)}><ArrowDown size={14} /></Button
            >
            <Button
              icon
              size="sm"
              aria-label="Remove part"
              onclick={() => removeUnit(u)}><Trash2 size={14} /></Button
            >
          </div>
          <ol>
            {#each unit.material_ids as id, i (id)}
              <li>
                <span>{titleOf(id)}</span>
                <Button
                  icon
                  size="sm"
                  aria-label="Move up"
                  disabled={i === 0}
                  onclick={() => moveLecture(u, i, -1)}
                  ><ArrowUp size={13} /></Button
                >
                <Button
                  icon
                  size="sm"
                  aria-label="Move down"
                  disabled={i === unit.material_ids.length - 1}
                  onclick={() => moveLecture(u, i, 1)}
                  ><ArrowDown size={13} /></Button
                >
                <Button
                  icon
                  size="sm"
                  aria-label="Remove from part"
                  onclick={() => removeLecture(u, i)}
                  ><Trash2 size={13} /></Button
                >
              </li>
            {/each}
          </ol>
        </div>
      {/each}
      {#if unplaced.length}
        <div class="unplaced">
          <h4>Published lectures not in the book</h4>
          {#each unplaced as l}
            <div class="row">
              <span>{l.title}</span>
              <Button
                size="sm"
                variant="outline"
                onclick={() => l.id && addLecture(l.id)}
                >Add to last part</Button
              >
            </div>
          {/each}
        </div>
      {/if}
      <div class="row">
        <Button
          variant="outline"
          disabled={!dirty || !!busy}
          onclick={() => void save()}>Save structure</Button
        >
        <Button
          variant="accent"
          disabled={!!busy}
          onclick={() => void rebuild()}
        >
          {busy === 'rebuild' ? 'Building…' : 'Rebuild book'}
        </Button>
      </div>
    </section>

    <section class="card">
      <h3>Versions</h3>
      {#if versions.length === 0}
        <EmptyState
          icon={BookOpen}
          title="No book built yet"
          description="Rebuild to assemble the published lectures into a book."
        />
      {:else}
        <label class="check">
          <input type="checkbox" bind:checked={sendEmail} />
          Email students when publishing
        </label>
        <ul>
          {#each versions as v}
            <li>
              <span>v{v.version_no} · {v.chapters} chapters</span>
              {#if v.published_at}<span class="pill">published</span>{/if}
              <Button
                size="sm"
                variant="ghost"
                disabled={!!busy}
                onclick={() => void exportAs('pdf', v)}
                ><Download size={13} /> PDF</Button
              >
              <Button
                size="sm"
                variant="ghost"
                disabled={!!busy}
                onclick={() => void exportAs('docx', v)}>DOCX</Button
              >
              {#if book.published_version_id !== v.id}
                <Button
                  size="sm"
                  variant="outline"
                  disabled={!!busy}
                  onclick={() => void publish(v)}>Publish</Button
                >
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    {#if error}<p class="err" role="alert">{error}</p>{/if}
    {#if notice}<p class="ok" role="status">{notice}</p>{/if}
  </div>
{:else if error}
  <p class="err" role="alert">{error}</p>
{/if}

<style>
  .center {
    display: grid;
    place-items: center;
    padding: 40px;
  }
  .wrap {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    background: var(--surface-1);
  }
  h3 {
    margin: 0;
    font-size: 16px;
    color: var(--text-primary);
  }
  h4 {
    margin: 0 0 4px;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-tertiary);
  }
  .head,
  .row,
  .unit-head,
  li {
    display: flex;
    gap: 6px;
    align-items: center;
    flex-wrap: wrap;
  }
  .head {
    justify-content: space-between;
  }
  .unit {
    padding: 8px;
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
  }
  .unit-head input {
    flex: 1;
    min-width: 160px;
  }
  ol,
  ul {
    list-style: none;
    margin: 6px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 14px;
    color: var(--text-secondary);
  }
  li > span:first-child {
    flex: 1;
    min-width: 140px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .field span {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-tertiary);
  }
  input[type='text'],
  textarea {
    padding: 7px 9px;
    border: 1px solid var(--border-default);
    border-radius: 8px;
    background: var(--surface-inset);
    color: var(--text-primary);
    font: inherit;
    font-size: 14px;
  }
  .check {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 14px;
    color: var(--text-secondary);
  }
  .muted {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-tertiary);
  }
  .pill {
    padding: 0 8px;
    border: 1px solid #4c9a68;
    border-radius: 999px;
    font-size: 11.5px;
    color: #7fd3a0;
  }
  .banner,
  .err,
  .ok {
    margin: 0;
    padding: 8px 12px;
    border-radius: 8px;
    font-size: 13px;
    border: 1px solid #c9892b;
    color: #f0b25c;
  }
  .err {
    border-color: #ff9b7a;
    color: #ff9b7a;
  }
  .ok {
    border-color: #4c9a68;
    color: #7fd3a0;
  }
</style>
