import type { SessionState } from './sessionState.svelte';
import type { RosterStudent } from './types';
import {
  uploadRoster,
  importRosterFile,
  getCourseRoster,
  createCourse,
  resolveCourseUuid,
  type ApiRosterStudent,
} from './api';

export function persist(state: SessionState) {
  if (state.session) {
    localStorage.setItem('klasync-session', JSON.stringify(state.session));
  }
  localStorage.setItem('klasync-roster', JSON.stringify(state.roster));
  if (state.rosterText) {
    localStorage.setItem('klasync-rosterText', state.rosterText);
  }
  localStorage.setItem('klasync-lecturer', state.lecturerName);
  if (state.courseId) {
    localStorage.setItem('klasync-courseId', state.courseId);
  }
  if (state.courseCode) {
    localStorage.setItem('klasync-courseCode', state.courseCode);
  }
  if (state.courseTitle) {
    localStorage.setItem('klasync-courseTitle', state.courseTitle);
  }
  if (state.currentUser) {
    localStorage.setItem('klasync-user', JSON.stringify(state.currentUser));
  }
}

export async function loadCourseRosterFromApi(
  state: SessionState
): Promise<boolean> {
  const courseCodeOrId = state.courseId || state.courseCode?.trim();
  if (!courseCodeOrId) {
    state.rosterNotice = 'Specify a Course Code to load roster from cloud.';
    return false;
  }
  try {
    const targetUuid = await resolveCourseUuid(
      courseCodeOrId,
      state.courseTitle,
      state.academicSession,
      state.semester
    );
    if (targetUuid) {
      state.courseId = targetUuid;
    }
    const remoteStudents = await getCourseRoster(targetUuid || courseCodeOrId);
    if (Array.isArray(remoteStudents)) {
      state.roster = remoteStudents.map((s) => ({
        matric: s.matric_number,
        name: s.full_name,
      }));
      state.rosterText = remoteStudents
        .map((s) => `${s.matric_number}, ${s.full_name}`)
        .join('\n');
      persist(state);
      state.rosterNotice =
        remoteStudents.length > 0
          ? `✓ Loaded ${remoteStudents.length} roster student${remoteStudents.length === 1 ? '' : 's'} from cloud.`
          : 'Cloud roster for this course is currently empty.';
      return true;
    }
    return false;
  } catch (err) {
    state.rosterNotice = `Cloud reload failed (${err instanceof Error ? err.message : 'API offline'}).`;
    return false;
  }
}

export async function saveToCloudRoster(state: SessionState): Promise<void> {
  const parsed = parseRosterTextToStudents(state.rosterText);
  if (parsed.length === 0) {
    state.rosterNotice = 'No student records to save. Add student rows first.';
    return;
  }

  const courseCode = state.courseCode?.trim();
  if (!courseCode) {
    state.rosterNotice = 'Course Code is required to save roster to cloud.';
    return;
  }

  state.roster = parsed;
  persist(state);

  const apiStudents: ApiRosterStudent[] = parsed.map((s) => ({
    matric_number: s.matric,
    full_name: s.name,
  }));

  try {
    const courseUuid = await resolveCourseUuid(
      state.courseId || courseCode,
      state.courseTitle?.trim() || courseCode,
      state.academicSession || '2025/2026',
      state.semester || 'Second Semester'
    );
    state.courseId = courseUuid;

    const res = await uploadRoster(courseUuid, apiStudents);
    persist(state);
    state.rosterNotice = `✓ Synced ${res.count} student${res.count === 1 ? '' : 's'} to cloud.`;
  } catch (err) {
    state.rosterNotice = `Cloud sync failed (${err instanceof Error ? err.message : 'API offline'}). Saved ${parsed.length} student${parsed.length === 1 ? '' : 's'} locally.`;
  }
}

