/**
 * Typed client for the lecture materials engine (backend `materials` module).
 *
 * The block document types mirror `backend/src/materials/blocks.rs`; keep them
 * in step with it (`schema_version` is 1). Text fields hold inline Markdown
 * (`**bold**`, `*italic*`, `` `code` ``); equations are bare LaTeX.
 */
import { apiRequest, httpBlob, API_BASE, getAccessToken } from './http';

// ---- block document -------------------------------------------------------

export type MaterialOrigin = 'lecture' | 'supplemented';
export type CalloutVariant = 'note' | 'tip' | 'warning' | 'misconception';

export interface SourceRange {
  start_sec: number;
  end_sec: number;
}

export interface ReviewQuestion {
  question: string;
  answer?: string;
  difficulty?: string;
}

export interface ReadingItem {
  title: string;
  note?: string;
}

export type BlockKind =
  | { type: 'heading'; level: 1 | 2 | 3 | 4; text: string }
  | { type: 'paragraph'; text: string }
  | { type: 'definition'; term: string; text: string }
  | { type: 'key_result'; title?: string; text: string }
  | {
      type: 'worked_example';
      title: string;
      problem: string;
      steps: string[];
      answer?: string;
    }
  | { type: 'equation'; latex: string; caption?: string; number?: number }
  | {
      type: 'figure';
      caption: string;
      alt: string;
      visualization_id?: string;
      text_fallback?: string;
      /** Nodes and edges to draw (concept maps). */
      diagram?: Diagram;
    }
  | { type: 'table'; caption?: string; header: string[]; rows: string[][] }
  | { type: 'callout'; variant: CalloutVariant; text: string }
  | { type: 'review_questions'; questions: ReviewQuestion[] }
  | { type: 'further_reading'; items: ReadingItem[] }
  | { type: 'summary'; points: string[] };

export interface Diagram {
  nodes: { id: string; label: string; group?: string | null }[];
  edges: { from: string; to: string; label?: string | null }[];
}

export type Block = BlockKind & {
  id: string;
  origin: MaterialOrigin;
  source_ranges?: SourceRange[];
  locked: boolean;
  low_confidence: boolean;
};

export interface MaterialDoc {
  schema_version: number;
  title: string;
  subtitle?: string;
  meta: { objectives: string[]; language?: string; ai_drafted: boolean };
  blocks: Block[];
}

// ---- API shapes -----------------------------------------------------------

export type MaterialStatus = 'draft' | 'in_review' | 'published' | 'archived';
export type GenerationState =
  'idle' | 'queued' | 'generating' | 'ready' | 'failed';

export interface LectureMaterial {
  id: string;
  session_id: string;
  course_id: string;
  status: MaterialStatus;
  template: string;
  language: string;
  draft_doc: MaterialDoc | null;
  /** Send back as `base_rev` when saving so concurrent edits are detected. */
  draft_rev: number;
  current_version_id: string | null;
  published_version_id: string | null;
  generation: number;
  generation_state: GenerationState;
  generation_error: string | null;
  updated_at: string;
}

export interface MaterialVersionSummary {
  id: string;
  version_no: number;
  origin: 'ai' | 'lecturer';
  model: string | null;
  published_at: string | null;
  created_at: string;
}

export interface MaterialDelivery {
  id: string;
  channel: string;
  status: 'queued' | 'sending' | 'sent' | 'partial' | 'failed';
  recipients: number;
  delivered: number;
  error: string | null;
  created_at: string;
}

export interface SlidesInfo {
  has_slides: boolean;
  chars: number;
  name: string | null;
}

export interface MaterialResponse {
  material: LectureMaterial | null;
  slides?: SlidesInfo;
  session: { id: string; title: string };
  versions?: MaterialVersionSummary[];
  deliveries?: MaterialDelivery[];
}

export interface MaterialSettings {
  auto_generate: boolean;
  audience_level: string;
  depth: 'concise' | 'standard' | 'detailed';
  language: string;
  tone: string;
  template: string;
  seed_from_previous: boolean;
}