export function parseRosterTextToStudents(rawText: string): RosterStudent[] {
  const cleanedText = rawText
    .replace(/^\uFEFF/, '')
    .replace(/[\u200B-\u200D\uFEFF]/g, '');
  const rawLines = cleanedText
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter(Boolean);
  if (rawLines.length === 0) return [];

  // Determine delimiter from first line (comma, semicolon, or tab)
  const firstLine = rawLines[0];
  let delimiter = ',';
  if (firstLine.includes('\t')) delimiter = '\t';
  else if (firstLine.includes(';')) delimiter = ';';

  // Check if line is a header
  const parseRow = (line: string) => {
    return line
      .split(delimiter)
      .map((col) => col.replace(/^["']|["']$/g, '').trim());
  };

  let startIdx = 0;
  let matricCol = 0;
  let nameCol = 1;

  const headerCols = parseRow(firstLine.toLowerCase());
  const hasHeaderMatric = headerCols.some(
    (c) => c.includes('matric') || c.includes('id') || c.includes('reg')
  );
  const hasHeaderName = headerCols.some(
    (c) => c.includes('name') || c.includes('student')
  );

  if (hasHeaderMatric || hasHeaderName) {
    startIdx = 1; // Skip header
    headerCols.forEach((col, idx) => {
      if (col.includes('matric') || col.includes('id') || col.includes('reg'))
        matricCol = idx;
      else if (col.includes('name') || col.includes('student')) nameCol = idx;
    });
  }

  const students: RosterStudent[] = [];
  for (let i = startIdx; i < rawLines.length; i++) {
    const cols = parseRow(rawLines[i]);
    const matric = cols[matricCol] ?? cols[0] ?? '';
    const name = cols[nameCol] ?? cols[1] ?? '';
    if (matric && name) {
      students.push({ matric, name });
    }
  }

  return students;
}

export async function parseRoster(state: SessionState) {
  const parsed = parseRosterTextToStudents(state.rosterText);
  state.roster = parsed;
  persist(state);

  const courseId = state.courseCode?.trim();
  if (courseId && parsed.length > 0) {
    try {
      const apiStudents: ApiRosterStudent[] = parsed.map((s) => ({
        matric_number: s.matric,
        full_name: s.name,
      }));

      // Ensure course exists on backend API before posting roster
      try {
        await createCourse({
          code: courseId,
          title: state.courseTitle?.trim() || courseId,
          academic_session: state.academicSession || '2025/2026',
          semester: state.semester || 'Second Semester',
        });
      } catch {
        // Course may already exist on backend
      }

      const targetCourseKey = state.activeCourse?.id || courseId;
      const res = await uploadRoster(targetCourseKey, apiStudents);
      state.rosterNotice = `✓ Synced ${res.count} student${res.count === 1 ? '' : 's'} to cloud.`;
    } catch {
      state.rosterNotice = `✓ Saved ${parsed.length} student${parsed.length === 1 ? '' : 's'} locally.`;
    }
  }
}

export function removeStudentFromRoster(state: SessionState, matric: string) {
  state.roster = state.roster.filter((s) => s.matric !== matric);
  persist(state);
}

export function clearRoster(state: SessionState) {
  state.roster = [];
  state.rosterText = '';
  state.rosterNotice = 'Roster cleared.';
  persist(state);
}

// Convert Excel column letters like 'A', 'B', 'AA' to 0-based column index
function colLetterToNumber(colStr: string): number {
  let num = 0;
  for (let i = 0; i < colStr.length; i++) {
    num = num * 26 + (colStr.charCodeAt(i) - 64);
  }
  return num - 1;
}

async function decompressDeflate(compressed: Uint8Array): Promise<Uint8Array> {
  try {
    const ds = new DecompressionStream('deflate-raw');
    const writer = ds.writable.getWriter();
    writer.write(compressed);
    writer.close();
    const res = new Response(ds.readable);
    const buf = await res.arrayBuffer();
    return new Uint8Array(buf);
  } catch {
    const ds = new DecompressionStream('deflate');
    const writer = ds.writable.getWriter();
    writer.write(compressed);
    writer.close();
    const res = new Response(ds.readable);
    const buf = await res.arrayBuffer();
    return new Uint8Array(buf);
  }
}

async function extractZipEntries(
  arrayBuffer: ArrayBuffer
): Promise<Map<string, Uint8Array>> {
  const bytes = new Uint8Array(arrayBuffer);
  const view = new DataView(arrayBuffer);
  const fileMap = new Map<string, Uint8Array>();
  const decoder = new TextDecoder('utf-8');

  let pos = 0;
  while (pos < bytes.length - 30) {
    const sig = view.getUint32(pos, true);
    if (sig === 0x04034b50) {
      const generalFlag = view.getUint16(pos + 6, true);
      const compressionMethod = view.getUint16(pos + 8, true);
      let compressedSize = view.getUint32(pos + 18, true);
      const fileNameLength = view.getUint16(pos + 26, true);
      const extraFieldLength = view.getUint16(pos + 28, true);
      const filename = decoder
        .decode(bytes.subarray(pos + 30, pos + 30 + fileNameLength))
        .toLowerCase();
      const dataOffset = pos + 30 + fileNameLength + extraFieldLength;

      if (compressedSize === 0 && generalFlag & 0x0008) {
        let nextPos = dataOffset;
        while (nextPos < bytes.length - 4) {
          const nextSig = view.getUint32(nextPos, true);
          if (nextSig === 0x04034b50 || nextSig === 0x02014b50) {
            break;
          }
          nextPos++;
        }
        compressedSize = nextPos - dataOffset;
        if (
          compressedSize >= 16 &&
          view.getUint32(nextPos - 16, true) === 0x08074b50
        ) {
          compressedSize -= 16;
        }
      }

      if (filename) {
        const compressedData = bytes.subarray(
          dataOffset,
          dataOffset + compressedSize
        );
        let decompressed: Uint8Array;
        if (compressionMethod === 0) {
          decompressed = compressedData;
        } else if (compressionMethod === 8) {
          decompressed = await decompressDeflate(compressedData);
        } else {
          pos = dataOffset + compressedSize;
          continue;
        }
        fileMap.set(filename, decompressed);
      }

      pos = dataOffset + compressedSize;
    } else if (sig === 0x02014b50 || sig === 0x06054b50) {
      break;
    } else {
      pos++;
    }
  }

  return fileMap;
}

export async function parseXlsxToCsv(
  arrayBuffer: ArrayBuffer
): Promise<string> {
  const fileMap = await extractZipEntries(arrayBuffer);
  const decoder = new TextDecoder('utf-8');

  // Parse Shared Strings if present
  const sharedStrings: string[] = [];
  const sharedStringsBytes = fileMap.get('xl/sharedstrings.xml');
  if (sharedStringsBytes) {
    const xmlText = decoder.decode(sharedStringsBytes);
    const parser = new DOMParser();
    const doc = parser.parseFromString(xmlText, 'application/xml');
    const siElements = doc.querySelectorAll('si');
    siElements.forEach((si) => {
      const tElements = si.querySelectorAll('t');
      if (tElements.length > 0) {
        let str = '';
        tElements.forEach((t) => (str += t.textContent || ''));
        sharedStrings.push(str);
      } else {
        sharedStrings.push(si.textContent || '');
      }
    });
  }

  // Find primary worksheet XML
  let sheetBytes: Uint8Array | undefined;
  for (const [key, val] of fileMap.entries()) {
    if (key.startsWith('xl/worksheets/sheet')) {
      sheetBytes = val;
      break;
    }
  }

  if (!sheetBytes) {
    throw new Error('No worksheet found in XLSX file.');
  }

  const sheetXml = decoder.decode(sheetBytes);
  const parser = new DOMParser();
  const doc = parser.parseFromString(sheetXml, 'application/xml');
  const rowElements = doc.querySelectorAll('row');

  const rows: string[][] = [];

  rowElements.forEach((rowEl) => {
    const cellElements = rowEl.querySelectorAll('c');
    const rowCells: string[] = [];

    cellElements.forEach((cEl) => {
      const cellRef = cEl.getAttribute('r') || '';
      const colLetters = cellRef.replace(/[0-9]/g, '').toUpperCase();
      const colIdx = colLetters
        ? colLetterToNumber(colLetters)
        : rowCells.length;

      const type = cEl.getAttribute('t');
      const vEl = cEl.querySelector('v');
      let val = '';

      if (type === 's' && vEl) {
        const idx = parseInt(vEl.textContent || '0', 10);
        val = sharedStrings[idx] ?? '';
      } else if (type === 'inlineStr') {
        const isEl = cEl.querySelector('is t') || cEl.querySelector('is');
        val = isEl?.textContent || '';
      } else if (vEl) {
        val = vEl.textContent || '';
      }

      while (rowCells.length < colIdx) {
        rowCells.push('');
      }
      rowCells[colIdx] = val.replace(/,/g, ' ').trim();
    });

    if (rowCells.some((c) => c.trim().length > 0)) {
      rows.push(rowCells);
    }
  });

  return rows.map((r) => r.join(', ')).join('\n');
}

export async function importFile(
  state: SessionState,
  eventOrFile: Event | File
) {
  let file: File | undefined;

  if (eventOrFile instanceof File) {
    file = eventOrFile;
  } else if (
    eventOrFile &&
    (eventOrFile.target as HTMLInputElement)?.files?.[0]
  ) {
    file = (eventOrFile.target as HTMLInputElement).files![0];
  }

  if (!file) return;

  const isCsvOrText = /\.csv$|\.txt$|\.tsv$/i.test(file.name);
  const isXlsx = /\.xlsx$/i.test(file.name);

  if (!isCsvOrText && !isXlsx) {
    state.rosterNotice =
      'Unsupported file format. Please upload a .csv, .tsv, or .xlsx file.';
    return;
  }

  const courseId = state.courseCode?.trim();

  // Try backend file import if course code is specified
  if (courseId) {
    try {
      try {
        await createCourse({
          code: courseId,
          title: state.courseTitle?.trim() || courseId,
          academic_session: state.academicSession || '2025/2026',
          semester: state.semester || 'Second Semester',
        });
      } catch {}

      const targetCourseKey = state.activeCourse?.id || courseId;
      const report = await importRosterFile(targetCourseKey, file);
      if (report && typeof report.imported_count === 'number') {
        state.rosterNotice = `✓ Imported ${report.imported_count} student${report.imported_count === 1 ? '' : 's'} via API.`;
      }
    } catch {
      // Fallback to client-side parsing if backend endpoint is unavailable
    }
  }

  const reader = new FileReader();

  if (isXlsx) {
    reader.onload = async () => {
      try {
        const arrayBuffer = reader.result as ArrayBuffer;
        const csvText = await parseXlsxToCsv(arrayBuffer);
        state.rosterText = csvText;
        await parseRoster(state);
        if (!state.rosterNotice.startsWith('✓')) {
          state.rosterNotice = `✓ Imported ${state.roster.length} student${state.roster.length === 1 ? '' : 's'} from ${file.name}.`;
        }
      } catch (err) {
        state.rosterNotice =
          'Error reading Excel file. Please ensure it is a valid .xlsx spreadsheet.';
      }
    };
    reader.onerror = () => {
      state.rosterNotice = 'Error loading file from disk.';
    };
    reader.readAsArrayBuffer(file);
  } else {
    reader.onload = async () => {
      try {
        const content = String(reader.result ?? '');
        state.rosterText = content.replace(/^\uFEFF/, '');
        await parseRoster(state);
        if (!state.rosterNotice.startsWith('✓')) {
          state.rosterNotice = `✓ Imported ${state.roster.length} student${state.roster.length === 1 ? '' : 's'} from ${file.name}.`;
        }
      } catch {
        state.rosterNotice =
          'Error reading file contents. Ensure the file contains readable student records.';
      }
    };
    reader.onerror = () => {
      state.rosterNotice = 'Error loading file from disk.';
    };
    reader.readAsText(file);
  }
}