export interface MaterialCapabilities {
  formats: {
    id: string;
    label: string;
    extension: string;
    available: boolean;
  }[];
  templates: { id: string; name: string }[];
  delivery_channels: { id: string; label: string; deferred: boolean }[];
  analyzers: { id: string; label: string }[];
}

/** Format ids come from `/materials/capabilities`; these ship by default. */
export type ExportFormat = 'pdf' | 'docx' | 'md' | 'html' | 'epub' | 'tex';

/**
 * Fetch an export. Asks the server for a short-lived signed storage URL first
 * (the bytes then skip the API); falls back to streaming through the API when
 * signing is unavailable or the browser cannot reach storage directly.
 */
async function exportBlob(path: string): Promise<Blob> {
  try {
    const sep = path.includes('?') ? '&' : '?';
    const signed = await apiRequest<{ url: string | null }>(
      `${path}${sep}mode=url`
    );
    if (signed.url) {
      const res = await fetch(signed.url);
      if (res.ok) return await res.blob();
    }
  } catch {
    /* fall through to the streamed download */
  }
  return httpBlob(path);
}

const lecturePath = (code: string) =>
  `/sessions/code/${encodeURIComponent(code)}/materials`;

// ---- lecturer: one lecture ------------------------------------------------

export const fetchMaterialCapabilities = () =>
  apiRequest<MaterialCapabilities>('/materials/capabilities');

export const fetchMaterial = (sessionCode: string) =>
  apiRequest<MaterialResponse>(lecturePath(sessionCode));

/** Transcript lines (`offset_sec` may be null) for reviewing a draft. */
export const fetchMaterialTranscript = (sessionCode: string) =>
  apiRequest<{
    lines: { offset_sec: number | null; text: string }[];
  }>(`${lecturePath(sessionCode)}/transcript`);

/** Slides/notes as plain text; empty text clears them. */
export const saveMaterialSlidesText = (
  sessionCode: string,
  text: string,
  name?: string
) =>
  apiRequest<SlidesInfo>(`${lecturePath(sessionCode)}/slides`, {
    method: 'PUT',
    body: JSON.stringify({ text, name }),
  });

/** Upload a .pptx; its text becomes generation context. */
export async function uploadMaterialSlides(
  sessionCode: string,
  file: File
): Promise<SlidesInfo> {
  const token = getAccessToken();
  const res = await fetch(
    `${API_BASE}${lecturePath(sessionCode)}/slides/pptx?name=${encodeURIComponent(file.name)}`,
    {
      method: 'PUT',
      credentials: 'include',
      headers: token ? { authorization: `Bearer ${token}` } : {},
      body: file,
    }
  );
  const payload = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(payload.error ?? 'Could not read that file');
  return payload as SlidesInfo;
}

/** Creates the material if needed and (re)starts generation. */
export const generateMaterial = (sessionCode: string) =>
  apiRequest<{ material_id: string; generation: number; state: string }>(
    lecturePath(sessionCode),
    { method: 'POST' }
  );

export const saveMaterialDraft = (
  sessionCode: string,
  doc: MaterialDoc,
  baseRev: number
) =>
  apiRequest<{ draft_rev: number }>(`${lecturePath(sessionCode)}/draft`, {
    method: 'PUT',
    body: JSON.stringify({ doc, base_rev: baseRev }),
  });

export const updateMaterial = (
  sessionCode: string,
  patch: { status?: 'draft' | 'in_review'; template?: string }
) =>
  apiRequest<{ ok: boolean }>(lecturePath(sessionCode), {
    method: 'PATCH',
    body: JSON.stringify(patch),
  });

/** Rewrite one section (identified by its heading block id). */
export const regenerateMaterialSection = (
  sessionCode: string,
  headingBlockId: string,
  instruction?: string
) =>
  apiRequest<{ state: string }>(
    `${lecturePath(sessionCode)}/sections/${encodeURIComponent(headingBlockId)}/regenerate`,
    { method: 'POST', body: JSON.stringify({ instruction }) }
  );

export const publishMaterial = (
  sessionCode: string,
  options: { deliver?: string[]; message?: string } = {}
) =>
  apiRequest<{
    version_id: string;
    version_no: number;
    resource_id: string | null;
    deliveries: string[];
  }>(`${lecturePath(sessionCode)}/publish`, {
    method: 'POST',
    body: JSON.stringify(options),
  });

export const unpublishMaterial = (sessionCode: string, archive = false) =>
  apiRequest<{ ok: boolean }>(`${lecturePath(sessionCode)}/unpublish`, {
    method: 'POST',
    body: JSON.stringify({ archive }),
  });

/** `version` is a version id, or `'draft'` for the working draft. */
export const downloadMaterial = (
  sessionCode: string,
  format: ExportFormat,
  version?: string,
  template?: string
) => {
  const params = new URLSearchParams({ format });
  if (version) params.set('version', version);
  if (template) params.set('template', template);
  // Drafts are rendered on demand and never cached, so they cannot be signed.
  return version === 'draft'
    ? httpBlob(`${lecturePath(sessionCode)}/export?${params}`)
    : exportBlob(`${lecturePath(sessionCode)}/export?${params}`);
};

// ---- lecturer: course level -----------------------------------------------

export const fetchMaterialSettings = (courseId: string) =>
  apiRequest<MaterialSettings>(`/courses/${courseId}/materials/settings`);

export const saveMaterialSettings = (
  courseId: string,
  settings: MaterialSettings
) =>
  apiRequest<MaterialSettings>(`/courses/${courseId}/materials/settings`, {
    method: 'PUT',
    body: JSON.stringify(settings),
  });

export const fetchCourseMaterials = (courseId: string) =>
  apiRequest<{
    /** Every lecture of the course; `has_material` is false until drafted. */
    materials: {
      id: string | null;
      has_material: boolean;
      session_id: string;
      title: string;
      session_status: string;
      status: MaterialStatus | null;
      generation_state: GenerationState | null;
      draft_rev: number | null;
      started_at: string | null;
      /** Session short code: the key for the per-lecture lecturer routes. */
      short_code: string;
    }[];
  }>(`/courses/${courseId}/materials`);

// ---- course book ----------------------------------------------------------

export interface FrontMatter {
  subtitle?: string;
  preface?: string;
  syllabus?: string;
  objectives?: string[];
}

export interface BookUnit {
  title?: string | null;
  material_ids: string[];
}

export interface CourseBook {
  id: string;
  course_id: string;
  title: string;
  front_matter: FrontMatter;
  structure: BookUnit[];
  /** True when a material was published or changed since the last rebuild. */
  stale: boolean;
  published_version_id: string | null;
}

export const fetchBook = (courseId: string) =>
  apiRequest<{
    book: CourseBook;
    versions: {
      id: string;
      version_no: number;
      published_at: string | null;
      created_at: string;
      chapters: number;
    }[];
  }>(`/courses/${courseId}/book`);

export const saveBook = (
  courseId: string,
  patch: { title?: string; front_matter?: FrontMatter; structure?: BookUnit[] }
) =>
  apiRequest<{ book: CourseBook }>(`/courses/${courseId}/book`, {
    method: 'PUT',
    body: JSON.stringify(patch),
  });

export const rebuildBook = (courseId: string) =>
  apiRequest<{
    version_id: string;
    version_no: number;
    chapters_rebuilt: number;
    chapters_reused: number;
  }>(`/courses/${courseId}/book/rebuild`, { method: 'POST' });

export const publishBook = (
  courseId: string,
  versionId: string,
  options: { deliver?: string[]; message?: string } = {}
) =>
  apiRequest<{ published: string; deliveries: string[] }>(
    `/courses/${courseId}/book/publish`,
    {
      method: 'POST',
      body: JSON.stringify({ version_id: versionId, ...options }),
    }
  );

export const downloadBook = (
  courseId: string,
  format: ExportFormat,
  version?: string
) => {
  const params = new URLSearchParams({ format });
  if (version) params.set('version', version);
  return exportBlob(`/courses/${courseId}/book/export?${params}`);
};

// ---- comparison -----------------------------------------------------------

export type SectionChange =
  'unchanged' | 'modified' | 'moved' | 'added' | 'removed';

export interface ComparisonReport {
  id: string;
  method: 'embeddings' | 'lexical';
  summary: Record<SectionChange, number>;
  coverage: {
    topic: string;
    topic_b: string | null;
    in_a: boolean;
    in_b: boolean;
    change: SectionChange;
    similarity: number;
  }[];
  changes: {
    topic: string;
    change: SectionChange;
    diff: {
      unchanged: number;
      added: string[];
      removed: string[];
      changed: { block_type: string; before: string; after: string }[];
    };
  }[];
  drift: string[];
  new_topics: string[];
  narrative?: string | null;
}

/** Compare two offerings (same course code) owned by the lecturer. */
export const compareOfferings = (
  courseA: string,
  courseB: string,
  narrative = false
) =>
  apiRequest<ComparisonReport>(
    `/courses/${courseA}/compare?a=${courseA}&b=${courseB}&narrative=${narrative}`,
    { method: 'POST' }
  );

export const fetchComparison = (comparisonId: string) =>
  apiRequest<ComparisonReport>(`/comparisons/${comparisonId}`);

// ---- students -------------------------------------------------------------

export interface PublishedMaterial {
  session_id: string;
  lecture_title: string;
  version_id: string;
  version_no: number;
  published_at: string | null;
  doc: MaterialDoc;
  formats: { id: string; label: string }[];
}

export const fetchStudentMaterial = (sessionId: string) =>
  apiRequest<PublishedMaterial>(`/students/sessions/${sessionId}/material`);

export const downloadStudentMaterial = (
  sessionId: string,
  format: ExportFormat
) =>
  exportBlob(
    `/students/sessions/${sessionId}/material/export?format=${format}`
  );

export const fetchStudentCourseMaterials = (courseId: string) =>
  apiRequest<{
    materials: {
      session_id: string;
      version_id: string;
      lecture_title: string;
      version_no: number;
      published_at: string | null;
    }[];
    book_version_id: string | null;
    formats: { id: string; label: string }[];
  }>(`/students/courses/${courseId}/materials`);

export const fetchStudentBook = (courseId: string) =>
  apiRequest<{
    title: string;
    version_id: string;
    version_no: number;
    published_at: string | null;
    doc: MaterialDoc;
    formats: { id: string; label: string }[];
  }>(`/students/courses/${courseId}/book`);

export const downloadStudentBook = (courseId: string, format: ExportFormat) =>
  exportBlob(`/students/courses/${courseId}/book/export?format=${format}`);

// ---- plug-in settings -----------------------------------------------------

export type PluginKind =
  'generator' | 'renderer' | 'delivery' | 'analyzer' | 'template';

export interface PluginSetting {
  kind: PluginKind;
  id: string;
  label: string;
  enabled: boolean;
  config: Record<string, unknown>;
  /** Where the effective value comes from. */
  source: 'course' | 'global' | 'default';
  /** Protected plug-ins cannot be turned off. */
  protected: boolean;
}

export const fetchPluginSettings = (courseId: string) =>
  apiRequest<{ plugins: PluginSetting[] }>(
    `/courses/${courseId}/materials/plugins`
  );

export const setPluginSetting = (
  courseId: string,
  kind: PluginKind,
  pluginId: string,
  setting: { enabled: boolean; config?: Record<string, unknown> }
) =>
  apiRequest<{ enabled: boolean }>(
    `/courses/${courseId}/materials/plugins/${kind}/${encodeURIComponent(pluginId)}`,
    { method: 'PUT', body: JSON.stringify(setting) }
  );

/** Drop the course override; the plug-in follows the global setting again. */
export const resetPluginSetting = (
  courseId: string,
  kind: PluginKind,
  pluginId: string
) =>
  apiRequest<{ ok: boolean }>(
    `/courses/${courseId}/materials/plugins/${kind}/${encodeURIComponent(pluginId)}`,
    { method: 'DELETE' }
  );
